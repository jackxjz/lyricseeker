import type { SongCandidate } from "../types";
import { formatDuration, SOURCE_LABEL } from "../types";

interface Props {
  results: SongCandidate[];
  selected: SongCandidate | null;
  busy: boolean;
  onPick: (candidate: SongCandidate) => void;
}

export function SongList({ results, selected, busy, onPick }: Props) {
  if (results.length === 0) {
    return (
      <section className="pane list-pane">
        <div className="empty">
          <p className="empty-title">还没有搜索结果</p>
          <p className="empty-hint">
            输入歌名或歌手后回车。QQ 音乐与酷狗同时检索，选中条目即可看到逐字歌词。
          </p>
        </div>
      </section>
    );
  }

  return (
    <section className="pane list-pane">
      <ul className="song-list">
        {results.map((candidate) => {
          const active =
            selected?.source === candidate.source && selected?.songId === candidate.songId;
          return (
            <li key={`${candidate.source}-${candidate.songId}`}>
              <button
                type="button"
                className={active ? "song-item active" : "song-item"}
                disabled={busy}
                onClick={() => onPick(candidate)}
              >
                <span className="song-title" title={candidate.title}>
                  {candidate.title}
                </span>
                <span className="song-meta">
                  <span className="song-artist" title={candidate.artists.join(", ")}>
                    {candidate.artists.join(" / ") || "未知歌手"}
                  </span>
                  <span className="dot">·</span>
                  <span className="song-duration">{formatDuration(candidate.durationMs)}</span>
                  <span className="song-source">{SOURCE_LABEL[candidate.source]}</span>
                </span>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
