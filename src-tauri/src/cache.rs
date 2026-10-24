//! 磁盘缓存：`%APPDATA%\com.lyricseeker.app\cache\{source}-{songId}.json`
//!
//! 只缓存取词结果（搜索不缓存）。24 小时内命中直接返回。

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::model::{AppError, LyricsDoc, Source};

const CACHE_TTL_SECS: u64 = 24 * 60 * 60;

/// 缓存结构版本。改了取词/解析逻辑导致旧结果不再可信时递增，
/// 旧文件会在读取时被当作未命中并删除。
///
/// v2：补齐翻译（QQ 改用容差匹配，酷狗新增 KRC `[language:]` 解析），
/// v1 存下的文档整首几乎没有译文，必须让它们失效。
const CACHE_SCHEMA: u32 = 2;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheEntry {
    /// 旧文件没有这个字段，`serde` 会补 0，因此一定被判为过期。
    #[serde(default)]
    schema: u32,
    fetched_at: u64,
    doc: LyricsDoc,
}

fn cache_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("com.lyricseeker.app").join("cache")
}

/// 文件名里的 `songId` 可能含 `/` 之类字符（QQ 的 mid 一般是字母数字），做一次安全化。
fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn entry_path(source: Source, song_id: &str) -> PathBuf {
    cache_dir().join(format!(
        "{}-{}.json",
        source.as_str(),
        sanitize(song_id)
    ))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// 读缓存；过期、版本不符或损坏都当作未命中。
pub fn load(source: Source, song_id: &str) -> Option<LyricsDoc> {
    let path = entry_path(source, song_id);
    let raw = fs::read_to_string(&path).ok()?;
    let entry: CacheEntry = serde_json::from_str(&raw).ok()?;

    if entry.schema != CACHE_SCHEMA {
        let _ = fs::remove_file(&path);
        return None;
    }

    if now_secs().saturating_sub(entry.fetched_at) > CACHE_TTL_SECS {
        // 过期即删，避免目录长期膨胀。
        let _ = fs::remove_file(&path);
        return None;
    }

    let mut doc = entry.doc;
    doc.from_cache = true;
    Some(doc)
}

/// 写缓存。失败不影响主流程，只记日志。
pub fn save(doc: &LyricsDoc) {
    if let Err(error) = try_save(doc) {
        log::warn!("写歌词缓存失败：{error}");
    }
}

fn try_save(doc: &LyricsDoc) -> Result<(), AppError> {
    let dir = cache_dir();
    fs::create_dir_all(&dir)
        .map_err(|error| AppError::io(format!("创建缓存目录失败：{error}")))?;

    let mut stored = doc.clone();
    stored.from_cache = false;

    let entry = CacheEntry {
        schema: CACHE_SCHEMA,
        fetched_at: now_secs(),
        doc: stored,
    };
    let text = serde_json::to_string(&entry)
        .map_err(|error| AppError::io(format!("序列化缓存失败：{error}")))?;

    fs::write(entry_path(doc.song.source, &doc.song.song_id), text)
        .map_err(|error| AppError::io(format!("写入缓存失败：{error}")))
}

/// 清空缓存，返回删除的文件数。
pub fn clear() -> Result<u64, AppError> {
    let dir = cache_dir();
    if !dir.exists() {
        return Ok(0);
    }

    let mut removed = 0u64;
    let entries =
        fs::read_dir(&dir).map_err(|error| AppError::io(format!("读取缓存目录失败：{error}")))?;
    for entry in entries.flatten() {
        if entry.path().extension().is_some_and(|ext| ext == "json")
            && fs::remove_file(entry.path()).is_ok()
        {
            removed += 1;
        }
    }
    Ok(removed)
}

/// 供「缓存位置」提示使用。
pub fn location() -> String {
    cache_dir().to_string_lossy().to_string()
}

/// 缓存有效期（秒），供前端文案使用。
pub const TTL_SECS: u64 = CACHE_TTL_SECS;

/// `Duration` 只是为了让文档与实现一致；此处仅导出一个便于阅读的常量。
pub const TTL: Duration = Duration::from_secs(CACHE_TTL_SECS);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_qq_mid_intact() {
        assert_eq!(sanitize("0039MnYb0qxYhV"), "0039MnYb0qxYhV");
    }

    #[test]
    fn sanitize_replaces_path_separators() {
        assert_eq!(sanitize("a/b\\c"), "a_b_c");
    }

    #[test]
    fn entry_path_is_inside_cache_dir() {
        let path = entry_path(Source::Qq, "0039MnYb0qxYhV");
        assert!(path.starts_with(cache_dir()));
        assert!(path.to_string_lossy().ends_with("qq-0039MnYb0qxYhV.json"));
    }

    #[test]
    fn legacy_entry_without_schema_is_rejected() {
        // 修复翻译之前写下的缓存没有 schema 字段，必须被当作未命中。
        let legacy = r#"{"fetchedAt":9999999999,"doc":{"song":{"source":"qq","songId":"x","numericId":null,"title":"t","artists":[],"album":"","durationMs":null},"rawFmt":"qrc","lines":[],"wordSynced":true,"fromCache":false,"notice":null}}"#;
        let entry: CacheEntry = serde_json::from_str(legacy).expect("字段缺失也应能解析");
        assert_eq!(entry.schema, 0);
        assert_ne!(entry.schema, CACHE_SCHEMA, "旧缓存必须被判为过期");
    }

    #[test]
    fn saved_entry_round_trips_with_current_schema() {
        let entry = CacheEntry {
            schema: CACHE_SCHEMA,
            fetched_at: now_secs(),
            doc: LyricsDoc {
                song: crate::model::SongCandidate {
                    source: Source::Kugou,
                    song_id: "hash".into(),
                    numeric_id: None,
                    title: "歌".into(),
                    artists: vec!["唱".into()],
                    album: String::new(),
                    duration_ms: Some(1000),
                },
                raw_fmt: "krc".into(),
                lines: Vec::new(),
                word_synced: true,
                from_cache: false,
                notice: None,
            },
        };
        let text = serde_json::to_string(&entry).unwrap();
        let back: CacheEntry = serde_json::from_str(&text).unwrap();
        assert_eq!(back.schema, CACHE_SCHEMA);
    }
}
