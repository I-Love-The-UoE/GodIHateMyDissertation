import type { ReactNode } from "react";

export type PanelProps = {
  readonly title: ReactNode;
  readonly actions?: ReactNode;
  readonly children: ReactNode;
};

export function Panel(props: PanelProps): ReactNode {
  const { title, actions, children } = props;

  return (
    <div className="panel">
      <div className="panel-header">
        <span>{title}</span>
        <span className="spacer" />
        {actions}
      </div>
      <div className="panel-body">{children}</div>
    </div>
  );
}
