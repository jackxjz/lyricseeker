//! 四种复制格式的文本发射器。
//!
//! 全部从 `LyricsDoc` DTO 生成，因此不依赖上游原始格式，
//! QQ（QRC）与酷狗（KRC）走同一套代码，输出可预期。

use crate::model::LyricsDoc;

/// 复制选项，与前端 `CopyOptions` 一一对应。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyOptions {
    /// 是否输出 `[ti:]/[ar:]/[al:]` 标签行。
    #[serde(default)]
    pub include_metadata: bool,
    /// 是否在每行后追加翻译行（`译：xxx`）。
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

fn metadata_header(doc: &LyricsDoc) -> String {
    let mut out = String::new();
    out.push_str(&format!("[ti:{}]\n", doc.song.title));
    out.push_str(&format!("[ar:{}]\n", doc.song.artist_text()));
    if !doc.song.album.is_empty() {
        out.push_str(&format!("[al:{}]\n", doc.song.album));
    }
    out
}

// ---------------------------------------------------------------------------
// 1) 逐字（增强 LRC）
// ---------------------------------------------------------------------------

/// 生成增强 LRC：`[行时间]<字时间>字<字时间>字`。
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

        match &line.syllables {
            Some(syllables) if !syllables.is_empty() => {
                for syllable in syllables {
                    if !options.keep_spaces && syllable.text.trim().is_empty() {
                        continue;
                    }
                    out.push('<');
                    out.push_str(&format_timestamp(syllable.start_ms));
                    out.push('>');
                    out.push_str(&syllable.text);
                }
            }
            _ => out.push_str(&line.text),
        }

        out.push('\n');

        if options.include_translation {
            if let Some(translation) = line.translation.as_ref().filter(|t| !t.trim().is_empty()) {
                out.push_str(&format!("译：{translation}\n"));
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// 2) 逐字（原始记号）
// ---------------------------------------------------------------------------

/// 生成逐字原始记号：`[行开始,行时长]字(字开始,字时长)字(...)`。
///
/// 与 QRC 正文同形；行时长缺失时写 0。
pub fn raw_syllable(doc: &LyricsDoc, options: &CopyOptions) -> String {
    let mut out = String::new();

    for line in &doc.lines {
        let Some(syllables) = line.syllables.as_ref().filter(|s| !s.is_empty()) else {
            // 没有逐字信息时仍输出行文本，避免用户拿到空结果。
            if line.text.trim().is_empty() {
                continue;
            }
            if should_skip(&line.text, options) {
                continue;
            }
            out.push_str(&format!(
                "[{},{}]{}",
                line.start_ms,
                line.dur_ms.unwrap_or(0),
                line.text
            ));
            out.push('\n');
            continue;
        };

        if should_skip(&line.text, options) {
            continue;
        }

        out.push_str(&format!(
            "[{},{}]",
            line.start_ms,
            line.dur_ms.unwrap_or(0)
        ));
        for syllable in syllables {
            if !options.keep_spaces && syllable.text.trim().is_empty() {
                continue;
            }
            out.push_str(&format!(
                "{}({},{})",
                syllable.text, syllable.start_ms, syllable.dur_ms
            ));
        }
        out.push('\n');
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
        out.push_str(&line.text);
        out.push('\n');

        if options.include_translation {
            if let Some(translation) = line.translation.as_ref().filter(|t| !t.trim().is_empty()) {
                out.push_str(&format!("译：{translation}\n"));
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// 4) 纯文本
// ---------------------------------------------------------------------------

/// 生成纯文本，只保留歌词正文。
pub fn plain_text(doc: &LyricsDoc, options: &CopyOptions) -> String {
    let mut out = String::new();

    for line in &doc.lines {
        if line.text.trim().is_empty() {
            continue;
        }
        if should_skip(&line.text, options) {
            continue;
        }
        out.push_str(&line.text);
        out.push('\n');

        if options.include_translation {
            if let Some(translation) = line.translation.as_ref().filter(|t| !t.trim().is_empty()) {
                out.push_str(&format!("译：{translation}\n"));
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
        "raw" => raw_syllable(doc, options),
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
                    text: "晴天 - 周杰伦".into(),
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
            "[00:00.000]<00:00.000>晴<00:00.160>天\n[00:02.250]<00:02.250>词：周杰伦\n"
        );
    }

    #[test]
    fn raw_syllable_matches_qrc_shape() {
        let options = CopyOptions::default();
        assert_eq!(
            raw_syllable(&doc(), &options),
            "[0,2250]晴(0,160)天(160,160)\n[2250,2250]词：周杰伦(2250,450)\n"
        );
    }

    #[test]
    fn plain_lrc_has_only_line_timestamps() {
        let options = CopyOptions::default();
        assert_eq!(
            plain_lrc(&doc(), &options),
            "[00:00.000]晴天 - 周杰伦\n[00:02.250]词：周杰伦\n"
        );
    }

    #[test]
    fn plain_text_strips_everything() {
        let options = CopyOptions::default();
        assert_eq!(plain_text(&doc(), &options), "晴天 - 周杰伦\n词：周杰伦\n");
    }

    #[test]
    fn metadata_and_translation_are_opt_in() {
        let options = CopyOptions {
            include_metadata: true,
            include_translation: true,
            ..CopyOptions::default()
        };
        let out = plain_lrc(&doc(), &options);
        assert!(out.starts_with("[ti:晴天]\n[ar:周杰伦]\n[al:叶惠美]\n"));
        assert!(out.contains("译：Lyrics: Jay Chou"));
    }

    #[test]
    fn credits_can_be_filtered() {
        let options = CopyOptions {
            filter_credits: true,
            ..CopyOptions::default()
        };
        assert_eq!(plain_text(&doc(), &options), "晴天 - 周杰伦\n");
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
    }
}
