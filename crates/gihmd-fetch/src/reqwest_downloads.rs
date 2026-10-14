use std::path::Path;

use futures_util::StreamExt;
use gihmd_core::DownloadError;
use gihmd_core::DownloadProgress;
use gihmd_core::Downloads;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct ReqwestDownloads {
    client: reqwest::Client,
}

impl Downloads for ReqwestDownloads {
    async fn download(
        &self,
        url: &str,
        dest: &Path,
        progress: mpsc::Sender<DownloadProgress>,
    ) -> Result<(), DownloadError> {
        let response =
            self.client
                .get(url)
                .send()
                .await
                .map_err(|error| DownloadError::Request {
                    url: url.to_owned(),
                    detail: error.to_string(),
                })?;
        let status = response.status();

        if !status.is_success() {
            return Err(DownloadError::Status {
                url: url.to_owned(),
                status: status.as_u16(),
            });
        }

        let total = response.content_length();

        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|source| DownloadError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
        }

        let mut file = tokio::fs::File::create(dest)
            .await
            .map_err(|source| DownloadError::Io {
                path: dest.to_path_buf(),
                source,
            })?;
        let mut stream = response.bytes_stream();
        let mut received = 0_u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| DownloadError::Interrupted {
                url: url.to_owned(),
                detail: error.to_string(),
            })?;
            file.write_all(&chunk)
                .await
                .map_err(|source| DownloadError::Io {
                    path: dest.to_path_buf(),
                    source,
                })?;
            received += chunk.len() as u64;
            let _ = progress.try_send(DownloadProgress { received, total });
        }

        file.flush().await.map_err(|source| DownloadError::Io {
            path: dest.to_path_buf(),
            source,
        })?;

        Ok(())
    }
}

pub fn default() -> ReqwestDownloads {
    // NOTE: reqwest is built without a bundled crypto provider; `ring` is installed once here.
    // A second install attempt (tests building two contexts) returns Err and changes nothing.
    let _ = rustls::crypto::ring::default_provider().install_default();

    ReqwestDownloads {
        client: reqwest::Client::new(),
    }
}
