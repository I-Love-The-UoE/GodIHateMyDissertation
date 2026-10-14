use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::context::Context;
use crate::download::DownloadProgress;
use crate::download::Downloads;
use crate::process::CommandSpec;
use crate::process::Processes;
use crate::tex::Diagnostic;
use crate::tex::TexError;
use crate::tex::TexToolchain;
use crate::tex::toolchain::validate_dir;

const MANAGED_DIR: &str = "texlive";
const INSTALLER_DIR: &str = "texlive-installer";
const INSTALLER_URL_UNIX: &str =
    "https://mirror.ctan.org/systems/texlive/tlnet/install-tl-unx.tar.gz";
const INSTALLER_URL_WINDOWS: &str = "https://mirror.ctan.org/systems/texlive/tlnet/install-tl.zip";
const BASE_SCHEME: &str = "scheme-basic";
const FALLBACK_REPOSITORY: &str = "https://texlive.info/CTAN/systems/texlive/tlnet";

/// Installed right after the base scheme so pdflatex, xelatex, lualatex, latexmk, `SyncTeX` and the
/// common bibliography tooling work without a further download.
pub const BASE_PACKAGES: &[&str] = &[
    "latexmk",
    "synctex",
    "collection-latexrecommended",
    "collection-fontsrecommended",
    "xetex",
    "luatex",
    "biblatex",
    "biber",
];

static FILE_NOT_FOUND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"File `([^']+)' not found").unwrap_or_else(|_| never()));
static CANT_FIND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"I can't find file `([^']+)'").unwrap_or_else(|_| never()));
static FONT_NOT_LOADABLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Font \\?\S*?=?([A-Za-z0-9-]+)(?: at [\d.]+pt)? not loadable")
        .unwrap_or_else(|_| never())
});

