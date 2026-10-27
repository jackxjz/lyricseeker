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
//! 译文行只保留首尾两个时间标签，不做逐字：`[起点]译文[终点]`；
//! 具体摆在哪（同一行 / 原文下面 / 原文上面 / 交错）由 `TranslationLayout` 决定。

use crate::model::{LyricLine, LyricsDoc};

/// 译文在复制结果里的摆放方式。
///
/// 四种形态对应设置页「翻译」那一段的四个单选；括号里是实际样子：
///
/// - `SameLine`   `[t]原词 译文`
/// - `NextLine`   `[t]原词` 换行 `[t]译文`（默认，播放器最通用）
/// - `Before`     `[t]译文` 换行 `[t]原词`
/// - `Interleaved` `[t]原词` 换行 `[下一行 t]译文`
///
/// `Interleaved` 用下一行的时间戳是刻意的：部分播放器按时间配对渲染，
/// 这样译文会跟着下一行走而不是和原文一起出现。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TranslationLayout {
    /// 1️⃣ 与原文同一行。
    SameLine,
    /// 2️⃣ 译文另起一行，紧跟在原文之后（默认）。
    #[default]
    NextLine,
    /// 3️⃣ 译文在原文之前。
    Before,
    /// 4️⃣ 译文跟在原文之后，但用下一行的时间戳。
    Interleaved,
}

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
    /// 译文的摆放方式。老版本设置文件没有这个字段，一律落到默认的 `NextLine`。
    #[serde(default)]
    pub translation_layout: TranslationLayout,
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
            translation_layout: TranslationLayout::default(),
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

