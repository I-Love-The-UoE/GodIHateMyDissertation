use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

use crate::process::CommandSpec;
use crate::process::Processes;
use crate::tex::TexError;
use crate::tex::TexToolchain;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardHit {
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub h: f64,
    pub v: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InverseHit {
    pub file: PathBuf,
    pub line: u32,
    pub column: i32,
}

fn spec(toolchain: &TexToolchain, compile_dir: &Path, args: Vec<String>) -> CommandSpec {
    CommandSpec {
        program: toolchain.synctex.clone(),
        args,
        cwd: compile_dir.to_path_buf(),
        env: Vec::new(),
        path_prepend: vec![toolchain.bin_dir.clone()],
    }
}

fn records(output: &str) -> Vec<Vec<(String, String)>> {
    let mut all = Vec::new();
    let mut current: Option<Vec<(String, String)>> = None;

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("Output:")
            && let Some(record) = current.take()
        {
            all.push(record);
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim();

            if key == "Output" {
                current = Some(vec![(key.to_owned(), value.trim().to_owned())]);
            } else if let Some(record) = current.as_mut() {
                record.push((key.to_owned(), value.trim().to_owned()));
            }
        }
    }

    if let Some(record) = current {
        all.push(record);
    }

    all
}

fn field<'a>(record: &'a [(String, String)], key: &str) -> Option<&'a str> {
    record
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn parse_forward(stdout: &str) -> Vec<ForwardHit> {
    records(stdout)
        .iter()
        .filter_map(|record| {
            Some(ForwardHit {
                page: field(record, "Page")?.parse().ok()?,
                x: field(record, "x")?.parse().ok()?,
                y: field(record, "y")?.parse().ok()?,
                h: field(record, "h")?.parse().ok()?,
                v: field(record, "v")?.parse().ok()?,
                width: field(record, "W")?.parse().ok()?,
                height: field(record, "H")?.parse().ok()?,
            })
        })
        .collect()
}

fn is_synctex_output(text: &str) -> bool {
    text.contains("SyncTeX result begin") || text.contains("SyncTeX Warning")
}

/// `file` is the absolute path of the source file. TeX records inputs found through `TEXINPUTS`
/// by absolute path and inputs found in the working directory by `./relative` path, so both
/// spellings are tried; an empty result means `SyncTeX` has no record for that file.
pub async fn forward<P: Processes>(
    processes: &P,
    toolchain: &TexToolchain,
    pdf: &Path,
    compile_dir: &Path,
    file: &Path,
    line: u32,
    column: u32,
) -> Result<Vec<ForwardHit>, TexError> {
    let mut names = vec![file.display().to_string()];

    if let Ok(relative) = file.strip_prefix(compile_dir) {
        names.push(format!(
            "./{}",
            relative.to_string_lossy().replace('\\', "/")
        ));
    }

    for name in names {
        let args = vec![
            "view".to_owned(),
            "-i".to_owned(),
            format!("{line}:{column}:{name}"),
            "-o".to_owned(),
            pdf.display().to_string(),
        ];
        let captured = processes
            .capture(spec(toolchain, compile_dir, args))
            .await?;

        if !is_synctex_output(&captured.stdout) {
            return Err(TexError::SynctexUnparsable {
                output: captured.stdout + &captured.stderr,
            });
        }

        let hits = parse_forward(&captured.stdout);

        if !hits.is_empty() {
            return Ok(hits);
        }
    }

    Ok(Vec::new())
}

pub async fn inverse<P: Processes>(
    processes: &P,
    toolchain: &TexToolchain,
    pdf: &Path,
    compile_dir: &Path,
    page: u32,
    x: f64,
    y: f64,
) -> Result<Vec<InverseHit>, TexError> {
    let args = vec![
        "edit".to_owned(),
        "-o".to_owned(),
        format!("{page}:{x}:{y}:{}", pdf.display()),
    ];
    let captured = processes
        .capture(spec(toolchain, compile_dir, args))
        .await?;
    let hits: Vec<InverseHit> = records(&captured.stdout)
        .iter()
        .filter_map(|record| {
            let input = field(record, "Input")?;
            let path = Path::new(input);
            let file = if path.is_absolute() {
                path.to_path_buf()
            } else {
                compile_dir.join(path)
            };

            Some(InverseHit {
                file: crate::paths::normalise(&file).unwrap_or(file),
                line: field(record, "Line")?.parse().ok()?,
                column: field(record, "Column")?.parse().ok()?,
            })
        })
        .collect();

    if hits.is_empty() && !is_synctex_output(&captured.stdout) {
        return Err(TexError::SynctexUnparsable {
            output: captured.stdout + &captured.stderr,
        });
    }

    Ok(hits)
}
