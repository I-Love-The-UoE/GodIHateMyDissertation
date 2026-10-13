import type { ReactNode } from "react";

export type TabItem = { readonly id: string; readonly label: string; readonly dirty?: boolean };

export type TabsProps = {
  readonly items: readonly TabItem[];
  readonly active: string | undefined;
  readonly onSelect: (id: string) => void;
  readonly onClose?: (id: string) => void;
};

export function Tabs(props: TabsProps): ReactNode {
  const { items, active, onSelect, onClose } = props;

  return (
    <div className="tabs" role="tablist">
      {items.map((item) => (
        <div
          key={item.id}
          role="tab"
          tabIndex={0}
          aria-selected={item.id === active}
          className={`tab${item.id === active ? " active" : ""}`}
          onClick={() => onSelect(item.id)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              onSelect(item.id);
            }
          }}
          onAuxClick={(event) => {
            if (event.button === 1) {
              onClose?.(item.id);
            }
          }}
          title={item.id}
        >
          {item.dirty === true ? <span className="dirty">●</span> : null}
          <span>{item.label}</span>
          {onClose === undefined ? null : (
            <button
              type="button"
              className="close"
              tabIndex={-1}
              onClick={(event) => {
                event.stopPropagation();
                onClose(item.id);
              }}
            >
              ×
            </button>
          )}
        </div>
      ))}
    </div>
  );
}
