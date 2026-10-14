import type { ReactNode } from "react";
import { useState } from "react";
import { useServices, useStore } from "@/app/services";
import { Button } from "@/components/button";
import { Dialog } from "@/components/dialog";
import { spawn } from "@/lib/async";
import { pickDirectory } from "@/lib/tauri";
import { SetupWarnings } from "./setup-warnings";

function CloneDialog(props: { readonly open: boolean; readonly onClose: () => void }): ReactNode {
  const { open, onClose } = props;
  const [url, setUrl] = useState("");
  const [dest, setDest] = useState("");
  const { backend } = useServices();
  const openProject = useStore((state) => state.openProject);
  const toast = useStore((state) => state.toast);
  const gitBusy = useStore((state) => state.gitBusy);

  const clone = async (): Promise<void> => {
    const result = await backend.gitClone(url.trim(), dest.trim());

    if (!result.ok) {
      toast("error", `Clone failed: ${result.error.kind}`);

      return;
    }

    onClose();
    await openProject(result.value);
  };

  return (
    <Dialog
      open={open}
      title="Clone a repository"
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button
            variant="primary"
            disabled={url.trim() === "" || dest.trim() === "" || gitBusy !== undefined}
            onClick={() => spawn(clone())}
          >
            Clone
          </Button>
        </>
      }
    >
      <div className="field">
        <label htmlFor="clone-url">Repository URL (SSH or HTTPS)</label>
        <input
          id="clone-url"
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          placeholder="git@github.com:you/thesis.git"
        />
      </div>
      <div className="field">
        <label htmlFor="clone-dest">Destination folder</label>
        <div style={{ display: "flex", gap: 8 }}>
          <input
            id="clone-dest"
            style={{ flex: 1 }}
            value={dest}
            onChange={(event) => setDest(event.target.value)}
          />
          <Button
            onClick={() =>
              spawn(
                pickDirectory("Choose the parent folder").then((picked) => {
                  if (picked !== undefined) {
                    const name =
                      url
                        .split("/")
                        .at(-1)
                        ?.replace(/\.git$/, "") ?? "repository";
                    setDest(`${picked}/${name}`);
                  }
                }),
              )
            }
          >
            Browse
          </Button>
        </div>
      </div>
      <p className="hint">
        HTTPS remotes prompt for a token once and store it in the system keychain.
      </p>
    </Dialog>
  );
}

export function StartScreen(): ReactNode {
  const recent = useStore((state) => state.recent);
  const openProject = useStore((state) => state.openProject);
  const createProject = useStore((state) => state.createProject);
  const forgetProject = useStore((state) => state.forgetProject);
  const setSettingsOpen = useStore((state) => state.setSettingsOpen);
  const [cloneOpen, setCloneOpen] = useState(false);

  const pickAndOpen = async (): Promise<void> => {
    const root = await pickDirectory("Open a LaTeX project folder");

    if (root !== undefined) {
      await openProject(root);
    }
  };

  const pickAndCreate = async (): Promise<void> => {
    const root = await pickDirectory("Choose an empty folder for the new project");

    if (root !== undefined) {
      await createProject(root);
    }
  };

  return (
    <div className="start">
      <h1>
        God<span>I</span>Hate<span>My</span>Dissertation
      </h1>
      <SetupWarnings />
      <div className="actions">
        <Button variant="primary" onClick={() => spawn(pickAndOpen())}>
          Open folder
        </Button>
        <Button onClick={() => spawn(pickAndCreate())}>New project</Button>
        <Button onClick={() => setCloneOpen(true)}>Clone repository</Button>
        <Button variant="ghost" onClick={() => setSettingsOpen(true)}>
          Settings
        </Button>
      </div>
      {recent.length === 0 ? (
        <p className="hint">Recent projects appear here.</p>
      ) : (
        <div className="recent">
          {recent.map((record) => (
            <div key={record.id} className="recent-row">
              <button
                type="button"
                className="recent-open"
                onClick={() => spawn(openProject(record.root))}
              >
                <span>📁</span>
                <span>{record.root.split("/").at(-1)}</span>
                <span className="path">{record.root}</span>
              </button>
              <Button
                small
                variant="ghost"
                title="Remove from recent"
                onClick={() => spawn(forgetProject(record.root))}
              >
                ×
              </Button>
            </div>
          ))}
        </div>
      )}
      <CloneDialog open={cloneOpen} onClose={() => setCloneOpen(false)} />
    </div>
  );
}
