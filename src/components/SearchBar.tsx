import type { RefObject } from "react";
import type { SourceFilter } from "../types";

const SOURCE_TABS: { value: SourceFilter; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "qq", label: "QQ 音乐" },
  { value: "kugou", label: "酷狗音乐" },
];

interface Props {
  inputRef: RefObject<HTMLInputElement | null>;
  keyword: string;
  source: SourceFilter;
  busy: boolean;
  onKeywordChange: (value: string) => void;
  onSourceChange: (value: SourceFilter) => void;
  onSubmit: () => void;
  onTop: boolean;
  onToggleTop: () => void;
}

export function SearchBar({
  inputRef,
  keyword,
  source,
  busy,
  onKeywordChange,
  onSourceChange,
  onSubmit,
  onTop,
  onToggleTop,
}: Props) {
  return (
    <header className="searchbar">
      <form
        className="search-form"
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit();
        }}
      >
        <input
          ref={inputRef}
          className="search-input"
          type="search"
          autoFocus
          spellCheck={false}
          placeholder="搜歌名或歌手，回车检索（Ctrl+L 聚焦）"
          value={keyword}
          onChange={(event) => onKeywordChange(event.target.value)}
        />
        <button className="primary" type="submit" disabled={busy}>
          {busy ? "搜索中…" : "搜索"}
        </button>
      </form>

      <div className="source-tabs" role="tablist" aria-label="音乐来源">
        {SOURCE_TABS.map((tab) => (
          <button
            key={tab.value}
            type="button"
            role="tab"
            aria-selected={source === tab.value}
            className={source === tab.value ? "tab active" : "tab"}
            onClick={() => onSourceChange(tab.value)}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <button
        type="button"
        className={onTop ? "icon-button active" : "icon-button"}
        title={onTop ? "取消窗口置顶" : "窗口置顶"}
        aria-pressed={onTop}
        onClick={onToggleTop}
      >
        置顶
      </button>
    </header>
  );
}
