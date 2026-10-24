//! 四种复制格式的文本发射器。
//!
//! 全部从 `LyricsDoc` DTO 生成，因此不依赖上游原始格式，
//! QQ（QRC）与酷狗（KRC）走同一套代码，输出可预期。
//!
//! 逐字格式的规范参考 LDDC 的输出习惯（用户提供的范例文件）：
//!
//! ```text
//! 增强逐字：[00:00.000]<00:00.000>晴<00:00.160>天<00:00.320>
//! 普通逐字：[00:00.000]晴[00:00.160]天[00:00.320]
//! ```
//!
//! 两种逐字格式都以**末音节的结束时间**作为行尾标签收尾（上例的
//! `<00:00.320>` / `[00:00.320]`），这样下游播放器能知道最后一个字唱到何时。
//! 译文行只保留首尾两个时间标签，不做逐字：`[起点]译文[终点]`。

use crate::model::{LyricLine, LyricsDoc};

/// 复制选项，与前端 `CopyOptions` 一一对应。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyOptions {
    /// 是否输出 `[ti:]/[ar:]/[al:]` 标签行。
    #[serde(default)]
    pub include_metadata: bool,
    /// 是否在每行后追加译文行（`[起点]译文[终点]`）。
    #[serde(default)]
    pub include_translation: bool,
    /// 是否过滤「词：/曲：/编曲：/制作人：」等制作信息行。
    #[serde(default)]
    pub filter_credits: bool,
    /// 是否保留逐字中的空白音节。
    #[serde(default = "default_true")]
    pub keep_spaces: bool,
}

fn default_true() -> bool {
    true
}

impl Default for CopyOptions {
    fn default() -> Self {
        Self {
            include_metadata: false,
            include_translation: false,
            filter_credits: false,
            keep_spaces: true,
        }
    }
}

/// 把毫秒格式化成 `mm:ss.mmm`。
pub fn format_timestamp(ms: i32) -> String {
    let ms = ms.max(0);
    let minute = ms / 60_000;
    let second = (ms % 60_000) / 1000;
    let milli = ms % 1000;
    format!("{minute:02}:{second:02}.{milli:03}")
}

/// 制作信息行的判定：`词：`、`曲：`、`编曲：`、`制作人：` 等。
fn is_credit_line(text: &str) -> bool {
    const CREDIT_PREFIXES: [&str; 12] = [
        "词：", "曲：", "编曲：", "制作人：", "作词：", "作曲：", "混音：", "录音：", "母带：",
        "和声：", "监制：", "出品：",
    ];
    let trimmed = text.trim_start();
    CREDIT_PREFIXES
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

fn should_skip(text: &str, options: &CopyOptions) -> bool {
    options.filter_credits && is_credit_line(text)
}

/// 有效音节（按 `keep_spaces` 过滤空白音节）。
fn visible_syllables<'a>(line: &'a LyricLine, options: &CopyOptions) -> Option<Vec<&'a crate::model::Syllable>> {
    let syllables = line.syllables.as_ref().filter(|s| !s.is_empty())?;
    let kept: Vec<&crate::model::Syllable> = syllables
        .iter()
        .filter(|s| options.keep_spaces || !s.text.trim().is_empty())
        .collect();
    if kept.is_empty() {
        None
    } else {
        Some(kept)
    }
}

/// 行尾结束时间：末音节的结束时间；没有逐字信息时退回 `start + dur`。
///
/// 逐字格式的行尾标签用它，而不是 `行起点 + 行时长`：QRC 的行时长
/// 常常一直算到下一行开始（含休止符），拿它当最后一个字的结束时间会让
/// 下游播放器把尾字拖长（实测《晴天》首行行时长 2250ms，而末字只唱到 2240ms）。
fn line_end_ms(line: &LyricLine) -> i32 {
    if let Some(syllables) = line.syllables.as_ref().filter(|s| !s.is_empty()) {
        if let Some(last) = syllables.last() {
            let end = last.start_ms + last.dur_ms.max(0);
            if end > line.start_ms {
                return end;
            }
        }
    }
    match line.dur_ms {
        Some(duration) if duration > 0 => line.start_ms + duration,
        _ => line.start_ms,
    }
}

