//! 取词后的解析与 DTO 组装。

use lyrics_parsers::parsers::{parse_lyrics, parse_lyrics_auto};

use crate::model::{
    LyricsDoc, LyricLine, SongCandidate, Source, attach_translations, convert_lines, has_syllables,
};

/// 解析后的歌词与来源信息。
pub struct Parsed {
    pub raw_fmt: &'static str,
    pub lines: Vec<LyricLine>,
    pub word_synced: bool,
    pub notice: Option<String>,
}

/// 解析 QQ 的歌词正文。
///
/// `word_synced` 由 providers 判定（QRC 分支为真）；解析失败时退回自动探测，
/// 仍失败则把整段正文当成单行纯文本，保证用户至少能复制点东西。
pub fn parse_qq(text: &str, word_synced: bool) -> Parsed {
    parse_with_fallback(text, word_synced, "Qrc", "qrc", "QQ 音乐返回的逐字歌词内容为空，已降级为逐行")
}

/// 解析酷狗的歌词正文。
pub fn parse_kugou(text: &str, word_synced: bool) -> Parsed {
    parse_with_fallback(text, word_synced, "Krc", "krc", "酷狗返回的逐字歌词无法解析，已降级为逐行")
}

/// 两种来源共用的解析流程。
///
/// 解析器在「没有可识别行」时仍可能返回 `Some`（只是 `lines` 为空），
/// 因此除了 `None` 之外还要判空，两种情况都退回纯文本，保证一定有内容可复制。
fn parse_with_fallback(
    text: &str,
    word_synced: bool,
    syllable_raw: &'static str,
    word_fmt: &'static str,
    downgrade_notice: &'static str,
) -> Parsed {
    let raw_type = if word_synced {
        match syllable_raw {
            "Qrc" => lyrics_core::models::LyricsRawTypes::Qrc,
            _ => lyrics_core::models::LyricsRawTypes::Krc,
        }
    } else {
        lyrics_core::models::LyricsRawTypes::Lrc
    };

    let parsed = parse_lyrics(text, raw_type)
        .or_else(|| parse_lyrics_auto(text))
        .map(|data| convert_lines(data.lines.as_deref().unwrap_or_default()))
        .filter(|lines| !lines.is_empty());

    match parsed {
        Some(lines) => {
            let word = has_syllables(&lines);
            Parsed {
                raw_fmt: if word { word_fmt } else { "lrc" },
                lines,
                word_synced: word,
                notice: if word_synced && !word {
                    Some(downgrade_notice.to_string())
                } else {
                    None
                },
            }
        }
        None => fallback_text(text),
    }
}

/// 解析失败时把正文按行拆成无时间戳的纯文本行。
fn fallback_text(text: &str) -> Parsed {
    let lines: Vec<LyricLine> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| LyricLine {
            start_ms: 0,
            dur_ms: None,
            text: line.to_string(),
            syllables: None,
            translation: None,
        })
        .collect();

    Parsed {
        raw_fmt: "lrc",
        lines,
        word_synced: false,
        notice: Some("歌词格式无法识别，已按纯文本展示".to_string()),
    }
}

/// 把翻译（纯文本 LRC）贴到主歌词行上。
pub fn attach_translation(parsed: &mut Parsed, translation: &str) {
    let Some(data) = parse_lyrics(translation, lyrics_core::models::LyricsRawTypes::Lrc)
        .or_else(|| parse_lyrics_auto(translation))
    else {
        return;
    };
    let translations = convert_lines(data.lines.as_deref().unwrap_or_default());
    attach_translations(&mut parsed.lines, &translations);
}

/// 组装最终 DTO。
pub fn into_doc(candidate: SongCandidate, parsed: Parsed) -> LyricsDoc {
    let notice = parsed.notice.or_else(|| {
        if parsed.word_synced {
            None
        } else {
            Some("该歌曲没有逐字歌词，已降级为逐行".to_string())
        }
    });

    LyricsDoc {
        song: candidate,
        raw_fmt: parsed.raw_fmt.to_string(),
        lines: parsed.lines,
        word_synced: parsed.word_synced,
        from_cache: false,
        notice,
    }
}

