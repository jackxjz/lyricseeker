import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
  CopyToggleKey,
  LyricsDoc,
  SongCandidate,
  Source,
  SourceFilter,
  ThemeMode,
  TranslationLayout,
} from "./types";
import { describeError, FORMAT_LABEL, FORMAT_ORDER } from "./types";
import { DEFAULT_SETTINGS, loadSettings, saveSettings, type Settings } from "./settings";

import { SearchBar } from "./components/SearchBar";
import { SongList } from "./components/SongList";
import { LyricPreview } from "./components/LyricPreview";
import { CopyBar } from "./components/CopyBar";
import { StatusBar } from "./components/StatusBar";
import { SettingsPage } from "./components/SettingsPage";
import { Toast, type ToastState } from "./components/Toast";

type Pending = "search" | "lyrics" | "copy" | null;

export default function App() {
  const [source, setSource] = useState<SourceFilter>("all");
  const [keyword, setKeyword] = useState("");
  const [results, setResults] = useState<SongCandidate[]>([]);
  const [selected, setSelected] = useState<SongCandidate | null>(null);
  const [doc, setDoc] = useState<LyricsDoc | null>(null);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [status, setStatus] = useState("输入歌名或歌手开始检索");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState<Pending>(null);
  const [toast, setToast] = useState<ToastState | null>(null);
  const [onTop, setOnTop] = useState(false);
  const [cache, setCache] = useState<CacheInfo | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const [cacheBusy, setCacheBusy] = useState(false);

  const searchInput = useRef<HTMLInputElement | null>(null);
  const toastTimer = useRef<number | null>(null);

  const showToast = useCallback((message: string, tone: ToastState["tone"] = "ok") => {
    setToast({ message, tone });
    if (toastTimer.current !== null) window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 2800);
  }, []);

  const refreshCache = useCallback(() => {
    getCacheInfo()
      .then(setCache)
      .catch(() => setCache(null));
  }, []);

  // 启动时读回上次的设置。用 dirty 标记保证：如果读取还没返回用户就已经改了
  // 设置，后到的旧值不能把新改动冲掉（否则表现为「刚点的开关自己弹回去」）。
  const dirty = useRef(false);

  useEffect(() => {
    loadSettings()
      .then((loaded) => {
        if (!dirty.current) setSettings(loaded);
      })
      .finally(() => setSettingsLoaded(true));
    refreshCache();
  }, [refreshCache]);

  useEffect(() => {
    if (settingsLoaded && dirty.current) void saveSettings(settings);
  }, [settings, settingsLoaded]);

  // 主题：`data-theme` 让 CSS 里的 light-dark() 知道该取哪一侧；同时告诉
  // 窗口自己（WebView2 + 原生标题栏）跟随哪种配色，否则标题栏会一直黑白
  // 对不上页面。传 null 表示交还给系统。
  useEffect(() => {
    const mode = settings.theme;
    const root = document.documentElement;
    if (mode === "system") {
      delete root.dataset.theme;
    } else {
      root.dataset.theme = mode;
    }
    getCurrentWindow()
      .setTheme(mode === "system" ? null : mode)
      .catch((error: unknown) => {
        // 换不了配色只是难看，不该打断使用。
        console.warn("切换窗口主题失败", error);
      });
  }, [settings.theme]);

  // 源被关掉后，主页的 tab 要跟着收口，否则会停在一个不存在的选项上。
  useEffect(() => {
    const enabled = settings.enabledSources;
    if (enabled.length === 0) return;
    if (source === "all") {
      // 只剩一个源时「全部」等价于那个源，直接切过去让 tab 高亮正确。
      if (enabled.length === 1) setSource(enabled[0]);
      return;
    }
    if (!enabled.includes(source)) setSource(enabled.length === 1 ? enabled[0] : "all");
  }, [settings.enabledSources, source]);

  const doSearch = useCallback(
    async (query: string, from: SourceFilter) => {
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
        const found = await searchSongs(from, trimmed, settings.enabledSources, 30);
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
    },
    [settings.enabledSources],
  );

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
      refreshCache();
    } catch (caught) {
      setDoc(null);
      setError(describeError(caught));
      setStatus("获取歌词失败");
    } finally {
      setPending(null);
    }
  }, [refreshCache]);

  const doCopy = useCallback(
    async (formatName: CopyFormat) => {
      if (!doc) {
        showToast("先选一首歌拿到歌词，再复制", "error");
        return;
      }
      setPending("copy");
      try {
        const generated = await generateText(doc, formatName, settings.copy);
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
    [doc, settings.copy, showToast],
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

  const toggleSource = useCallback((target: Source) => {
    dirty.current = true;
    setSettings((current) => {
      const on = current.enabledSources.includes(target);
      const next = on
        ? current.enabledSources.filter((item) => item !== target)
        : [...current.enabledSources, target];
      // 顺序固定成 qq → kugou，避免每次开关都把 tab 顺序洗一遍。
      const ordered = (["qq", "kugou"] as Source[]).filter((item) => next.includes(item));
      return { ...current, enabledSources: ordered };
    });
  }, []);

  const toggleCopy = useCallback((key: CopyToggleKey) => {
    dirty.current = true;
    setSettings((current) => ({
      ...current,
      copy: { ...current.copy, [key]: !current.copy[key] } as CopyOptions,
    }));
  }, []);

  const selectTranslation = useCallback((layout: TranslationLayout) => {
    dirty.current = true;
    setSettings((current) => ({
      ...current,
      copy: { ...current.copy, translationLayout: layout },
    }));
  }, []);

  const selectTheme = useCallback((mode: ThemeMode) => {
    dirty.current = true;
    setSettings((current) => ({ ...current, theme: mode }));
  }, []);

  const doClearCache = useCallback(async () => {
    setCacheBusy(true);
    try {
      const removed = await clearCache();
      showToast(`已清除 ${removed} 条缓存`);
      refreshCache();
    } catch (caught) {
      showToast(describeError(caught), "error");
    } finally {
      setCacheBusy(false);
    }
  }, [showToast, refreshCache]);

  // Ctrl+1..4 复制四种格式，Ctrl+L 聚焦搜索框，Ctrl+, 开关设置页。
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setSettingsOpen(false);
        return;
      }
      if (!event.ctrlKey) return;
      if (event.key === ",") {
        event.preventDefault();
        setSettingsOpen((open) => !open);
        return;
      }
      // 设置页打开时不抢复制快捷键，避免误触后不知道复制到了什么。
      if (settingsOpen) return;
      const index = Number(event.key) - 1;
      if (Number.isInteger(index) && index >= 0 && index < FORMAT_ORDER.length) {
        event.preventDefault();
        void doCopy(FORMAT_ORDER[index]);
      } else if (event.key.toLowerCase() === "l") {
        event.preventDefault();
        searchInput.current?.focus();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [doCopy, settingsOpen]);

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
        enabledSources={settings.enabledSources}
        busy={pending === "search"}
        onKeywordChange={setKeyword}
        onSourceChange={setSource}
        onSubmit={() => void doSearch(keyword, source)}
        onTop={onTop}
        onToggleTop={() => void toggleOnTop()}
        onOpenSettings={() => setSettingsOpen(true)}
      />

      {doc?.notice ? <div className="banner">{doc.notice}</div> : null}

      <main className="panes">
        <SongList
          results={results}
          selected={selected}
          busy={pending === "lyrics"}
          enabledSources={settings.enabledSources}
          onPick={(candidate) => void doFetch(candidate)}
        />
        <LyricPreview doc={doc} busy={pending === "lyrics"} />
      </main>

      <CopyBar
        doc={doc}
        busy={pending === "copy"}
        disabled={!hasDoc}
        onCopy={(formatName) => void doCopy(formatName)}
        onRefresh={() => {
          if (selected) void doFetch(selected, true);
        }}
      />

      <StatusBar
        status={status}
        error={error}
        pending={pending}
        onDismissError={() => setError(null)}
      />

      {settingsOpen ? (
        <SettingsPage
          enabledSources={settings.enabledSources}
          copy={settings.copy}
          theme={settings.theme}
          cacheBytes={cache?.bytes ?? 0}
          cacheFiles={cache?.files ?? 0}
          cacheLocation={cache?.location ?? ""}
          cacheBusy={cacheBusy}
          onToggleSource={toggleSource}
          onToggleCopy={toggleCopy}
          onSelectTranslation={selectTranslation}
          onSelectTheme={selectTheme}
          onClearCache={() => void doClearCache()}
          onClose={() => setSettingsOpen(false)}
        />
      ) : null}

      <Toast toast={toast} />
    </div>
  );
}
