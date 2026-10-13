import type { ReactNode } from "react";
import { Group, Panel, Separator } from "react-resizable-panels";

export type SplitPane = {
  readonly id: string;
  readonly content: ReactNode;
  readonly defaultSize: number;
  readonly minSize?: number;
};

export type SplitProps = {
  readonly id: string;
  readonly direction: "horizontal" | "vertical";
  readonly panes: readonly SplitPane[];
};

export function Split(props: SplitProps): ReactNode {
  const { id, direction, panes } = props;

  return (
    <Group id={id} orientation={direction} style={{ height: "100%", width: "100%" }}>
      {panes.map((pane, index) => (
        <PaneWithSeparator key={pane.id} pane={pane} first={index === 0} direction={direction} />
      ))}
    </Group>
  );
}

function PaneWithSeparator(props: {
  readonly pane: SplitPane;
  readonly first: boolean;
  readonly direction: "horizontal" | "vertical";
}): ReactNode {
  const { pane, first, direction } = props;

  return (
    <>
      {first ? null : <Separator className={`separator ${direction}`} />}
      <Panel
        id={pane.id}
        defaultSize={pane.defaultSize}
        minSize={pane.minSize ?? 10}
        style={{ minHeight: 0, minWidth: 0 }}
      >
        {pane.content}
      </Panel>
    </>
  );
}
