import type { ReactNode } from "react";
import { useState } from "react";
import { useStore } from "@/app/services";
import { Button } from "@/components/button";
import { ContextMenu } from "@/components/context-menu";
import { Dialog } from "@/components/dialog";
import { Panel } from "@/components/panel";
import { Tree } from "@/components/tree";
import { spawn } from "@/lib/async";
import type { FileNode } from "@/lib/tauri";

type Menu = { readonly node: FileNode; readonly x: number; readonly y: number };
type Prompt =
  | { readonly kind: "new_file"; readonly dir: string | null }
  | { readonly kind: "new_dir"; readonly dir: string | null }
  | { readonly kind: "rename"; readonly path: string }
  | { readonly kind: "delete"; readonly path: string };

function dirOf(node: FileNode): string | null {
  if (node.kind === "dir") {
    return node.path;
  }

  const parent = node.path?.split("/").slice(0, -1).join("/") ?? "";

  return parent === "" ? null : parent;
}

function joinPath(dir: string | null, name: string): string {
  return dir === null ? name : `${dir}/${name}`;
}

export function FilePanel(): ReactNode {
  const project = useStore((state) => state.project);
  const activeTab = useStore((state) => state.activeTab);
  const openTab = useStore((state) => state.openTab);
  const createFile = useStore((state) => state.createFile);
  const createDir = useStore((state) => state.createDir);
  const renamePath = useStore((state) => state.renamePath);
  const deletePath = useStore((state) => state.deletePath);
  const movePath = useStore((state) => state.movePath);
  const setMainFile = useStore((state) => state.setMainFile);
  const refreshTree = useStore((state) => state.refreshTree);
  const [menu, setMenu] = useState<Menu | undefined>(undefined);
  const [prompt, setPrompt] = useState<Prompt | undefined>(undefined);
  const [value, setValue] = useState("");

  if (project === undefined) {
    return null;
  }

  const submit = async (): Promise<void> => {
    if (prompt === undefined) {
      return;
    }

    const name = value.trim();

    switch (prompt.kind) {
      case "new_file":
        if (name !== "") {
          const path = joinPath(prompt.dir, name);

          if (await createFile(path)) {
            openTab(path);
          }
        }

        break;
      case "new_dir":
        if (name !== "") {
          await createDir(joinPath(prompt.dir, name));
        }

        break;
      case "rename":
        if (name !== "") {
          const parent = prompt.path.split("/").slice(0, -1).join("/");
          await renamePath(prompt.path, parent === "" ? name : `${parent}/${name}`);
        }

        break;
      case "delete":
        await deletePath(prompt.path);

        break;
      default:
        return prompt satisfies never;
    }

    setPrompt(undefined);
  };

  const entriesFor = (
    node: FileNode,
  ): readonly {
    readonly label: string;
    readonly onSelect: () => void;
    readonly danger?: boolean;
  }[] => {
    const dir = dirOf(node);
    const common = [
      {
        label: "New file",
        onSelect: () => {
          setValue("");
          setPrompt({ kind: "new_file", dir });
        },
      },
      {
        label: "New folder",
        onSelect: () => {
          setValue("");
          setPrompt({ kind: "new_dir", dir });
        },
      },
    ];

    if (node.path === null) {
      return common;
    }

    const path = node.path;
    const own = [
      {
        label: "Rename",
        onSelect: () => {
          setValue(node.name);
          setPrompt({ kind: "rename", path });
        },
      },
      { label: "Delete", danger: true, onSelect: () => setPrompt({ kind: "delete", path }) },
    ];

    if (node.kind === "file" && node.name.endsWith(".tex")) {
      own.unshift({ label: "Set as main file", onSelect: () => spawn(setMainFile(path)) });
    }

    return [...common, ...own];
  };

  const title =
    prompt === undefined
      ? ""
      : {
          new_file: "New file",
          new_dir: "New folder",
          rename: "Rename",
          delete: "Delete",
        }[prompt.kind];

  return (
    <Panel
      title={project.record.root.split("/").at(-1) ?? "Project"}
      actions={
        <>
          <Button
            small
            variant="ghost"
            title="New file"
            onClick={() => {
              setValue("");
              setPrompt({ kind: "new_file", dir: null });
            }}
          >
            +
          </Button>
          <Button small variant="ghost" title="Refresh" onClick={() => spawn(refreshTree())}>
            ↻
          </Button>
        </>
      }
    >
      <Tree
        root={project.tree}
        selected={activeTab}
        mainFile={project.record.mainFile}
        onOpen={openTab}
        onContextMenu={(node, x, y) => setMenu({ node, x, y })}
        onMove={(from, toDir) => spawn(movePath(from, toDir).then(() => undefined))}
      />
      {menu === undefined ? null : (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          entries={entriesFor(menu.node)}
          onClose={() => setMenu(undefined)}
        />
      )}
      <Dialog
        open={prompt !== undefined}
        title={title}
        onClose={() => setPrompt(undefined)}
        footer={
          <>
            <Button onClick={() => setPrompt(undefined)}>Cancel</Button>
            <Button
              variant={prompt?.kind === "delete" ? "danger" : "primary"}
              onClick={() => spawn(submit())}
            >
              {prompt?.kind === "delete" ? "Delete" : "OK"}
            </Button>
          </>
        }
      >
        {prompt?.kind === "delete" ? (
          <p>Delete {prompt.path}? This cannot be undone from the app.</p>
        ) : (
          <div className="field">
            <label htmlFor="file-prompt">Name</label>
            <input
              id="file-prompt"
              value={value}
              onChange={(event) => setValue(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  spawn(submit());
                }
              }}
            />
          </div>
        )}
      </Dialog>
    </Panel>
  );
}