fn metadata_header(doc: &LyricsDoc) -> String {
    let mut out = String::new();
    out.push_str(&format!("[ti:{}]\n", doc.song.title));
    out.push_str(&format!("[ar:{}]\n", doc.song.artist_text()));
    if !doc.song.album.is_empty() {
        out.push_str(&format!("[al:{}]\n", doc.song.album));
    }
    out.push_str("[offset:0]\n\n");
    out
}

/// 译文行的发射：`[起点]译文[终点]`。
///
/// 只取整行的首尾时间，不复制逐字结构 —— 译文是整句对应，不是逐字对应。
fn push_translation(out: &mut String, line: &LyricLine) {
    let Some(translation) = line.translation.as_ref().filter(|t| !t.trim().is_empty()) else {
        return;
    };
    out.push('[');
    out.push_str(&format_timestamp(line.start_ms));
    out.push(']');
    out.push_str(translation.trim());
    out.push('[');
    out.push_str(&format_timestamp(line_end_ms(line)));
    out.push(']');
    out.push('\n');
}

// ---------------------------------------------------------------------------
// 1) 逐字（增强 LRC）
// ---------------------------------------------------------------------------

/// 生成增强 LRC：`[行时间]<字时间>字…<行结束时间>`。
///
/// 无逐字信息的行退化为 `[行时间]整行文本`，因此同一份输出里逐字行与逐行行可以共存。
pub fn enhanced_lrc(doc: &LyricsDoc, options: &CopyOptions) -> String {
    let mut out = String::new();
    if options.include_metadata {
        out.push_str(&metadata_header(doc));
    }

    for line in &doc.lines {
        if line.text.trim().is_empty() && line.syllables.is_none() {
            continue;
        }
        if should_skip(&line.text, options) {
            continue;
        }

        out.push('[');
        out.push_str(&format_timestamp(line.start_ms));
        out.push(']');

        match visible_syllables(line, options) {
            Some(syllables) => {
                for syllable in &syllables {
                    out.push('<');
                    out.push_str(&format_timestamp(syllable.start_ms));
                    out.push('>');
                    out.push_str(&syllable.text);
                }
                // 行尾标签：末音节的结束时间。之前先裁掉末字的尾随空白，
                // 否则会留下 `…La <00:45.710>` 这种悬空空格（LDDC 范例里是 `La<00:45.710>`）。
                let trimmed = out.trim_end().len();
                out.truncate(trimmed);
                out.push('<');
                out.push_str(&format_timestamp(line_end_ms(line)));
                out.push('>');
            }
            None => out.push_str(&line.text),
        }

        out.push('\n');

        if options.include_translation {
            push_translation(&mut out, line);
        }
    }

    out
}

// ---------------------------------------------------------------------------
// 2) 逐字（普通 LRC 记号）
// ---------------------------------------------------------------------------

/// 生成普通逐字：`[行时间]字[字时间]字…[行结束时间]`。
///
/// 与增强 LRC 的唯一差别是不用尖括号。时间标签放在**它前面的字之前**，
/// 因此首字的时间标签与行标签重合、末字后面跟行结束时间。
pub fn plain_syllable(doc: &LyricsDoc, options: &CopyOptions) -> String {
    let mut out = String::new();
    if options.include_metadata {
        out.push_str(&metadata_header(doc));
    }

    for line in &doc.lines {
        if line.text.trim().is_empty() && line.syllables.is_none() {
            continue;
        }
        if should_skip(&line.text, options) {
            continue;
        }

        let Some(syllables) = visible_syllables(line, options) else {
            // 没有逐字信息时仍输出行文本，避免用户拿到空结果。
            out.push('[');
            out.push_str(&format_timestamp(line.start_ms));
            out.push(']');
            out.push_str(&line.text);
            out.push('\n');
            continue;
        };

        let end = line_end_ms(line);

        out.push('[');
        out.push_str(&format_timestamp(line.start_ms));
        out.push(']');

        for (index, syllable) in syllables.iter().enumerate() {
            out.push_str(&syllable.text);
            let next = syllables.get(index + 1).map_or(end, |next| next.start_ms);
            if index + 1 == syllables.len() {
                // 末字之后不再有其他字，先裁掉尾随空白再收口（同增强格式）。
                let trimmed = out.trim_end().len();
                out.truncate(trimmed);
            }
            out.push('[');
            out.push_str(&format_timestamp(next));
            out.push(']');
        }

        out.push('\n');

        if options.include_translation {
            push_translation(&mut out, line);
        }
    }

    out
}

