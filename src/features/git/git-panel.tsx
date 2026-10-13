import type { ReactNode } from "react";
import { useState } from "react";
import { useServices, useStore } from "@/app/services";
import { Button } from "@/components/button";
import { Dialog } from "@/components/dialog";
import { spawn } from "@/lib/async";
import type { Change, StatusEntry } from "@/lib/tauri";
import { DiffView } from "./diff-view";

const CHANGE_LETTER: Readonly<Record<Change, string>> = {
  added: "A",
  modified: "M",
  deleted: "D",
  renamed: "R",
  type_change: "T",
  untracked: "U",
  conflicted: "C",
};

function ChangeRow(props: {
  readonly entry: StatusEntry;
  readonly change: Change;
  readonly staged: boolean;
}): ReactNode {
  const { entry, change, staged } = props;
  const stage = useStore((state) => state.stage);
  const unstage = useStore((state) => state.unstage);
  const discard = useStore((state) => state.discard);
  const showDiff = useStore((state) => state.showDiff);
  const openTab = useStore((state) => state.openTab);

  return (
    <div className="git-row">
      <span className={`change change-${change}`}>{CHANGE_LETTER[change]}</span>
      <button
        type="button"
        className="path"
        onClick={() => spawn(showDiff(entry.path, { kind: staged ? "staged" : "unstaged" }))}
        onDoubleClick={() => openTab(entry.path)}
        title={entry.path}
      >
        {entry.path}
      </button>
      {staged ? (
        <Button small variant="ghost" title="Unstage" onClick={() => spawn(unstage([entry.path]))}>
          −
        </Button>
      ) : (
        <>
          <Button
            small
            variant="ghost"
            title="Discard changes"
            onClick={() => spawn(discard([entry.path]))}
          >
            ↶
          </Button>
          <Button small variant="ghost" title="Stage" onClick={() => spawn(stage([entry.path]))}>
            +
          </Button>
        </>
      )}
    </div>
  );
}

function RemoteDialog(props: { readonly open: boolean; readonly onClose: () => void }): ReactNode {
  const { open, onClose } = props;
  const remotes = useStore((state) => state.remotes);
  const addRemote = useStore((state) => state.addRemote);
  const removeRemote = useStore((state) => state.removeRemote);
  const [name, setName] = useState("origin");
  const [url, setUrl] = useState("");

  return (
    <Dialog
      open={open}
      title="Remotes"
      onClose={onClose}
      footer={<Button onClick={onClose}>Close</Button>}
    >
      {remotes.map((remote) => (
        <div key={remote.name} className="git-row">
          <span style={{ fontWeight: 600 }}>{remote.name}</span>
          <span className="path">{remote.url}</span>
          <Button
            small
            variant="ghost"
            title="Remove"
            onClick={() => spawn(removeRemote(remote.name))}
          >
            ×
          </Button>
        </div>
      ))}
      <div className="field">
        <label htmlFor="remote-name">Name</label>
        <input id="remote-name" value={name} onChange={(event) => setName(event.target.value)} />
      </div>
      <div className="field">
        <label htmlFor="remote-url">URL</label>
        <input
          id="remote-url"
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          placeholder="git@github.com:you/thesis.git"
        />
      </div>
      <Button
        variant="primary"
        disabled={name.trim() === "" || url.trim() === ""}
        onClick={() => spawn(addRemote(name.trim(), url.trim()).then(() => setUrl("")))}
      >
        Add remote
      </Button>
    </Dialog>
  );
}

function BranchDialog(props: { readonly open: boolean; readonly onClose: () => void }): ReactNode {
  const { open, onClose } = props;
  const branches = useStore((state) => state.branches);
  const createBranch = useStore((state) => state.createBranch);
  const checkoutBranch = useStore((state) => state.checkoutBranch);
  const [name, setName] = useState("");

  return (
    <Dialog
      open={open}
      title="Branches"
      onClose={onClose}
      footer={<Button onClick={onClose}>Close</Button>}
    >
      {branches
        .filter((branch) => !branch.remote)
        .map((branch) => (
          <div key={branch.name} className="git-row">
            <span style={{ fontWeight: branch.isHead ? 700 : 400 }}>
              {branch.isHead ? "● " : "○ "}
              {branch.name}
            </span>
            <span className="path hint">{branch.upstream ?? ""}</span>
            {branch.isHead ? null : (
              <Button small onClick={() => spawn(checkoutBranch(branch.name).then(onClose))}>
                Checkout
              </Button>
            )}
          </div>
        ))}
      <div className="field">
        <label htmlFor="branch-name">New branch from HEAD</label>
        <div style={{ display: "flex", gap: 8 }}>
          <input
            id="branch-name"
            style={{ flex: 1 }}
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
          <Button
            variant="primary"
            disabled={name.trim() === ""}
            onClick={() => spawn(createBranch(name.trim(), true).then(onClose))}
          >
            Create and switch
          </Button>
        </div>
      </div>
    </Dialog>
  );
}

