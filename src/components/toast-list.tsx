import type { ReactNode } from "react";

export type ToastItem = {
  readonly id: number;
  readonly level: "info" | "warn" | "error";
  readonly text: string;
};

export type ToastListProps = {
  readonly toasts: readonly ToastItem[];
  readonly onDismiss: (id: number) => void;
};

export function ToastList(props: ToastListProps): ReactNode {
  const { toasts, onDismiss } = props;

  return (
    <div className="toasts">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`toast ${toast.level}`}
          onClick={() => onDismiss(toast.id)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              onDismiss(toast.id);
            }
          }}
          role="status"
        >
          {toast.text}
        </div>
      ))}
    </div>
  );
}
