import { useEffect, useState } from "react";

export interface ToastState {
  message: string;
  tone: "ok" | "error";
}

interface Props {
  toast: ToastState | null;
}

export function Toast({ toast }: Props) {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (!toast) {
      setVisible(false);
      return;
    }
    setVisible(true);
  }, [toast]);

  if (!toast) return null;

  return (
    <div className={visible ? `toast show ${toast.tone}` : `toast ${toast.tone}`} role="status">
      {toast.message}
    </div>
  );
}