#[expect(
    clippy::panic,
    reason = "A constant regex that fails to compile is a programming error."
)]
fn never() -> Regex {
    panic!("Constant regex failed to compile.")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum TexInstallEvent {
    Downloading { received: u64, total: Option<u64> },
    Extracting,
    Installing { line: String },
    Packages { names: Vec<String> },
    PackageLine { line: String },
    PackagesInstalled { names: Vec<String> },
    Finished { bin_dir: PathBuf },
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedTexLive {
    root: PathBuf,
    installer_dir: PathBuf,
}

fn forward_slashes(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// Missing-file names from a failed compile, each with an extension, deduplicated in order.
pub fn missing_files(diagnostics: &[Diagnostic]) -> Vec<String> {
    let mut files = Vec::new();

    for diagnostic in diagnostics {
        let text = match &diagnostic.detail {
            Some(detail) => format!("{}\n{detail}", diagnostic.message),
            None => diagnostic.message.clone(),
        };

        for capture in FILE_NOT_FOUND
            .captures_iter(&text)
            .chain(CANT_FIND.captures_iter(&text))
        {
            if let Some(name) = capture.get(1) {
                let name = name.as_str().trim();
                let normalised = if Path::new(name).extension().is_some() {
                    name.to_owned()
                } else {
                    format!("{name}.tex")
                };

                if !files.contains(&normalised) {
                    files.push(normalised);
                }
            }
        }

        for capture in FONT_NOT_LOADABLE.captures_iter(&text) {
            if let Some(name) = capture.get(1) {
                let normalised = format!("{}.tfm", name.as_str());

                if !files.contains(&normalised) {
                    files.push(normalised);
                }
            }
        }
    }

    files
}

fn parse_search(output: &str, file: &str) -> Option<String> {
    let suffix = format!("/{file}");
    let mut current: Option<&str> = None;

    for line in output.lines() {
        if !line.starts_with(char::is_whitespace)
            && line.ends_with(':')
            && !line.starts_with("tlmgr")
        {
            current = Some(line.trim_end_matches(':'));

            continue;
        }

        if line.trim().ends_with(&suffix)
            && let Some(package) = current
        {
            return Some(package.to_owned());
        }
    }

    None
}

impl ManagedTexLive {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            root: data_dir.join(MANAGED_DIR),
            installer_dir: data_dir.join(INSTALLER_DIR),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The platform binary directory written by `install-tl`, present as soon as the base
    /// install finished even before the extra packages are in.
    fn platform_bin_dir(&self) -> Option<PathBuf> {
        let tlmgr = if cfg!(windows) { "tlmgr.bat" } else { "tlmgr" };
        let entries = std::fs::read_dir(self.root.join("bin")).ok()?;

        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|dir| dir.join(tlmgr).is_file())
    }

    /// The platform binary directory of the managed install, if the toolchain is complete.
    pub fn bin_dir(&self) -> Option<PathBuf> {
        self.platform_bin_dir()
            .filter(|dir| validate_dir(dir).is_ok())
    }

    fn tlmgr_spec(&self, bin_dir: &Path, args: &[String]) -> CommandSpec {
        let mut all = Vec::new();
        let program = if cfg!(windows) {
            all.push("/c".to_owned());
            all.push(bin_dir.join("tlmgr.bat").display().to_string());

            PathBuf::from("cmd")
        } else {
            bin_dir.join("tlmgr")
        };
        all.extend(args.iter().cloned());

        CommandSpec {
            program,
            args: all,
            cwd: self.root.clone(),
            env: Vec::new(),
            path_prepend: vec![bin_dir.to_path_buf()],
        }
    }

    pub async fn packages_for_files<C: Context>(
        &self,
        ctx: &C,
        files: &[String],
    ) -> Result<Vec<String>, TexError> {
        let bin_dir = self
            .platform_bin_dir()
            .ok_or_else(|| TexError::ToolchainNotInstalled {
                root: self.root.clone(),
            })?;
        let mut packages = Vec::new();

        for file in files {
            let args = vec![
                "search".to_owned(),
                "--global".to_owned(),
                "--file".to_owned(),
                format!("/{file}"),
            ];
            let captured = ctx
                .processes()
                .capture(self.tlmgr_spec(&bin_dir, &args))
                .await?;

            if let Some(package) = parse_search(&captured.stdout, file)
                && !packages.contains(&package)
            {
                packages.push(package);
            }
        }

        Ok(packages)
    }

    /// `tlmgr install` exits 0 even when a mirror serves a corrupt archive, so the result is
    /// verified against the local database and retried once on a known-good repository.
    pub async fn install_packages<C: Context>(
        &self,
        ctx: &C,
        names: &[String],
        events: &mpsc::Sender<TexInstallEvent>,
    ) -> Result<(), TexError> {
        let bin_dir = self
            .platform_bin_dir()
            .ok_or_else(|| TexError::ToolchainNotInstalled {
                root: self.root.clone(),
            })?;
        let _ = events
            .send(TexInstallEvent::Packages {
                names: names.to_vec(),
            })
            .await;
        let mut pending = names.to_vec();

        for repository in [None, Some(FALLBACK_REPOSITORY)] {
            let mut args = vec!["install".to_owned()];

            if let Some(repository) = repository {
                args.push("--repository".to_owned());
                args.push(repository.to_owned());
            }

            args.extend(pending.iter().cloned());
            let (_, tail) = self
                .stream(ctx, self.tlmgr_spec(&bin_dir, &args), events, |line| {
                    TexInstallEvent::PackageLine { line }
                })
                .await?;
            pending = self.not_installed(ctx, &bin_dir, &pending).await?;

            if pending.is_empty() {
                return Ok(());
            }

            tracing::warn!(packages = ?pending, repository = ?repository, "tlmgr did not install every package.");
            let _ = tail;
        }

        Err(TexError::InstallFailed {
            step: "tlmgr install".to_owned(),
            detail: format!("not installed: {}", pending.join(", ")),
        })
    }

    /// Files kpathsea cannot find among `files` (`name.sty`, `name.cls`, ...), checked in one call.
    pub async fn missing_from_tree<C: Context>(
        &self,
        ctx: &C,
        files: &[String],
    ) -> Result<Vec<String>, TexError> {
        let bin_dir = self
            .platform_bin_dir()
            .ok_or_else(|| TexError::ToolchainNotInstalled {
                root: self.root.clone(),
            })?;

        if files.is_empty() {
            return Ok(Vec::new());
        }

        let kpsewhich = bin_dir.join(if cfg!(windows) {
            "kpsewhich.exe"
        } else {
            "kpsewhich"
        });
        let spec = CommandSpec {
            program: kpsewhich,
            args: files.to_vec(),
            cwd: self.root.clone(),
            env: Vec::new(),
            path_prepend: vec![bin_dir],
        };
        let captured = ctx.processes().capture(spec).await?;
        let found: Vec<String> = captured
            .stdout
            .lines()
            .filter_map(|line| {
                Path::new(line.trim())
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .collect();

        Ok(files
            .iter()
            .filter(|file| !found.contains(file))
            .cloned()
            .collect())
    }

    /// Installs whatever is needed for `files` to exist: the package named like the file first
    /// (true for nearly every CTAN package), then a precise `tlmgr search` for the rest. Returns
    /// the files no package provides.
    pub async fn ensure_files<C: Context>(
        &self,
        ctx: &C,
        files: &[String],
        events: &mpsc::Sender<TexInstallEvent>,
    ) -> Result<Vec<String>, TexError> {
        let missing = self.missing_from_tree(ctx, files).await?;

        if missing.is_empty() {
            return Ok(Vec::new());
        }

        let guessed: Vec<String> = missing
            .iter()
            .filter_map(|file| {
                Path::new(file)
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            })
            .collect();
        let mut installed = Vec::new();

        if let Ok(()) = self.install_packages(ctx, &guessed, events).await {
            installed.extend(guessed.iter().cloned());
        }

        let still_missing = self.missing_from_tree(ctx, &missing).await?;

        if !still_missing.is_empty() {
            let packages = self.packages_for_files(ctx, &still_missing).await?;

            if !packages.is_empty() {
                self.install_packages(ctx, &packages, events).await?;
                installed.extend(packages);
            }
        }

        if !installed.is_empty() {
            let _ = events
                .send(TexInstallEvent::PackagesInstalled { names: installed })
                .await;
        }

        self.missing_from_tree(ctx, &missing).await
    }

    async fn not_installed<C: Context>(
        &self,
        ctx: &C,
        bin_dir: &Path,
        names: &[String],
    ) -> Result<Vec<String>, TexError> {
        let mut args = vec![
            "info".to_owned(),
            "--only-installed".to_owned(),
            "--data".to_owned(),
            "name".to_owned(),
        ];
        args.extend(names.iter().cloned());
        let captured = ctx
            .processes()
            .capture(self.tlmgr_spec(bin_dir, &args))
            .await?;
        let installed: Vec<&str> = captured.stdout.lines().map(str::trim).collect();

        Ok(names
            .iter()
            .filter(|name| !installed.contains(&name.as_str()))
            .cloned()
            .collect())
    }

    async fn stream<C: Context>(
        &self,
        ctx: &C,
        spec: CommandSpec,
        events: &mpsc::Sender<TexInstallEvent>,
        map: fn(String) -> TexInstallEvent,
    ) -> Result<(Option<i32>, String), TexError> {
        let (lines_tx, mut lines_rx) = mpsc::channel::<String>(256);
        let forward = events.clone();
        let collector = tokio::spawn(async move {
            let mut tail: Vec<String> = Vec::new();

            while let Some(line) = lines_rx.recv().await {
                tail.push(line.clone());

                if tail.len() > 12 {
                    tail.remove(0);
                }

                let _ = forward.send(map(line)).await;
            }

            tail.join("\n")
        });
        let exit = ctx
            .processes()
            .run(spec, lines_tx, CancellationToken::new())
            .await?;
        let tail = collector.await.unwrap_or_default();

        Ok((exit.code, tail))
    }

    fn profile(&self) -> String {
        let root = forward_slashes(&self.root);

        format!(
            "selected_scheme {BASE_SCHEME}\nTEXDIR {root}\nTEXMFLOCAL {root}/texmf-local\nTEXMFSYSCONFIG {root}/texmf-config\nTEXMFSYSVAR {root}/texmf-var\nTEXMFCONFIG {root}/texmf-config\nTEXMFVAR {root}/texmf-var\nTEXMFHOME {root}/texmf-home\ninstopt_adjustpath 0\ninstopt_adjustrepo 1\ninstopt_letter 0\ninstopt_portable 1\ntlpdbopt_autobackup 0\ntlpdbopt_desktop_integration 0\ntlpdbopt_file_assocs 0\ntlpdbopt_install_docfiles 0\ntlpdbopt_install_srcfiles 0\n"
        )
    }

    /// Downloads the official installer, runs it unattended into the managed root, then installs
    /// the base package set. Progress is reported on `events`; the final event is `Finished` or
    /// `Failed`.
    pub async fn install<C: Context>(
        &self,
        ctx: &C,
        events: mpsc::Sender<TexInstallEvent>,
    ) -> Result<TexToolchain, TexError> {
        let result = self.install_inner(ctx, &events).await;

        match &result {
            Ok(toolchain) => {
                let _ = events
                    .send(TexInstallEvent::Finished {
                        bin_dir: toolchain.bin_dir.clone(),
                    })
                    .await;
            }
            Err(error) => {
                let _ = events
                    .send(TexInstallEvent::Failed {
                        error: error.to_string(),
                    })
                    .await;
            }
        }

        result
    }

    /// Downloads and unpacks the installer into a fresh installer directory, returning it.
    async fn fetch_installer<C: Context>(
        &self,
        ctx: &C,
        events: &mpsc::Sender<TexInstallEvent>,
    ) -> Result<PathBuf, TexError> {
        let windows = cfg!(windows);
        let url = if windows {
            INSTALLER_URL_WINDOWS
        } else {
            INSTALLER_URL_UNIX
        };
        let archive = self.installer_dir.join(if windows {
            "install-tl.zip"
        } else {
            "install-tl-unx.tar.gz"
        });
        let unpacked = self.installer_dir.join("unpacked");
        let _ = tokio::fs::remove_dir_all(&self.installer_dir).await;
        tokio::fs::create_dir_all(&unpacked)
            .await
            .map_err(|source| TexError::Io {
                path: unpacked.clone(),
                source,
            })?;
        let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(64);
        let forward = events.clone();
        let forwarder = tokio::spawn(async move {
            while let Some(progress) = progress_rx.recv().await {
                let _ = forward
                    .send(TexInstallEvent::Downloading {
                        received: progress.received,
                        total: progress.total,
                    })
                    .await;
            }
        });
        ctx.downloads().download(url, &archive, progress_tx).await?;
        let _ = forwarder.await;
        let _ = events.send(TexInstallEvent::Extracting).await;
        let tar = CommandSpec {
            program: PathBuf::from("tar"),
            args: vec![
                "-xf".to_owned(),
                archive.display().to_string(),
                "-C".to_owned(),
                unpacked.display().to_string(),
                "--strip-components=1".to_owned(),
            ],
            cwd: self.installer_dir.clone(),
            env: Vec::new(),
            path_prepend: Vec::new(),
        };
        let extracted = ctx.processes().capture(tar).await?;

        if extracted.outcome.code != Some(0) {
            return Err(TexError::InstallFailed {
                step: "extract".to_owned(),
                detail: extracted.stderr,
            });
        }

        Ok(unpacked)
    }

    async fn install_inner<C: Context>(
        &self,
        ctx: &C,
        events: &mpsc::Sender<TexInstallEvent>,
    ) -> Result<TexToolchain, TexError> {
        let windows = cfg!(windows);
        let unpacked = self.fetch_installer(ctx, events).await?;
        let profile_path = self.installer_dir.join("profile.txt");
        tokio::fs::write(&profile_path, self.profile())
            .await
            .map_err(|source| TexError::Io {
                path: profile_path.clone(),
                source,
            })?;
        let (program, mut args) = if windows {
            (
                PathBuf::from("cmd"),
                vec![
                    "/c".to_owned(),
                    unpacked
                        .join("install-tl-windows.bat")
                        .display()
                        .to_string(),
                ],
            )
        } else {
            (
                PathBuf::from("perl"),
                vec![unpacked.join("install-tl").display().to_string()],
            )
        };
        args.push("-profile".to_owned());
        args.push(profile_path.display().to_string());
        args.push("-no-gui".to_owned());
        let installer = CommandSpec {
            program,
            args,
            cwd: unpacked.clone(),
            env: vec![("TEXLIVE_INSTALL_ENV_NOCHECK".to_owned(), "1".to_owned())],
            path_prepend: Vec::new(),
        };
        let (code, tail) = self
            .stream(ctx, installer, events, |line| TexInstallEvent::Installing {
                line,
            })
            .await?;

        if code != Some(0) {
            return Err(TexError::InstallFailed {
                step: "install-tl".to_owned(),
                detail: tail,
            });
        }

        let names: Vec<String> = BASE_PACKAGES
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        self.install_packages(ctx, &names, events).await?;
        let _ = tokio::fs::remove_dir_all(&self.installer_dir).await;
        let bin_dir = self
            .platform_bin_dir()
            .ok_or_else(|| TexError::ToolchainNotInstalled {
                root: self.root.clone(),
            })?;

        validate_dir(&bin_dir).map_err(|missing| TexError::ToolchainInvalid {
            dir: bin_dir,
            missing,
        })
    }
}
