interface Props {
  status: string;
  error: string | null;
  pending: "search" | "lyrics" | "copy" | null;
  onDismissError: () => void;
}

export function StatusBar({ status, error, pending, onDismissError }: Props) {
  return (
    <div className={error ? "statusbar error" : "statusbar"}>
      <span className={pending ? "status-dot busy" : "status-dot"} />
      <span className="status-text">{error ?? status}</span>
      {pending ? <span className="status-pending">处理中…</span> : null}
      {error ? (
        <button type="button" className="ghost small" onClick={onDismissError}>
          知道了
        </button>
      ) : null}
    </div>
  );
}