/// 按来源分派的便捷入口。
pub fn parse_for(source: Source, text: &str, word_synced: bool) -> Parsed {
    match source {
        Source::Qq => parse_qq(text, word_synced),
        Source::Kugou => parse_kugou(text, word_synced),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QRC_SAMPLE: &str = "[ti:晴天]\n[ar:周杰伦]\n[0,2250]晴(0,160)天(160,160) (320,160)-(480,160)周(640,160)杰(800,160)伦(960,160)\n[2250,2250]词(2250,450)：(2700,450)周(3150,450)\n";

    const KRC_SAMPLE: &str = "[ti:晴天]\n[ar:周杰伦]\n[0,2250]<0,160,0>晴<160,160,0>天<320,160,0> <480,160,0>-\n[2250,2250]<0,450,0>词<450,450,0>：\n";

    const LRC_SAMPLE: &str = "[ti:晴天]\n[00:00.00]晴天 - 周杰伦\n[00:02.25]词：周杰伦\n";

    #[test]
    fn qrc_sample_yields_syllables_and_absolute_times() {
        let parsed = parse_qq(QRC_SAMPLE, true);
        assert!(parsed.word_synced, "QRC 应识别为逐字");
        assert_eq!(parsed.raw_fmt, "qrc");
        assert_eq!(parsed.lines.len(), 2);

        let first = &parsed.lines[0];
        assert_eq!(first.start_ms, 0);
        let syllables = first.syllables.as_ref().expect("首行应有逐字");
        assert_eq!(syllables[0].text, "晴");
        assert_eq!(syllables[0].start_ms, 0);
        assert_eq!(syllables[1].start_ms, 160);
        // QRC 的字符时间是绝对值，第二行首字应与行起始一致。
        let second = &parsed.lines[1];
        let second_syllables = second.syllables.as_ref().expect("第二行应有逐字");
        assert_eq!(second_syllables[0].start_ms, second.start_ms);
    }

    #[test]
    fn krc_sample_yields_syllables_with_absolute_times() {
        let parsed = parse_kugou(KRC_SAMPLE, true);
        assert!(parsed.word_synced, "KRC 应识别为逐字");
        assert_eq!(parsed.raw_fmt, "krc");
        let second = &parsed.lines[1];
        let syllables = second.syllables.as_ref().expect("第二行应有逐字");
        // KRC 行内相对时间必须已被解析器加和成绝对时间。
        assert_eq!(syllables[0].start_ms, second.start_ms);
    }

    #[test]
    fn plain_lrc_has_no_syllables_and_gets_notice() {
        let parsed = parse_qq(LRC_SAMPLE, false);
        assert!(!parsed.word_synced);
        assert_eq!(parsed.raw_fmt, "lrc");
        assert!(parsed.lines.iter().all(|line| line.syllables.is_none()));
        assert!(parsed.notice.is_none(), "逐行歌词不应报降级");

        let doc = into_doc(sample_candidate(), parsed);
        assert!(doc.notice.is_some(), "无逐字时应有降级提示");
    }

    #[test]
    fn translation_is_attached_by_matching_start_time() {
        let mut parsed = parse_qq(QRC_SAMPLE, true);
        attach_translation(&mut parsed, "[00:00.00]Fine Day\n[00:02.25]Lyrics: Jay Chou\n");
        assert_eq!(
            parsed.lines[0].translation.as_deref(),
            Some("Fine Day"),
            "首行翻译应按时间贴上"
        );
    }

    #[test]
    fn unparsable_text_falls_back_to_plain_lines() {
        let parsed = parse_qq("这不是歌词\n只是两行字", false);
        assert_eq!(parsed.lines.len(), 2);
        assert!(!parsed.word_synced);
        assert!(parsed.notice.is_some());
    }

    fn sample_candidate() -> SongCandidate {
        SongCandidate {
            source: Source::Qq,
            song_id: "0039MnYb0qxYhV".into(),
            numeric_id: Some(97773),
            title: "晴天".into(),
            artists: vec!["周杰伦".into()],
            album: "叶惠美".into(),
            duration_ms: Some(269_000),
        }
    }
}
