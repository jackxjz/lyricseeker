import type { CopyFormat, LyricsDoc } from "../types";
import { FORMAT_LABEL, FORMAT_ORDER } from "../types";

interface Props {
  doc: LyricsDoc | null;
  busy: boolean;
  disabled: boolean;
  onCopy: (formatName: CopyFormat) => void;
  onRefresh: () => void;
}

/** hint 展示时间标签的形态，让人一眼看出两种逐字的区别。 */
const FORMAT_HINTS: Record<CopyFormat, string> = {
  plain: "[行]字[字]字…[行尾]",
  enhanced: "[行]‹字›字…‹行尾›",
  lrc: "标准 LRC，一行一条",
  text: "不带任何时间标记",
};

export function CopyBar({ doc, busy, disabled, onCopy, onRefresh }: Props) {
  return (
    <footer className="copybar">
      <div className="copy-buttons">
        {FORMAT_ORDER.map((format, index) => (
          <button
            key={format}
            type="button"
            className="copy-button"
            disabled={disabled || busy}
            title={`${FORMAT_HINTS[format]} · Ctrl+${index + 1}`}
            onClick={() => onCopy(format)}
          >
            <span className="copy-label">{FORMAT_LABEL[format]}</span>
            <span className="copy-shortcut">Ctrl+{index + 1}</span>
          </button>
        ))}
      </div>

      <div className="copy-side">
        <button
          type="button"
          className="ghost"
          title="忽略缓存，重新向上游请求"
          disabled={!doc || busy}
          onClick={onRefresh}
        >
          重新获取
        </button>
      </div>
    </footer>
  );
}