export function GitPanel(): ReactNode {
  const { backend } = useServices();
  const repo = useStore((state) => state.repo);
  const remotes = useStore((state) => state.remotes);
  const history = useStore((state) => state.history);
  const diff = useStore((state) => state.diff);
  const gitBusy = useStore((state) => state.gitBusy);
  const gitProgress = useStore((state) => state.gitProgress);
  const gitInit = useStore((state) => state.gitInit);
  const stage = useStore((state) => state.stage);
  const commit = useStore((state) => state.commit);
  const fetch = useStore((state) => state.fetch);
  const pull = useStore((state) => state.pull);
  const push = useStore((state) => state.push);
  const cancelGit = useStore((state) => state.cancelGit);
  const clearDiff = useStore((state) => state.clearDiff);
  const showDiff = useStore((state) => state.showDiff);
  const [message, setMessage] = useState("");
  const [remotesOpen, setRemotesOpen] = useState(false);
  const [branchesOpen, setBranchesOpen] = useState(false);

  if (repo === undefined) {
    return <div className="empty">Loading git status…</div>;
  }

  if (repo === null) {
    return (
      <div className="empty" style={{ flexDirection: "column", gap: 12 }}>
        <span>This project is not a git repository.</span>
        <Button variant="primary" onClick={() => spawn(gitInit())}>
          Initialise repository
        </Button>
      </div>
    );
  }

  if (diff !== undefined) {
    return (
      <div className="panel">
        <div className="panel-header" style={{ fontWeight: 400 }}>
          <span>
            {diff.path}{" "}
            <span className="hint">
              ({diff.oldLabel} → {diff.newLabel}, +{diff.additions} −{diff.deletions})
            </span>
          </span>
          <span className="spacer" />
          <Button small onClick={clearDiff}>
            Back
          </Button>
        </div>
        <div className="panel-body">
          <DiffView diff={diff} />
        </div>
      </div>
    );
  }

  const staged = repo.entries.filter((entry) => entry.staged !== null);
  const unstaged = repo.entries.filter((entry) => entry.unstaged !== null);
  const remote = remotes[0]?.name;
  const progressText =
    gitProgress === undefined
      ? ""
      : gitProgress.progress.kind === "sideband"
        ? gitProgress.progress.text
        : gitProgress.progress.kind === "receiving"
          ? `Receiving ${gitProgress.progress.received}/${gitProgress.progress.total}`
          : gitProgress.progress.kind === "resolving"
            ? `Resolving ${gitProgress.progress.current}/${gitProgress.progress.total}`
            : gitProgress.progress.kind === "pushing"
              ? `Pushing ${gitProgress.progress.current}/${gitProgress.progress.total}`
              : `Counting ${gitProgress.progress.received}/${gitProgress.progress.total}`;

  return (
    <div className="panel">
      <div className="panel-header" style={{ fontWeight: 400 }}>
        <Button small onClick={() => setBranchesOpen(true)} title="Branches">
          ⎇ {repo.branch ?? "detached"}
        </Button>
        {repo.upstream === null ? null : (
          <span className="hint">
            ↑{repo.ahead} ↓{repo.behind}
          </span>
        )}
        <span className="spacer" />
        {gitBusy === undefined ? (
          <>
            <Button
              small
              disabled={remote === undefined}
              onClick={() => remote !== undefined && spawn(fetch(remote))}
            >
              Fetch
            </Button>
            <Button
              small
              disabled={remote === undefined}
              onClick={() => remote !== undefined && spawn(pull(remote))}
            >
              Pull
            </Button>
            <Button
              small
              disabled={remote === undefined}
              onClick={() => remote !== undefined && spawn(push(remote))}
            >
              Push
            </Button>
            <Button small variant="ghost" onClick={() => setRemotesOpen(true)}>
              Remotes
            </Button>
          </>
        ) : (
          <>
            <span className="hint">
              {gitBusy}… {progressText}
            </span>
            <Button small variant="danger" onClick={() => spawn(cancelGit())}>
              Cancel
            </Button>
          </>
        )}
      </div>
      <div className="panel-body">
        <div className="git-section">
          <h4>Commit</h4>
          <textarea
            rows={3}
            style={{ width: "100%", resize: "vertical" }}
            placeholder="Commit message"
            value={message}
            onChange={(event) => setMessage(event.target.value)}
          />
          <div style={{ display: "flex", gap: 8, marginTop: 6 }}>
            <Button
              small
              onClick={() => spawn(stage(unstaged.map((entry) => entry.path)))}
              disabled={unstaged.length === 0}
            >
              Stage all
            </Button>
            <span className="spacer" style={{ flex: 1 }} />
            <Button
              small
              variant="primary"
              disabled={message.trim() === "" || staged.length === 0}
              onClick={() =>
                spawn(
                  commit(message.trim()).then((done) => {
                    if (done) {
                      setMessage("");
                    }
                  }),
                )
              }
            >
              Commit {staged.length > 0 ? `(${staged.length})` : ""}
            </Button>
          </div>
        </div>
        <div className="git-section">
          <h4>Staged ({staged.length})</h4>
          {staged.map((entry) => (
            <ChangeRow
              key={`s-${entry.path}`}
              entry={entry}
              change={entry.staged ?? "modified"}
              staged
            />
          ))}
        </div>
        <div className="git-section">
          <h4>Changes ({unstaged.length})</h4>
          {unstaged.map((entry) => (
            <ChangeRow
              key={`u-${entry.path}`}
              entry={entry}
              change={entry.unstaged ?? "modified"}
              staged={false}
            />
          ))}
        </div>
        <div className="git-section">
          <h4>History</h4>
          {history.map((item) => (
            <button
              type="button"
              key={item.oid}
              className="commit-row"
              onClick={() => spawn(showDiffForCommit(item.oid))}
            >
              <div>{item.summary}</div>
              <div className="meta">
                {item.shortOid} · {item.authorName} ·{" "}
                {new Date(item.timeUnix * 1000).toLocaleString()}
              </div>
            </button>
          ))}
        </div>
      </div>
      <RemoteDialog open={remotesOpen} onClose={() => setRemotesOpen(false)} />
      <BranchDialog open={branchesOpen} onClose={() => setBranchesOpen(false)} />
    </div>
  );

  async function showDiffForCommit(oid: string): Promise<void> {
    const files = await backend.gitCommitFiles(oid);
    const first = files.ok ? files.value[0] : undefined;

    if (first !== undefined) {
      await showDiff(first.path, { kind: "commit", oid });
    }
  }
}
