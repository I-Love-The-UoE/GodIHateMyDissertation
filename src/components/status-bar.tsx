import type { ReactNode } from "react";

export type StatusItem = {
  readonly id: string;
  readonly content: ReactNode;
  readonly onClick?: () => void;
  readonly title?: string;
};

export type StatusBarProps = {
  readonly left: readonly StatusItem[];
  readonly right: readonly StatusItem[];
};

function Item(props: { readonly item: StatusItem }): ReactNode {
  const { item } = props;

  if (item.onClick === undefined) {
    return (
      <span className="status-item" title={item.title}>
        {item.content}
      </span>
    );
  }

  return (
    <button
      type="button"
      className="status-item btn btn-ghost btn-small"
      onClick={item.onClick}
      title={item.title}
    >
      {item.content}
    </button>
  );
}

export function StatusBar(props: StatusBarProps): ReactNode {
  const { left, right } = props;

  return (
    <div className="status-bar">
      {left.map((item) => (
        <Item key={item.id} item={item} />
      ))}
      <span className="spacer" />
      {right.map((item) => (
        <Item key={item.id} item={item} />
      ))}
    </div>
  );
}
