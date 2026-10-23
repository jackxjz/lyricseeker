import { invoke } from "@tauri-apps/api/core";
import type {
  CacheInfo,
  CopyFormat,
  CopyOptions,
  GeneratedText,
  LyricsDoc,
  SongCandidate,
  SourceFilter,
} from "./types";

/** 搜索歌曲；`source` 传 "all" 时两端并发。 */
export function searchSongs(
  source: SourceFilter,
  keyword: string,
  limit = 30,
): Promise<SongCandidate[]> {
  return invoke<SongCandidate[]>("search", { source, keyword, limit });
}

/** 取歌词；命中缓存直接返回。 */
export function fetchLyrics(
  candidate: SongCandidate,
  forceRefresh = false,
): Promise<LyricsDoc> {
  return invoke<LyricsDoc>("fetch_lyrics", { candidate, forceRefresh });
}

/** 在 Rust 侧生成待复制的文本。 */
export function generateText(
  doc: LyricsDoc,
  formatName: CopyFormat,
  options: CopyOptions,
): Promise<GeneratedText> {
  return invoke<GeneratedText>("generate_text", { doc, formatName, options });
}

export function clearCache(): Promise<number> {
  return invoke<number>("clear_cache");
}

export function getCacheInfo(): Promise<CacheInfo> {
  return invoke<CacheInfo>("cache_info");
}

export function setAlwaysOnTop(on: boolean): Promise<void> {
  return invoke<void>("set_always_on_top", { on });
}
