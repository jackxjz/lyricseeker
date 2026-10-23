import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  clearCache,
  fetchLyrics,
  generateText,
  getCacheInfo,
  searchSongs,
  setAlwaysOnTop,
} from "./api";
import type {
  CacheInfo,
  CopyFormat,
  CopyOptions,
  LyricsDoc,
  SongCandidate,
  SourceFilter,
} from "./types";
import { DEFAULT_COPY_OPTIONS, describeError, FORMAT_LABEL } from "./types";

import { SearchBar } from "./components/SearchBar";
import { SongList } from "./components/SongList";
import { LyricPreview } from "./components/LyricPreview";
import { CopyBar } from "./components/CopyBar";
import { StatusBar } from "./components/StatusBar";
import { Toast, type ToastState } from "./components/Toast";

type Pending = "search" | "lyrics" | "copy" | null;

export default function App() {
  const [source, setSource] = useState<SourceFilter>("all");
  const [keyword, setKeyword] = useState("");
  const [results, setResults] = useState<SongCandidate[]>([]);
  const [selected, setSelected] = useState<SongCandidate | null>(null);
  const [doc, setDoc] = useState<LyricsDoc | null>(null);
  const [options, setOptions] = useState<CopyOptions>(DEFAULT_COPY_OPTIONS);
  const [status, setStatus] = useState("输入歌名或歌手开始检索");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState<Pending>(null);
  const [toast, setToast] = useState<ToastState | null>(null);
  const [onTop, setOnTop] = useState(false);
  const [cache, setCache] = useState<CacheInfo | null>(null);

  const searchInput = useRef<HTMLInputElement | null>(null);
  const toastTimer = useRef<number | null>(null);

  const showToast = useCallback((message: string, tone: ToastState["tone"] = "ok") => {
    setToast({ message, tone });
    if (toastTimer.current !== null) window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 2800);
  }, []);

  useEffect(() => {
    getCacheInfo()
      .then(setCache)
      .catch(() => setCache(null));
  }, []);

  const doSearch = useCallback(async (query: string, from: SourceFilter) => {
    const trimmed = query.trim();
    if (!trimmed) {
      setStatus("请输入歌名或歌手");
      searchInput.current?.focus();
      return;
    }

    setPending("search");
    setError(null);
    setStatus(`正在搜索「${trimmed}」…`);
    try {
      const found = await searchSongs(from, trimmed, 30);
      setResults(found);
      setSelected(null);
      setDoc(null);
      setStatus(
        found.length === 0
          ? `没有找到「${trimmed}」，换个关键词试试`
          : `找到 ${found.length} 首，选择一首查看歌词`,
      );
    } catch (caught) {
      setResults([]);
      setSelected(null);
      setDoc(null);
      setError(describeError(caught));
      setStatus("搜索失败");
    } finally {
      setPending(null);
    }
  }, []);

  const doFetch = useCallback(async (candidate: SongCandidate, forceRefresh = false) => {
    setSelected(candidate);
    setPending("lyrics");
    setError(null);
    setStatus(`正在获取「${candidate.title}」的歌词…`);
    try {
      const lyrics = await fetchLyrics(candidate, forceRefresh);
      setDoc(lyrics);
      const wordLines = lyrics.lines.filter((line) => line.syllables).length;
      const prefix = lyrics.fromCache ? "缓存命中 · " : "";
      setStatus(
        lyrics.wordSynced
          ? `${prefix}逐字歌词 · ${lyrics.lines.length} 行（${wordLines} 行带逐字）`
          : `${prefix}逐行歌词 · ${lyrics.lines.length} 行`,
      );
    } catch (caught) {
      setDoc(null);
      setError(describeError(caught));
      setStatus("获取歌词失败");
    } finally {
      setPending(null);
    }
  }, []);

  const doCopy = useCallback(
    async (formatName: CopyFormat) => {
      if (!doc) return;
      setPending("copy");
      try {
        const generated = await generateText(doc, formatName, options);
        await writeText(generated.text);
        showToast(
          `已复制${FORMAT_LABEL[formatName]} · ${generated.lineCount} 行 / ${generated.charCount} 字`,
        );
      } catch (caught) {
        showToast(describeError(caught), "error");
      } finally {
        setPending(null);
      }
    },
    [doc, options, showToast],
  );

  const toggleOnTop = useCallback(async () => {
    const next = !onTop;
    try {
      await setAlwaysOnTop(next);
      setOnTop(next);
      showToast(next ? "窗口已置顶" : "已取消置顶");
    } catch (caught) {
      showToast(describeError(caught), "error");
    }
  }, [onTop, showToast]);

  const doClearCache = useCallback(async () => {
    try {
      const removed = await clearCache();
      showToast(`已清除 ${removed} 条缓存`);
      getCacheInfo()
        .then(setCache)
        .catch(() => undefined);
    } catch (caught) {
      showToast(describeError(caught), "error");
    }
  }, [showToast]);

  // Ctrl+1..4 复制四种格式，Ctrl+L 聚焦搜索框。
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (!event.ctrlKey) return;
      const formats: CopyFormat[] = ["enhanced", "raw", "lrc", "text"];
      const index = Number(event.key) - 1;
      if (Number.isInteger(index) && index >= 0 && index < formats.length) {
        event.preventDefault();
        void doCopy(formats[index]);
      } else if (event.key.toLowerCase() === "l") {
        event.preventDefault();
        searchInput.current?.focus();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [doCopy]);

  useEffect(() => {
    return () => {
      if (toastTimer.current !== null) window.clearTimeout(toastTimer.current);
    };
  }, []);

  const hasDoc = useMemo(() => doc !== null && doc.lines.length > 0, [doc]);

  return (
    <div className="app">
      <SearchBar
        inputRef={searchInput}
        keyword={keyword}
        source={source}
        busy={pending === "search"}
        onKeywordChange={setKeyword}
        onSourceChange={setSource}
        onSubmit={() => void doSearch(keyword, source)}
        onTop={onTop}
        onToggleTop={() => void toggleOnTop()}
      />

      {doc?.notice ? <div className="banner">{doc.notice}</div> : null}

      <main className="panes">
        <SongList
          results={results}
          selected={selected}
          busy={pending === "lyrics"}
          onPick={(candidate) => void doFetch(candidate)}
        />
        <LyricPreview doc={doc} busy={pending === "lyrics"} />
      </main>

      <CopyBar
        doc={doc}
        options={options}
        busy={pending === "copy"}
        disabled={!hasDoc}
        onOptionsChange={setOptions}
        onCopy={(formatName) => void doCopy(formatName)}
        onRefresh={() => {
          if (selected) void doFetch(selected, true);
        }}
        onClearCache={() => void doClearCache()}
        cacheLocation={cache?.location ?? null}
      />

      <StatusBar
        status={status}
        error={error}
        pending={pending}
        onDismissError={() => setError(null)}
      />

      <Toast toast={toast} />
    </div>
  );
}
