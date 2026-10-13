import type { ReactNode } from "react";
import { useState } from "react";
import type { FileNode } from "@/lib/tauri";

export type TreeProps = {
  readonly root: FileNode;
  readonly selected: string | undefined;
  readonly mainFile: string | null;
  readonly onOpen: (path: string) => void;
  readonly onContextMenu: (node: FileNode, x: number, y: number) => void;
  readonly onMove: (from: string, toDir: string | null) => void;
};

type RowProps = {
  readonly node: FileNode;
  readonly depth: number;
  readonly props: TreeProps;
  readonly expanded: ReadonlySet<string>;
  readonly toggle: (path: string) => void;
};

function Row(rowProps: RowProps): ReactNode {
  const { node, depth, props, expanded, toggle } = rowProps;
  const [dropTarget, setDropTarget] = useState(false);
  const path = node.path ?? "";
  const isDir = node.kind === "dir";
  const open = node.path === null || expanded.has(path);
  const classes = [
    "tree-row",
    props.selected === path ? "selected" : "",
    props.mainFile === path ? "main-file" : "",
    dropTarget ? "drop-target" : "",
  ].join(" ");

  return (
    <>
      <div
        className={classes}
        style={{ paddingLeft: 8 + depth * 14 }}
        draggable={node.path !== null}
        onDragStart={(event) => {
          event.dataTransfer.setData("text/gihmd-path", path);
        }}
        onDragOver={(event) => {
          if (isDir) {
            event.preventDefault();
            setDropTarget(true);
          }
        }}
        onDragLeave={() => setDropTarget(false)}
        onDrop={(event) => {
          setDropTarget(false);
          const from = event.dataTransfer.getData("text/gihmd-path");

          if (isDir && from !== "" && from !== path && !path.startsWith(`${from}/`)) {
            event.preventDefault();
            props.onMove(from, node.path);
          }
        }}
        onClick={() => {
          if (isDir) {
            toggle(path);
          } else {
            props.onOpen(path);
          }
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            if (isDir) {
              toggle(path);
            } else {
              props.onOpen(path);
            }
          }
        }}
        onContextMenu={(event) => {
          event.preventDefault();
          props.onContextMenu(node, event.clientX, event.clientY);
        }}
        role="treeitem"
        aria-selected={props.selected === path}
        aria-expanded={isDir ? open : undefined}
        tabIndex={0}
      >
        <span className="chevron">{isDir ? (open ? "▾" : "▸") : ""}</span>
        <span>{isDir ? "📁" : iconFor(node.name)}</span>
        <span>{node.name}</span>
      </div>
      {isDir && open
        ? node.children.map((child) => (
            <Row
              key={child.path ?? child.name}
              node={child}
              depth={depth + 1}
              props={props}
              expanded={expanded}
              toggle={toggle}
            />
          ))
        : null}
    </>
  );
}

function iconFor(name: string): string {
  if (name.endsWith(".tex")) {
    return "📄";
  }

  if (name.endsWith(".bib")) {
    return "📚";
  }

  if (/\.(png|jpe?g|pdf|svg|eps)$/i.test(name)) {
    return "🖼";
  }

  return "▫";
}

export function Tree(props: TreeProps): ReactNode {
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const toggle = (path: string): void => {
    setExpanded((current) => {
      const next = new Set(current);

      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }

      return next;
    });
  };

  return (
    <div
      className="tree"
      role="tree"
      onDragOver={(event) => event.preventDefault()}
      onDrop={(event) => {
        const from = event.dataTransfer.getData("text/gihmd-path");

        if (from !== "" && from.includes("/")) {
          event.preventDefault();
          props.onMove(from, null);
        }
      }}
      onContextMenu={(event) => {
        if (event.target === event.currentTarget) {
          event.preventDefault();
          props.onContextMenu(props.root, event.clientX, event.clientY);
        }
      }}
    >
      {props.root.children.map((child) => (
        <Row
          key={child.path ?? child.name}
          node={child}
          depth={0}
          props={props}
          expanded={expanded}
          toggle={toggle}
        />
      ))}
    </div>
  );
}
