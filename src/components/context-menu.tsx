import type { ReactNode } from "react";
import { useEffect } from "react";

export type MenuEntry = {
  readonly label: string;
  readonly onSelect: () => void;
  readonly danger?: boolean;
};

export type ContextMenuProps = {
  readonly x: number;
  readonly y: number;
  readonly entries: readonly MenuEntry[];
  readonly onClose: () => void;
};

export function ContextMenu(props: ContextMenuProps): ReactNode {
  const { x, y, entries, onClose } = props;

  useEffect(() => {
    const close = (): void => onClose();
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", close);

    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", close);
    };
  }, [onClose]);

  return (
    <div
      className="menu"
      style={{ left: x, top: y }}
      onMouseDown={(event) => event.stopPropagation()}
      role="menu"
    >
      {entries.map((entry) => (
        <button
          key={entry.label}
          type="button"
          role="menuitem"
          style={entry.danger === true ? { color: "var(--err)" } : undefined}
          onClick={() => {
            entry.onSelect();
            onClose();
          }}
        >
          {entry.label}
        </button>
      ))}
    </div>
  );
}
