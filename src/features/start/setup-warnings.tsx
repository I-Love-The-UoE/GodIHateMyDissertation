import type { ReactNode } from "react";
import { useStore } from "@/app/services";
import { Button } from "@/components/button";
import { spawn } from "@/lib/async";
import type { TexInstallEvent } from "@/lib/tauri";
import { browse } from "@/lib/tauri";

const CLAUDE_INSTALL_URL = "https://code.claude.com/docs/en/quickstart";
const PERL_URL = "https://www.perl.org/get.html";

function megabytes(bytes: number): string {
  return `${Math.round(bytes / 1048576)} MB`;
}

function describeInstall(event: TexInstallEvent): string {
  switch (event.phase) {
    case "downloading":
      return event.total === null
        ? `Downloading the installer… ${megabytes(event.received)}`
        : `Downloading the installer… ${megabytes(event.received)} of ${megabytes(event.total)}`;
    case "extracting":
      return "Unpacking the installer…";
    case "installing":
      return `Installing TeX Live… ${event.line}`;
    case "packages":
      return `Installing packages: ${event.names.join(", ")}`;
    case "package_line":
      return `Installing packages… ${event.line}`;
    case "packages_installed":
      return `Installed ${event.names.join(", ")}`;
    case "finished":
      return "TeX Live is ready.";
    case "failed":
      return `Install failed: ${event.error}`;
    default:
      return event satisfies never;
  }
}

function Warning(props: {
  readonly title: string;
  readonly body: string;
  readonly action?: ReactNode;
}): ReactNode {
  const { title, body, action } = props;

  return (
    <div className="setup-warning">
      <div className="setup-warning-text">
        <strong>{title}</strong>
        <span className="hint">{body}</span>
      </div>
      {action}
    </div>
  );
}

export function SetupWarnings(): ReactNode {
  const texProbe = useStore((state) => state.texProbe);
  const managedTex = useStore((state) => state.managedTex);
  const texInstall = useStore((state) => state.texInstall);
  const claudeProbe = useStore((state) => state.claudeProbe);
  const gitIdentity = useStore((state) => state.gitIdentity);
  const installTex = useStore((state) => state.installTex);
  const setSettingsOpen = useStore((state) => state.setSettingsOpen);
  const warnings: ReactNode[] = [];
  const installing =
    texInstall !== undefined && texInstall.phase !== "finished" && texInstall.phase !== "failed";

  if (texProbe.state === "error" && managedTex !== undefined) {
    if (managedTex.customDir !== null) {
      warnings.push(
        <Warning
          key="tex-custom"
          title="The custom TeX directory is not usable"
          body={`${managedTex.customDir} is missing latexmk, synctex or pdflatex. Clear it in Settings to use the managed TeX Live instead.`}
          action={
            <Button small onClick={() => setSettingsOpen(true)}>
              Open settings
            </Button>
          }
        />,
      );
    } else if (!managedTex.perlAvailable) {
      warnings.push(
        <Warning
          key="perl"
          title="Perl is required to install TeX Live"
          body="TeX Live's installer and package manager are written in Perl. Install Perl from your package manager, then come back here."
          action={
            <Button small onClick={() => spawn(browse(PERL_URL))}>
              How to install Perl
            </Button>
          }
        />,
      );
    } else {
      warnings.push(
        <Warning
          key="tex"
          title="TeX Live is not installed yet"
          body={
            installing && texInstall !== undefined
              ? describeInstall(texInstall)
              : "Compiling needs TeX Live. The app installs its own copy (about 400 MB, a few minutes) into its data folder; nothing else on your system changes."
          }
          action={
            <Button
              small
              variant="primary"
              disabled={installing}
              onClick={() => spawn(installTex())}
            >
              {installing ? "Installing…" : "Install TeX Live"}
            </Button>
          }
        />,
      );
    }
  }

  if (claudeProbe.state === "error") {
    warnings.push(
      <Warning
        key="claude"
        title="Claude Code is not installed"
        body="The Claude panel needs the Claude Code CLI, signed in with your account. Everything else works without it."
        action={
          <Button small onClick={() => spawn(browse(CLAUDE_INSTALL_URL))}>
            Install guide
          </Button>
        }
      />,
    );
  }

  if (gitIdentity === null) {
    warnings.push(
      <Warning
        key="git-identity"
        title="Git does not know who you are"
        body='Commits need a name and email. Run git config --global user.name "Your Name" and git config --global user.email "you@example.com" in the Terminal tab.'
      />,
    );
  }

  if (warnings.length === 0) {
    return null;
  }

  return <div className="setup-warnings">{warnings}</div>;
}