// ---------------------------------------------------------------------------
// 3) 逐行（带时间戳的标准 LRC）
// ---------------------------------------------------------------------------

/// 生成标准 LRC：`[mm:ss.mmm]整行文本`。
pub fn plain_lrc(doc: &LyricsDoc, options: &CopyOptions) -> String {
    let mut out = String::new();
    if options.include_metadata {
        out.push_str(&metadata_header(doc));
    }

    for line in &doc.lines {
        if line.text.trim().is_empty() {
            continue;
        }
        if should_skip(&line.text, options) {
            continue;
        }

        out.push('[');
        out.push_str(&format_timestamp(line.start_ms));
        out.push(']');
        out.push_str(line.text.trim_end());
        out.push('\n');

        if options.include_translation {
            push_translation(&mut out, line);
        }
    }

    out
}

// ---------------------------------------------------------------------------
// 4) 纯文本
// ---------------------------------------------------------------------------

/// 生成纯文本，只保留歌词正文；勾选译文时译文另起一行（用括号区分）。
pub fn plain_text(doc: &LyricsDoc, options: &CopyOptions) -> String {
    let mut out = String::new();

    for line in &doc.lines {
        if line.text.trim().is_empty() {
            continue;
        }
        if should_skip(&line.text, options) {
            continue;
        }
        out.push_str(line.text.trim_end());
        out.push('\n');

        if options.include_translation {
            if let Some(translation) = line.translation.as_ref().filter(|t| !t.trim().is_empty()) {
                out.push_str("（");
                out.push_str(translation.trim());
                out.push_str("）\n");
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// 分派
// ---------------------------------------------------------------------------

/// 按格式名生成文本。未知格式返回 `None`。
pub fn generate(doc: &LyricsDoc, format: &str, options: &CopyOptions) -> Option<String> {
    let text = match format {
        "enhanced" => enhanced_lrc(doc, options),
        "plain" => plain_syllable(doc, options),
        "lrc" => plain_lrc(doc, options),
        "text" => plain_text(doc, options),
        _ => return None,
    };
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LyricLine, SongCandidate, Source, Syllable};

    fn doc() -> LyricsDoc {
        LyricsDoc {
            song: SongCandidate {
                source: Source::Qq,
                song_id: "0039MnYb0qxYhV".into(),
                numeric_id: Some(97773),
                title: "晴天".into(),
                artists: vec!["周杰伦".into()],
                album: "叶惠美".into(),
                duration_ms: Some(269_000),
            },
            raw_fmt: "qrc".into(),
            lines: vec![
                LyricLine {
                    start_ms: 0,
                    dur_ms: Some(2_250),
                    text: "晴天".into(),
                    syllables: Some(vec![
                        Syllable {
                            text: "晴".into(),
                            start_ms: 0,
                            dur_ms: 160,
                        },
                        Syllable {
                            text: "天".into(),
                            start_ms: 160,
                            dur_ms: 160,
                        },
                    ]),
                    translation: None,
                },
                LyricLine {
                    start_ms: 2_250,
                    dur_ms: Some(2_250),
                    text: "词：周杰伦".into(),
                    syllables: Some(vec![Syllable {
                        text: "词：周杰伦".into(),
                        start_ms: 2_250,
                        dur_ms: 450,
                    }]),
                    translation: Some("Lyrics: Jay Chou".into()),
                },
            ],
            word_synced: true,
            from_cache: false,
            notice: None,
        }
    }

    #[test]
    fn timestamp_is_millisecond_precise() {
        assert_eq!(format_timestamp(0), "00:00.000");
        assert_eq!(format_timestamp(2_250), "00:02.250");
        assert_eq!(format_timestamp(269_000), "04:29.000");
        assert_eq!(format_timestamp(-5), "00:00.000");
    }

    #[test]
    fn enhanced_lrc_has_line_and_syllable_timestamps() {
        let options = CopyOptions::default();
        assert_eq!(
            enhanced_lrc(&doc(), &options),
            "[00:00.000]<00:00.000>晴<00:00.160>天<00:00.320>\n\
             [00:02.250]<00:02.250>词：周杰伦<00:02.700>\n"
        );
    }

    #[test]
    fn plain_syllable_uses_bracket_timestamps() {
        let options = CopyOptions::default();
        assert_eq!(
            plain_syllable(&doc(), &options),
            "[00:00.000]晴[00:00.160]天[00:00.320]\n\
             [00:02.250]词：周杰伦[00:02.700]\n"
        );
    }

    #[test]
    fn trailing_tag_is_last_syllable_end_not_line_duration() {
        // 行时长 2250 但末字只唱到 320 —— 行尾标签必须是 320。
        let options = CopyOptions::default();
        let out = enhanced_lrc(&doc(), &options);
        let first = out.lines().next().unwrap();
        assert!(first.ends_with("<00:00.320>"), "实际：{first}");
        assert!(!first.contains("<00:02.250>"));
    }

    #[test]
    fn translation_line_keeps_only_head_and_tail_tags() {
        let options = CopyOptions {
            include_translation: true,
            ..CopyOptions::default()
        };
        let out = enhanced_lrc(&doc(), &options);
        let lines: Vec<&str> = out.lines().collect();
        // lines[0] 是首行原文（无译文），lines[1] 第二行原文，lines[2] 它的译文。
        assert_eq!(lines[2], "[00:02.250]Lyrics: Jay Chou[00:02.700]");
        // 译文行不带逐字标记。
        assert!(!lines[2].contains('<'));
    }

    #[test]
    fn plain_lrc_has_only_line_timestamps() {
        let options = CopyOptions::default();
        assert_eq!(
            plain_lrc(&doc(), &options),
            "[00:00.000]晴天\n[00:02.250]词：周杰伦\n"
        );
    }

    #[test]
    fn plain_text_strips_everything() {
        let options = CopyOptions::default();
        assert_eq!(plain_text(&doc(), &options), "晴天\n词：周杰伦\n");
    }

    #[test]
    fn metadata_and_translation_are_opt_in() {
        let options = CopyOptions {
            include_metadata: true,
            include_translation: true,
            ..CopyOptions::default()
        };
        let out = plain_lrc(&doc(), &options);
        assert!(out.starts_with("[ti:晴天]\n[ar:周杰伦]\n[al:叶惠美]\n[offset:0]\n\n"));
        assert!(out.contains("[00:02.250]Lyrics: Jay Chou[00:02.700]"));
    }

    #[test]
    fn credits_can_be_filtered() {
        let options = CopyOptions {
            filter_credits: true,
            ..CopyOptions::default()
        };
        assert_eq!(plain_text(&doc(), &options), "晴天\n");
    }

    #[test]
    fn spaces_can_be_dropped() {
        let mut doc = doc();
        doc.lines[0].syllables = Some(vec![
            Syllable {
                text: "晴".into(),
                start_ms: 0,
                dur_ms: 160,
            },
            Syllable {
                text: " ".into(),
                start_ms: 160,
                dur_ms: 160,
            },
            Syllable {
                text: "天".into(),
                start_ms: 320,
                dur_ms: 160,
            },
        ]);
        let keep = CopyOptions::default();
        assert!(enhanced_lrc(&doc, &keep).contains("<00:00.160> "));

        let drop = CopyOptions {
            keep_spaces: false,
            ..CopyOptions::default()
        };
        assert!(!enhanced_lrc(&doc, &drop).contains("<00:00.160> "));
    }

    #[test]
    fn unknown_format_returns_none() {
        assert!(generate(&doc(), "bogus", &CopyOptions::default()).is_none());
        assert!(generate(&doc(), "enhanced", &CopyOptions::default()).is_some());
        assert!(generate(&doc(), "plain", &CopyOptions::default()).is_some());
    }
}
