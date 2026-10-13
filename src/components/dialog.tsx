import type { ReactNode } from "react";
import { useEffect } from "react";

export type DialogProps = {
  readonly open: boolean;
  readonly title: string;
  readonly onClose: () => void;
  readonly children: ReactNode;
  readonly footer?: ReactNode;
};

export function Dialog(props: DialogProps): ReactNode {
  const { open, title, onClose, children, footer } = props;

  useEffect(() => {
    if (!open) {
      return;
    }

    const handler = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", handler);

    return () => window.removeEventListener("keydown", handler);
  }, [open, onClose]);

  if (!open) {
    return null;
  }

  return (
    <div className="dialog-backdrop">
      <button
        type="button"
        className="dialog-backdrop-close"
        aria-label="Close"
        onClick={onClose}
      />
      <div className="dialog" role="dialog" aria-label={title}>
        <div className="dialog-header">{title}</div>
        <div className="dialog-body">{children}</div>
        {footer === undefined ? null : <div className="dialog-footer">{footer}</div>}
      </div>
    </div>
  );
}