/// 这一行的译文文本（去空白；空串与纯空白视为没有译文）。
fn translation_text(line: &LyricLine) -> Option<&str> {
    line.translation
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// 写一条带首尾时间的译文：`[起点]译文[终点]`。
fn push_timed(out: &mut String, text: &str, start_ms: i32, end_ms: i32) {
    out.push('[');
    out.push_str(&format_timestamp(start_ms));
    out.push(']');
    out.push_str(text);
    out.push('[');
    out.push_str(&format_timestamp(end_ms));
    out.push(']');
    out.push('\n');
}

/// 下一条「会被输出」的行的 (起点, 结束时间)。
///
/// `Interleaved` 要用它 —— 译文挂在下一行的时间上。为了不被空行和被过滤掉的
/// 制作信息行带偏，这里按输出条件往后找，而不是直接取 `index + 1`。
fn next_emitted(doc: &LyricsDoc, index: usize, options: &CopyOptions) -> Option<(i32, i32)> {
    doc.lines
        .iter()
        .skip(index + 1)
        .find(|line| !is_blank_line(line) && !should_skip(&line.text, options))
        .map(|line| (line.start_ms, line_end_ms(line)))
}

/// 收尾一行：按 `layout` 处理译文，并补上这一行的换行。
///
/// `mark` 是写入原文之前记录的 `out.len()` —— `Before` 需要用 `split_off`
/// 把刚写好的原文挪到译文后面，否则就得先把译文缓存起来再拼。
fn finish_line(
    out: &mut String,
    mark: usize,
    line: &LyricLine,
    next: Option<(i32, i32)>,
    options: &CopyOptions,
    with_timestamps: bool,
    paren: bool,
) {
    let Some(raw) = translation_text(line).filter(|_| options.include_translation) else {
        out.push('\n');
        return;
    };
    // 纯文本没有时间戳，原文与译文难免看起来一样；只有这种格式保留括号，
    // 让人一眼能认出哪一句是译文。
    let bracketed;
    let text = if paren {
        bracketed = format!("（{raw}）");
        bracketed.as_str()
    } else {
        raw
    };

    match options.translation_layout {
        TranslationLayout::SameLine => {
            // 原文与译文之间固定一个空格；原文本身以空格结尾时不重复加。
            if !out.ends_with(' ') {
                out.push(' ');
            }
            out.push_str(text);
            out.push('\n');
        }
        TranslationLayout::NextLine => {
            out.push('\n');
            if with_timestamps {
                push_timed(out, text, line.start_ms, line_end_ms(line));
            } else {
                out.push_str(text);
                out.push('\n');
            }
        }
        TranslationLayout::Before => {
            let original = out.split_off(mark);
            if with_timestamps {
                push_timed(out, text, line.start_ms, line_end_ms(line));
            } else {
                out.push_str(text);
                out.push('\n');
            }
            out.push_str(&original);
            out.push('\n');
        }
        TranslationLayout::Interleaved => {
            out.push('\n');
            // 最后一行没有「下一行」，退回用自己这行的时间，至少不丢译文。
            let (start_ms, end_ms) = next.unwrap_or((line.start_ms, line_end_ms(line)));
            if with_timestamps {
                push_timed(out, text, start_ms, end_ms);
            } else {
                out.push_str(text);
                out.push('\n');
            }
        }
    }
}

/// 完全空的行（既没文本也没逐字）。
fn is_blank_line(line: &LyricLine) -> bool {
    line.text.trim().is_empty() && line.syllables.is_none()
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

    for (index, line) in doc.lines.iter().enumerate() {
        if is_blank_line(line) {
            continue;
        }
        if should_skip(&line.text, options) {
            continue;
        }

        let mark = out.len();
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

        finish_line(
            &mut out,
            mark,
            line,
            next_emitted(doc, index, options),
            options,
            true,
            false,
        );
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

    for (index, line) in doc.lines.iter().enumerate() {
        if is_blank_line(line) {
            continue;
        }
        if should_skip(&line.text, options) {
            continue;
        }

        let mark = out.len();
        let next = next_emitted(doc, index, options);

        let Some(syllables) = visible_syllables(line, options) else {
            // 没有逐字信息时仍输出行文本，避免用户拿到空结果。
            out.push('[');
            out.push_str(&format_timestamp(line.start_ms));
            out.push(']');
            out.push_str(&line.text);
            finish_line(&mut out, mark, line, next, options, true, false);
            continue;
        };

        let end = line_end_ms(line);

        out.push('[');
        out.push_str(&format_timestamp(line.start_ms));
        out.push(']');

        for (index, syllable) in syllables.iter().enumerate() {
            out.push_str(&syllable.text);
            let next_start = syllables.get(index + 1).map_or(end, |next| next.start_ms);
            if index + 1 == syllables.len() {
                // 末字之后不再有其他字，先裁掉尾随空白再收口（同增强格式）。
                let trimmed = out.trim_end().len();
                out.truncate(trimmed);
            }
            out.push('[');
            out.push_str(&format_timestamp(next_start));
            out.push(']');
        }

        finish_line(&mut out, mark, line, next, options, true, false);
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

    for (index, line) in doc.lines.iter().enumerate() {
        if line.text.trim().is_empty() {
            continue;
        }
        if should_skip(&line.text, options) {
            continue;
        }

        let mark = out.len();
        out.push('[');
        out.push_str(&format_timestamp(line.start_ms));
        out.push(']');
        out.push_str(line.text.trim_end());

        finish_line(
            &mut out,
            mark,
            line,
            next_emitted(doc, index, options),
            options,
            true,
            false,
        );
    }

    out
}

// ---------------------------------------------------------------------------
// 4) 纯文本
// ---------------------------------------------------------------------------

/// 生成纯文本，只保留歌词正文；勾选译文时译文另起一行（用括号区分）。
pub fn plain_text(doc: &LyricsDoc, options: &CopyOptions) -> String {
    let mut out = String::new();

    for (index, line) in doc.lines.iter().enumerate() {
        if line.text.trim().is_empty() {
            continue;
        }
        if should_skip(&line.text, options) {
            continue;
        }
        let mark = out.len();
        out.push_str(line.text.trim_end());

        finish_line(
            &mut out,
            mark,
            line,
            next_emitted(doc, index, options),
            options,
            false,
            true,
        );
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

    /// 每行都有译文，用于验证四种摆放方式。
    fn doc_translated() -> LyricsDoc {
        let mut doc = doc();
        doc.lines[0].translation = Some("Sunny day".into());
        doc
    }

    fn with_layout(layout: TranslationLayout) -> CopyOptions {
        CopyOptions {
            include_translation: true,
            translation_layout: layout,
            ..CopyOptions::default()
        }
    }

    #[test]
    fn layout_same_line_puts_translation_after_original() {
        let out = plain_lrc(&doc_translated(), &with_layout(TranslationLayout::SameLine));
        assert_eq!(
            out,
            "[00:00.000]晴天 Sunny day\n[00:02.250]词：周杰伦 Lyrics: Jay Chou\n"
        );
    }

    #[test]
    fn layout_next_line_is_the_default() {
        // 默认值必须是 2️⃣：设置文件里没有该字段时也走这一支。
        assert_eq!(CopyOptions::default().translation_layout, TranslationLayout::NextLine);
        let out = plain_lrc(&doc_translated(), &with_layout(TranslationLayout::NextLine));
        assert_eq!(
            out,
            "[00:00.000]晴天\n[00:00.000]Sunny day[00:00.320]\n\
             [00:02.250]词：周杰伦\n[00:02.250]Lyrics: Jay Chou[00:02.700]\n"
        );
    }

    #[test]
    fn layout_before_puts_translation_first() {
        let out = plain_lrc(&doc_translated(), &with_layout(TranslationLayout::Before));
        assert_eq!(
            out,
            "[00:00.000]Sunny day[00:00.320]\n[00:00.000]晴天\n\
             [00:02.250]Lyrics: Jay Chou[00:02.700]\n[00:02.250]词：周杰伦\n"
        );
    }

    #[test]
    fn layout_interleaved_borrows_next_line_time() {
        let out = plain_lrc(&doc_translated(), &with_layout(TranslationLayout::Interleaved));
        // 首行译文用下一行的 2250 而不是自己的 0；末行没有下一行，退回自己这行的时间。
        assert_eq!(
            out,
            "[00:00.000]晴天\n[00:02.250]Sunny day[00:02.700]\n\
             [00:02.250]词：周杰伦\n[00:02.250]Lyrics: Jay Chou[00:02.700]\n"
        );
    }

    #[test]
    fn layout_is_ignored_when_translation_is_off() {
        let options = CopyOptions {
            include_translation: false,
            translation_layout: TranslationLayout::Before,
            ..CopyOptions::default()
        };
        assert_eq!(
            plain_lrc(&doc_translated(), &options),
            "[00:00.000]晴天\n[00:02.250]词：周杰伦\n"
        );
    }

    #[test]
    fn plain_text_still_brackets_translation() {
        let out = plain_text(&doc_translated(), &with_layout(TranslationLayout::NextLine));
        assert_eq!(out, "晴天\n（Sunny day）\n词：周杰伦\n（Lyrics: Jay Chou）\n");
    }
}
