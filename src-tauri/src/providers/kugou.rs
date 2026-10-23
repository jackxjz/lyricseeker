//! 酷狗音乐：搜索 + 取逐字歌词（KRC）。
//!
//! 端点（实测）：
//! - 搜索：`https://songsearch.kugou.com/song_search_v2?keyword=..&page=1&pagesize=N&platform=WebFilter`
//! - 歌词候选：`https://lyrics.kugou.com/search?ver=1&man=yes&client=pc&keyword=..&hash=..&timelength=..`
//! - 下载：`https://lyrics.kugou.com/download?ver=1&client=pc&id=..&accesskey=..&fmt=krc&charset=utf8`
//!
//! **注意**：`fmt=krc` 也可能返回明文 LRC，必须先判 `krc1` 魔数再决定是否解密。

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Deserialize;

use lyrics_crypto::decrypter::krc::decrypter::decrypt_lyrics as decrypt_krc;

use crate::model::{AppError, SongCandidate, Source};
use crate::providers::{client, map_http_error};

const SEARCH_URL: &str = "https://songsearch.kugou.com/song_search_v2";
const LYRIC_SEARCH_URL: &str = "https://lyrics.kugou.com/search";
const LYRIC_DOWNLOAD_URL: &str = "https://lyrics.kugou.com/download";

// ---------------------------------------------------------------------------
// 响应体
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SongSearchResponse {
    data: Option<SongSearchData>,
}

#[derive(Debug, Deserialize)]
struct SongSearchData {
    lists: Option<Vec<SongItem>>,
}

