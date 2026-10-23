import { useEffect, useMemo, useRef, useState } from "react";
import type { LyricsDoc } from "../types";

interface Props {
  doc: LyricsDoc | null;
  busy: boolean;
}

/** 把毫秒格式化成 mm:ss.mmm，用于行首时间轴。 */
function stamp(ms: number): string {
  const safe = Math.max(0, ms);
  const minute = Math.floor(safe / 60000);
  const second = Math.floor((safe % 60000) / 1000);
  const milli = safe % 1000;
  return `${String(minute).padStart(2, "0")}:${String(second).padStart(2, "0")}.${String(
    milli,
  ).padStart(3, "0")}`;
}

export function LyricPreview({ doc, busy }: Props) {
  const [playing, setPlaying] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [follow, setFollow] = useState(true);
  const frame = useRef<number | null>(null);
  const startedAt = useRef(0);

  const lastEnd = useMemo(() => {
    if (!doc) return 0;
    return doc.lines.reduce((max, line) => {
      const end = line.startMs + (line.durMs ?? 0);
      return Math.max(max, end, line.startMs);
    }, 0);
  }, [doc]);

  // 停止播放：切歌或卸载时复位。
  useEffect(() => {
    setPlaying(false);
    setElapsed(0);
    if (frame.current !== null) {
      cancelAnimationFrame(frame.current);
      frame.current = null;
    }
  }, [doc]);

  useEffect(() => {
    if (!playing) {
      if (frame.current !== null) {
        cancelAnimationFrame(frame.current);
        frame.current = null;
      }
      return;
    }

    startedAt.current = performance.now() - elapsed;
    const tick = () => {
      const next = performance.now() - startedAt.current;
      if (next >= lastEnd) {
        setElapsed(lastEnd);
        setPlaying(false);
        return;
      }
      setElapsed(next);
      frame.current = requestAnimationFrame(tick);
    };
    frame.current = requestAnimationFrame(tick);

    return () => {
      if (frame.current !== null) cancelAnimationFrame(frame.current);
    };
    // elapsed 只用于起播时定位，故不进依赖。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [playing, lastEnd]);

  const activeIndex = useMemo(() => {
    if (!doc || doc.lines.length === 0) return -1;
    let index = -1;
    for (let i = 0; i < doc.lines.length; i += 1) {
      if (doc.lines[i].startMs <= elapsed) index = i;
      else break;
    }
    return index;
  }, [doc, elapsed]);

  const activeRef = useRef<HTMLLIElement | null>(null);
  useEffect(() => {
    if (!playing || !follow) return;
    activeRef.current?.scrollIntoView({ block: "center", behavior: "smooth" });
  }, [activeIndex, playing, follow]);

  if (busy) {
    return (
      <section className="pane preview-pane">
        <div className="empty">
          <p className="empty-title">正在获取歌词…</p>
        </div>
      </section>
    );
  }

  if (!doc) {
    return (
      <section className="pane preview-pane">
        <div className="empty">
          <p className="empty-title">选择左侧任意一首</p>
          <p className="empty-hint">
            逐字歌词会以「逐字时间戳 + 单字」的形式展示；没有逐字资源的曲目会自动降级为逐行。
          </p>
        </div>
      </section>
    );
  }

  return (
    <section className="pane preview-pane">
      <div className="preview-head">
        <div className="preview-title">
          <span className="preview-song">{doc.song.title}</span>
          <span className="preview-sub">
            {doc.song.artists.join(" / ") || "未知歌手"}
            {doc.song.album ? ` · ${doc.song.album}` : ""}
          </span>
        </div>
        <div className="preview-tools">
          <span className={doc.wordSynced ? "badge ok" : "badge"}>
            {doc.wordSynced ? "逐字" : "逐行"}
          </span>
          {doc.fromCache ? <span className="badge">缓存</span> : null}
          <button
            type="button"
            className="ghost"
            disabled={!doc.wordSynced}
            title={doc.wordSynced ? "按时间轴走字预览" : "该歌词没有逐字时间信息"}
            onClick={() => {
              if (!playing && elapsed >= lastEnd) setElapsed(0);
              setPlaying(!playing);
            }}
          >
            {playing ? "暂停" : "预览"}
          </button>
          <label className="toggle">
            <input
              type="checkbox"
              checked={follow}
              onChange={(event) => setFollow(event.target.checked)}
            />
            跟随
          </label>
          <span className="clock">{stamp(Math.round(elapsed))}</span>
        </div>
      </div>

      <ol className="lyric-lines">
        {doc.lines.map((line, index) => {
          const isActive = index === activeIndex;
          return (
            <li
              key={`${line.startMs}-${index}`}
              ref={isActive ? activeRef : undefined}
              className={isActive ? "lyric-line active" : "lyric-line"}
            >
              <span className="line-time">{stamp(line.startMs)}</span>
              <span className="line-body">
                {line.syllables && line.syllables.length > 0
                  ? line.syllables.map((syllable, syllableIndex) => {
                      const sung = line.startMs + syllable.startMs >= elapsed;
                      const passed = elapsed >= syllable.startMs;
                      return (
                        <span
                          key={`${syllable.startMs}-${syllableIndex}`}
                          className={
                            playing ? (passed ? "syl passed" : "syl") : sung ? "syl" : "syl"
                          }
                          title={`${stamp(syllable.startMs)} · ${syllable.durMs}ms`}
                        >
                          {syllable.text}
                        </span>
                      );
                    })
                  : line.text}
              </span>
              {line.translation ? (
                <span className="line-translation">{line.translation}</span>
              ) : null}
            </li>
          );
        })}
      </ol>
    </section>
  );
}
