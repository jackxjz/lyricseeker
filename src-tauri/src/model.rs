//! 跨端数据模型（DTO）与错误类型。
//!
//! 所有结构体以 camelCase 序列化，与前端 `src/types.ts` 一一对应。
//! 时间基准统一为**绝对毫秒**。

use serde::{Deserialize, Serialize};

use lyrics_core::models::LineInfo;

// ---------------------------------------------------------------------------
// 错误
// ---------------------------------------------------------------------------

/// 统一的错误类型，`kind` 供前端做分支，`message` 为可直接展示的中文。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: String,
    pub message: String,
}

impl AppError {
    pub fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.into(),
        }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::new("Network", message)
    }

    pub fn parse(message: impl Into<String>) -> Self {
        Self::new("Parse", message)
    }

    pub fn no_lyrics(message: impl Into<String>) -> Self {
        Self::new("NoLyrics", message)
    }

    pub fn no_word_lyrics(message: impl Into<String>) -> Self {
        Self::new("NoWordLyrics", message)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new("Io", message)
    }

    pub fn upstream(message: impl Into<String>) -> Self {
        Self::new("Upstream", message)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.kind, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            AppError::network("请求超时，请检查网络后重试")
        } else if error.is_connect() {
            AppError::network("无法连接上游服务，请检查网络或代理设置")
        } else {
            AppError::network(format!("网络请求失败：{error}"))
        }
    }
}

// ---------------------------------------------------------------------------
// 歌曲候选
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Qq,
    Kugou,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Qq => "qq",
            Source::Kugou => "kugou",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Source::Qq => "QQ 音乐",
            Source::Kugou => "酷狗音乐",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "qq" | "qqmusic" => Some(Source::Qq),
            "kugou" | "kg" => Some(Source::Kugou),
            _ => None,
        }
    }
}

/// 搜索结果里的一条歌曲。`song_id` 对 QQ 是 `songmid`，对酷狗是 `FileHash`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongCandidate {
    pub source: Source,
    pub song_id: String,
    pub numeric_id: Option<i64>,
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration_ms: Option<i32>,
}

impl SongCandidate {
    pub fn artist_text(&self) -> String {
        self.artists.join(", ")
    }
}

// ---------------------------------------------------------------------------
// 歌词
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Syllable {
    pub text: String,
    pub start_ms: i32,
    pub dur_ms: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub start_ms: i32,
    pub dur_ms: Option<i32>,
    pub text: String,
    /// `None` 表示该行没有逐字信息。
    pub syllables: Option<Vec<Syllable>>,
    pub translation: Option<String>,
}

/// 一次取词的完整结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsDoc {
    pub song: SongCandidate,
    /// `qrc` / `krc` / `lrc`
    pub raw_fmt: String,
    pub lines: Vec<LyricLine>,
    /// 是否真的拿到了逐字信息。
    pub word_synced: bool,
    /// 是否来自本地缓存。
    pub from_cache: bool,
    /// 降级说明（如"该歌曲无逐字歌词，已降级为逐行"）。
    pub notice: Option<String>,
}

impl LyricsDoc {
    /// 收藏到剪贴板的纯文本行（供前端显示字数、Toast 预览）。
    pub fn line_count(&self) -> usize {
        self.lines.iter().filter(|l| !l.text.trim().is_empty()).count()
    }
}

// ---------------------------------------------------------------------------
// lyrics-core -> DTO 转换
// ---------------------------------------------------------------------------

/// 把 `lyrics-core` 的行模型转成 DTO 行；无开始时间的行直接丢弃。
pub fn convert_lines(lines: &[LineInfo]) -> Vec<LyricLine> {
    lines
        .iter()
        .filter_map(|line| {
            let start_ms = line.start_time()?;
            let syllables = line.syllables().map(|items| {
                items
                    .iter()
                    .flat_map(|item| item.parts())
                    .map(|part| Syllable {
                        text: part.text.clone(),
                        start_ms: part.start_time,
                        dur_ms: part.end_time - part.start_time,
                    })
                    .collect::<Vec<_>>()
            });
            Some(LyricLine {
                start_ms,
                dur_ms: line.duration(),
                text: line.full_text(),
                syllables,
                translation: None,
            })
        })
        .collect()
}

/// 把翻译歌词按开始时间贴到主歌词行上（时间必须完全相等才贴）。
pub fn attach_translations(lines: &mut [LyricLine], translations: &[LyricLine]) {
    if translations.is_empty() {
        return;
    }
    for line in lines.iter_mut() {
        if let Some(found) = translations
            .iter()
            .find(|t| t.start_ms == line.start_ms && !t.text.trim().is_empty())
        {
            line.translation = Some(found.text.clone());
        }
    }
}

/// 判断解析结果里是否真的存在逐字行。
pub fn has_syllables(lines: &[LyricLine]) -> bool {
    lines.iter().any(|line| {
        line.syllables
            .as_ref()
            .is_some_and(|syllables| syllables.len() > 1)
    })
}