#[derive(Debug, Deserialize)]
struct SongItem {
    #[serde(rename = "FileHash")]
    file_hash: String,
    #[serde(rename = "SongName")]
    song_name: String,
    /// 多个歌手以 `、` 或 `&` 连接
    #[serde(rename = "SingerName")]
    singer_name: Option<String>,
    #[serde(rename = "AlbumName")]
    album_name: Option<String>,
    /// 秒
    #[serde(rename = "Duration")]
    duration: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct LyricSearchResponse {
    candidates: Option<Vec<LyricCandidate>>,
}

#[derive(Debug, Deserialize)]
struct LyricCandidate {
    id: String,
    accesskey: String,
    song: Option<String>,
    singer: Option<String>,
    /// 毫秒
    duration: Option<i64>,
    /// 匹配分，越高越可信
    score: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct LyricDownloadResponse {
    status: Option<i32>,
    content: Option<String>,
    /// 后端实际下发的格式，可能与我们请求的 `krc` 不一致
    fmt: Option<String>,
}

// ---------------------------------------------------------------------------
// 搜索
// ---------------------------------------------------------------------------

/// 搜索歌曲。
pub async fn search(keyword: &str, limit: usize) -> Result<Vec<SongCandidate>, AppError> {
    let response = client()
        .get(SEARCH_URL)
        .query(&[
            ("keyword", keyword),
            ("page", "1"),
            ("pagesize", &limit.clamp(1, 50).to_string()),
            ("platform", "WebFilter"),
            ("userid", "0"),
            ("clientver", ""),
            ("isbang", "0"),
            ("filter", "2"),
            ("iscorrection", "1"),
            ("privilege_filter", "0"),
        ])
        .send()
        .await
        .map_err(|error| map_http_error("酷狗搜索", error))?;

    if !response.status().is_success() {
        return Err(AppError::upstream(format!(
            "酷狗搜索返回 HTTP {}",
            response.status().as_u16()
        )));
    }

    let parsed: SongSearchResponse = response
        .json()
        .await
        .map_err(|error| AppError::parse(format!("酷狗搜索响应解析失败：{error}")))?;

    let Some(songs) = parsed.data.and_then(|data| data.lists) else {
        return Ok(Vec::new());
    };

    Ok(songs
        .into_iter()
        .map(|song| SongCandidate {
            source: Source::Kugou,
            song_id: song.file_hash,
            numeric_id: None,
            title: song.song_name,
            artists: split_artists(song.singer_name.as_deref().unwrap_or_default()),
            album: song.album_name.unwrap_or_default(),
            duration_ms: song.duration.map(|seconds| seconds * 1000),
        })
        .collect())
}

/// 按常见分隔符拆分歌手串。
fn split_artists(raw: &str) -> Vec<String> {
    const SEPARATORS: [&str; 5] = ["、", "/", " & ", "&", " feat. "];
    let mut parts = vec![raw.to_string()];
    for separator in SEPARATORS {
        parts = parts
            .into_iter()
            .flat_map(|part| {
                part.split(separator)
                    .map(str::trim)
                    .filter(|piece| !piece.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .collect();
    }
    if parts.is_empty() {
        vec![raw.to_string()]
    } else {
        parts
    }
}

// ---------------------------------------------------------------------------
// 取歌词
// ---------------------------------------------------------------------------

/// 取回的原始歌词。
pub struct RawLyrics {
    pub text: String,
    pub word_synced: bool,
    pub translation: Option<String>,
}

impl RawLyrics {
    fn plain(text: String) -> Self {
        Self {
            word_synced: false,
            text,
            translation: None,
        }
    }
}

/// 取一首歌的逐字歌词。
///
/// 先按 hash + 时长找候选，挑时长最接近、分数最高的一个，再下载。
pub async fn fetch_lyrics(candidate: &SongCandidate) -> Result<RawLyrics, AppError> {
    let duration_ms = candidate.duration_ms.unwrap_or(0);
    let response = client()
        .get(LYRIC_SEARCH_URL)
        .query(&[
            ("ver", "1"),
            ("man", "yes"),
            ("client", "pc"),
            ("keyword", candidate.title.as_str()),
            ("hash", candidate.song_id.as_str()),
            ("timelength", &duration_ms.to_string()),
        ])
        .send()
        .await
        .map_err(|error| map_http_error("酷狗歌词检索", error))?;

    if !response.status().is_success() {
        return Err(AppError::upstream(format!(
            "酷狗歌词检索返回 HTTP {}",
            response.status().as_u16()
        )));
    }

    let parsed: LyricSearchResponse = response
        .json()
        .await
        .map_err(|error| AppError::parse(format!("酷狗歌词检索响应解析失败：{error}")))?;

    let Some(candidates) = parsed.candidates.filter(|list| !list.is_empty()) else {
        return Err(AppError::no_lyrics("该曲目在酷狗没有歌词"));
    };

    let best = pick_best_candidate(candidates, candidate);

    let download = client()
        .get(LYRIC_DOWNLOAD_URL)
        .query(&[
            ("ver", "1"),
            ("client", "pc"),
            ("id", best.id.as_str()),
            ("accesskey", best.accesskey.as_str()),
            ("fmt", "krc"),
            ("charset", "utf8"),
        ])
        .send()
        .await
        .map_err(|error| map_http_error("酷狗歌词下载", error))?;

    if !download.status().is_success() {
        return Err(AppError::upstream(format!(
            "酷狗歌词下载返回 HTTP {}",
            download.status().as_u16()
        )));
    }

    let payload: LyricDownloadResponse = download
        .json()
        .await
        .map_err(|error| AppError::parse(format!("酷狗歌词下载响应解析失败：{error}")))?;

    if let Some(status) = payload.status.filter(|status| *status != 200) {
        return Err(AppError::upstream(format!("酷狗歌词下载返回状态 {status}")));
    }

    let encoded = match payload.content {
        Some(text) if !text.is_empty() => text,
        _ => return Err(AppError::no_lyrics("该曲目在酷狗没有歌词")),
    };

    decode_payload(&encoded, payload.fmt.as_deref())
}

/// 从候选里挑一条：优先时长最接近，其次分数最高。
fn pick_best_candidate(candidates: Vec<LyricCandidate>, target: &SongCandidate) -> LyricCandidate {
    let target_duration = target.duration_ms.map(|ms| ms as i64).unwrap_or(0);
    candidates
        .into_iter()
        .max_by_key(|candidate| {
            let duration_gap = match (candidate.duration, target_duration) {
                (Some(duration), target) if target > 0 => (duration - target).abs(),
                _ => i64::MAX / 2,
            };
            let score = candidate.score.unwrap_or(0) as i64;
            let exact_title = candidate
                .song
                .as_deref()
                .is_some_and(|song| song.trim() == target.title.trim());
            (
                exact_title,
                // 时长差越小越好，取负值参与 max
                -(duration_gap.min(i64::MAX / 2)),
                score,
            )
        })
        .expect("调用方已保证候选非空")
}

/// 解码酷狗下发的歌词正文。
///
/// `fmt` 声称是 `krc` 时也可能给明文 LRC，因此先看 base64 解出来的魔数：
/// `krc1` 才走解密，其余按纯文本处理。
fn decode_payload(encoded: &str, fmt: Option<&str>) -> Result<RawLyrics, AppError> {
    let bytes = BASE64
        .decode(encoded.trim())
        .map_err(|error| AppError::parse(format!("酷狗歌词 base64 解码失败：{error}")))?;

    if bytes.starts_with(b"krc1") {
        return match decrypt_krc(encoded.trim()) {
            Some(text) => Ok(RawLyrics {
                text,
                word_synced: true,
                translation: None,
            }),
            None => Err(AppError::parse("酷狗逐字歌词解密失败")),
        };
    }

    // 不是 krc1：可能是 GB18030/UTF-8 明文 LRC。
    let text = decode_text(&bytes);
    if text.trim().is_empty() {
        return Err(AppError::no_lyrics("该曲目在酷狗没有可用歌词"));
    }
    let _ = fmt;
    Ok(RawLyrics::plain(text))
}

/// 优先按 UTF-8 解码（自动去 BOM），失败时按 GB18030 兜底。
fn decode_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => {
            // GB18030 覆盖 GBK/GB2312；错误字节用替换字符，避免整首丢弃。
            let (decoded, _, _) = encoding_rs::GB18030.decode(bytes);
            decoded.into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, song: &str, duration: i64, score: i32) -> LyricCandidate {
        LyricCandidate {
            id: id.to_string(),
            accesskey: "ak".to_string(),
            song: Some(song.to_string()),
            singer: Some("周杰伦".to_string()),
            duration: Some(duration),
            score: Some(score),
        }
    }

    #[test]
    fn splits_artists_on_common_separators() {
        assert_eq!(split_artists("周杰伦"), vec!["周杰伦"]);
        assert_eq!(split_artists("周杰伦、费玉清"), vec!["周杰伦", "费玉清"]);
        assert_eq!(split_artists("A & B"), vec!["A", "B"]);
        assert_eq!(split_artists("A feat. B"), vec!["A", "B"]);
    }

    #[test]
    fn picking_prefers_closest_duration_over_score() {
        let target = SongCandidate {
            source: Source::Kugou,
            song_id: "hash".into(),
            numeric_id: None,
            title: "晴天".into(),
            artists: vec!["周杰伦".into()],
            album: "叶惠美".into(),
            duration_ms: Some(269_000),
        };
        let picked = pick_best_candidate(
            vec![
                candidate("wrong", "晴天", 130_000, 60),
                candidate("right", "晴天", 269_792, 20),
            ],
            &target,
        );
        assert_eq!(picked.id, "right");
    }

    #[test]
    fn picking_prefers_exact_title_first() {
        let target = SongCandidate {
            source: Source::Kugou,
            song_id: "hash".into(),
            numeric_id: None,
            title: "晴天".into(),
            artists: vec!["周杰伦".into()],
            album: String::new(),
            duration_ms: Some(269_000),
        };
        let picked = pick_best_candidate(
            vec![
                candidate("cover", "晴天 (深情版)", 269_500, 60),
                candidate("original", "晴天", 268_000, 30),
            ],
            &target,
        );
        assert_eq!(picked.id, "original");
    }

    #[test]
    fn plain_lrc_payload_is_not_decrypted() {
        let encoded = BASE64.encode("[ti:晴天]\n[00:00.00]晴天".as_bytes());
        let result = decode_payload(&encoded, Some("krc")).expect("应识别为明文 LRC");
        assert!(!result.word_synced);
        assert!(result.text.contains("[ti:晴天]"));
    }

    #[test]
    fn utf8_bom_is_stripped_from_plain_payload() {
        let mut bytes = b"\xEF\xBB\xBF".to_vec();
        bytes.extend_from_slice("[00:00.00]晴天".as_bytes());
        let encoded = BASE64.encode(&bytes);
        let result = decode_payload(&encoded, Some("lrc")).unwrap();
        assert_eq!(result.text, "[00:00.00]晴天");
    }

    #[test]
    fn gb18030_fallback_decodes() {
        let (bytes, _, _) = encoding_rs::GB18030.encode("[00:00.00]晴天");
        let encoded = BASE64.encode(&bytes);
        let result = decode_payload(&encoded, Some("lrc")).unwrap();
        assert!(result.text.contains("晴天"));
    }
}
