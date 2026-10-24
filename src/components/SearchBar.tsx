import type { RefObject } from "react";
import type { Source, SourceFilter } from "../types";
import { SOURCE_LABEL } from "../types";

interface Props {
  inputRef: RefObject<HTMLInputElement | null>;
  keyword: string;
  source: SourceFilter;
  /** 设置页里打开的歌词源；关掉的源连 tab 都不显示。 */
  enabledSources: Source[];
  busy: boolean;
  onKeywordChange: (value: string) => void;
  onSourceChange: (value: SourceFilter) => void;
  onSubmit: () => void;
  onTop: boolean;
  onToggleTop: () => void;
  onOpenSettings: () => void;
}

export function SearchBar({
  inputRef,
  keyword,
  source,
  enabledSources,
  busy,
  onKeywordChange,
  onSourceChange,
  onSubmit,
  onTop,
  onToggleTop,
  onOpenSettings,
}: Props) {
  // 只剩一个源时「全部」没有意义，直接省掉这个 tab。
  const tabs: { value: SourceFilter; label: string }[] =
    enabledSources.length > 1
      ? [{ value: "all", label: "全部" }, ...enabledSources.map((s) => ({ value: s, label: SOURCE_LABEL[s] }))]
      : enabledSources.map((s) => ({ value: s, label: SOURCE_LABEL[s] }));

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
        {tabs.map((tab) => (
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

      <button
        type="button"
        className="icon-button"
        title="设置（Ctrl+,）"
        onClick={onOpenSettings}
      >
        设置
      </button>
    </header>
  );
}
