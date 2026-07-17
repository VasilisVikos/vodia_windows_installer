use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use reqwest::Client;
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Component, Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

pub async fn download_file(client: &Client, url: &str, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut response = client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("Download failed: {url}"))?;

    let mut file = tokio::fs::File::create(output).await?;

    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }

    Ok(())
}

pub fn extract_zip(zip_path: &Path, output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;

        let enclosed_name = entry
            .enclosed_name()
            .context("ZIP contains an unsafe path")?;

        let output_path = output_dir.join(enclosed_name);

        if entry.is_dir() {
            fs::create_dir_all(&output_path)?;
        } else {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut output = File::create(&output_path)?;
            io::copy(&mut entry, &mut output)?;
        }
    }

    Ok(())
}

pub fn extract_tar(tar_path: &Path, output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    let file = File::open(tar_path)?;
    extract_tar_reader(file, output_dir)
}

pub fn extract_tar_gz(tar_gz_path: &Path, output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    let file = File::open(tar_gz_path)?;
    let decoder = GzDecoder::new(file);

    extract_tar_reader(decoder, output_dir)
}

fn extract_tar_reader<R: Read>(reader: R, output_dir: &Path) -> Result<()> {
    let mut archive = tar::Archive::new(reader);

    for entry in archive.entries()? {
        let mut entry = entry?;

        let entry_path = entry.path()?.into_owned();
        let safe_path = safe_archive_relative_path(&entry_path)?;
        let output_path = output_dir.join(safe_path);

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        entry.unpack(&output_path)?;
    }

    Ok(())
}

fn safe_archive_relative_path(path: &Path) -> Result<PathBuf> {
    let mut safe = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            Component::CurDir => {}
            _ => bail!("Archive contains an unsafe path: {}", path.display()),
        }
    }

    if safe.as_os_str().is_empty() {
        bail!("Archive contains an empty path");
    }

    Ok(safe)
}

pub fn find_setup_exe(dir: &Path) -> Option<PathBuf> {
    let direct = dir.join("setup.exe");

    if direct.exists() {
        return Some(direct);
    }

    for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|entry| entry.ok()) {
        let path = entry.path();

        if path.is_file()
            && path
                .file_name()
                .map(|name| name.to_string_lossy().eq_ignore_ascii_case("setup.exe"))
                .unwrap_or(false)
        {
            return Some(path.to_path_buf());
        }
    }

    None
}