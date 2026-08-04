use crate::releases::{SetupPackageCandidate, SetupPackageKind, TargetPlatform};
use anyhow::{bail, Context, Result};
use reqwest::{header::RANGE, Client};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
struct ManifestItem {
    name: String,
    link: String,
    executable: bool,
}

pub async fn run_vodia_setup(
    version: &str,
    target: TargetPlatform,
    prefer_setup_package: bool,
    install_after_download: bool,
    install_dir: &Path,
) -> Result<()> {
    let client = Client::builder().user_agent("VodiaPBXWizard/1.0").build()?;

    if target == TargetPlatform::Windows64 && prefer_setup_package {
        let candidates = crate::releases::candidate_windows_setup_packages(version);

        println!("Checking Windows package candidates:");
        for candidate in &candidates {
            println!("  [{}] {}", candidate.kind.display_name(), candidate.url);
        }
        println!();

        let mut found_any_candidate = false;

        for candidate in &candidates {
            if !url_exists(&client, &candidate.url).await {
                continue;
            }

            found_any_candidate = true;

            println!("Found {} candidate:", candidate.kind.display_name());
            println!("{}", candidate.url);
            println!();

            if candidate.kind.is_archive() {
                let launched = try_run_setup_archive(&client, version, candidate).await?;

                if launched {
                    return Ok(());
                }

                println!(
                    "The {} package did not contain setup.exe, so it will not be used as a setup package.",
                    candidate.kind.display_name()
                );
                println!();
                continue;
            }

            if candidate.kind == SetupPackageKind::Exe {
                println!("The EXE candidate appears to be the direct PBX executable.");
                println!("It is not treated as a full setup package by itself.");
                println!("Using XML manifest mode so pbxctrl.exe, pbxctrl.dat, and opus.dll are downloaded together.");
                println!();
                break;
            }
        }

        if !found_any_candidate {
            println!("No ZIP, TAR, TGZ, or EXE package candidate was found.");
            println!("Using XML manifest mode instead.");
            println!();
        }
    }

    run_manifest_download(client, version, target, install_after_download, install_dir).await?;

    Ok(())
}

async fn try_run_setup_archive(
    client: &Client,
    version: &str,
    candidate: &SetupPackageCandidate,
) -> Result<bool> {
    let work_dir = env::temp_dir().join(format!(
        "VodiaInstallerWizard-{version}-{}",
        candidate.kind.temp_label()
    ));

    let package_path = work_dir.join(&candidate.filename);
    let extract_dir = work_dir.join("extracted");

    if work_dir.exists() {
        fs::remove_dir_all(&work_dir)?;
    }

    fs::create_dir_all(&work_dir)?;

    println!("Downloading {} package to:", candidate.kind.display_name());
    println!("{}", package_path.display());
    println!();

    crate::installer::download_file(client, &candidate.url, &package_path).await?;

    println!("Extracting {} package to:", candidate.kind.display_name());
    println!("{}", extract_dir.display());
    println!();

    match candidate.kind {
        SetupPackageKind::Zip => {
            crate::installer::extract_zip(&package_path, &extract_dir)?;
        }

        SetupPackageKind::Tar => {
            crate::installer::extract_tar(&package_path, &extract_dir)?;
        }

        SetupPackageKind::TarGz | SetupPackageKind::Tgz => {
            crate::installer::extract_tar_gz(&package_path, &extract_dir)?;
        }

        SetupPackageKind::Exe => {
            return Ok(false);
        }
    }

    let setup_exe = match crate::installer::find_setup_exe(&extract_dir) {
        Some(path) => path,
        None => return Ok(false),
    };

    println!("Launching Vodia setup.exe as Administrator:");
    println!("{}", setup_exe.display());
    println!();

    crate::windows_admin::run_elevated(&setup_exe)?;

    Ok(true)
}

async fn run_manifest_download(
    client: Client,
    version: &str,
    target: TargetPlatform,
    install_after_download: bool,
    install_dir: &Path,
) -> Result<()> {
    let (manifest_url, xml) = fetch_manifest_xml(&client, version).await?;

    println!("Using manifest:");
    println!("{manifest_url}");
    println!();

    let document =
        roxmltree::Document::parse(&xml).context("Failed to parse Vodia XML manifest")?;

    let downloads = collect_manifest_items(&document, target)?;

    if downloads.is_empty() {
        bail!(
            "The manifest did not contain any shared files or {} files.",
            target.display_name()
        );
    }

    let target_dir =
        env::current_dir()?.join(format!("VodiaPBX-{version}-{}", target.folder_label()));

    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
    }

    fs::create_dir_all(&target_dir)?;

    println!("Downloading manifest files into:");
    println!("{}", target_dir.display());
    println!();

    for item in downloads {
        let relative_path = safe_relative_path(&item.name)?;
        let output_path = target_dir.join(relative_path);

        println!("Downloading {}", item.name);
        println!("  from: {}", item.link);
        println!("  to:   {}", output_path.display());

        crate::installer::download_file(&client, &item.link, &output_path).await?;
        make_executable_if_supported(&output_path, item.executable)?;

        println!();
    }

    validate_required_files(&target_dir, target)?;

    println!("Manifest download completed.");
    println!();
    println!("Files are staged here:");
    println!("{}", target_dir.display());
    println!();

    match target {
        TargetPlatform::Windows64 => {
            println!("Downloaded Windows PBX files should include:");
            println!("  pbxctrl.exe");
            println!("  pbxctrl.dat");
            println!("  opus.dll");
            println!();

            if install_after_download {
                println!("Launching elevated Windows install step...");
                println!("This will copy the staged files to:");
                println!("{}", install_dir.display());
                println!("and create/start the PBX Windows service.");
                println!();
                println!("Final login details will be written after the PBX initializes:");
                println!("{}", install_dir.join("installation.txt").display());
                println!();

                crate::manifest_installer::install_staged_windows_folder(&target_dir, install_dir)?;
            } else {
                println!("Install step skipped.");
                println!("Files were staged only.");
                println!();
                println!("To install in the same step, run:");
                println!(
                    "vodia-pbx-wizard.exe {} --install --install-dir \"{}\"",
                    version,
                    install_dir.display()
                );
            }
        }

        TargetPlatform::MacOS => {
            println!("Downloaded MacOS PBX files should include:");
            println!("  pbxctrl");
            println!("  pbxctrl.dat");
            println!();
            println!("After copying this folder to a Mac, you may need:");
            println!();
            println!("chmod +x pbxctrl");
            println!("./pbxctrl --version");
        }
    }

    Ok(())
}

async fn fetch_manifest_xml(client: &Client, version: &str) -> Result<(String, String)> {
    let candidates = crate::releases::candidate_manifest_urls(version);
    let mut last_error = String::new();

    for (index, url) in candidates.iter().enumerate() {
        println!("Trying manifest:");
        println!("{url}");
        println!();

        match client.get(url).send().await {
            Ok(response) => {
                let status = response.status();

                if !status.is_success() {
                    last_error = format!("{url} returned {status}");
                    continue;
                }

                let xml = response
                    .text()
                    .await
                    .with_context(|| format!("Failed to read manifest body: {url}"))?;

                if index > 0 && version_has_patch(version) && !xml.contains(version) {
                    last_error = format!(
                        "{url} downloaded successfully, but it did not appear to describe version {version}"
                    );
                    continue;
                }

                return Ok((url.to_string(), xml));
            }

            Err(error) => {
                last_error = format!("{url} failed: {error}");
            }
        }
    }

    if last_error.is_empty() {
        bail!("Could not download a Vodia manifest for version {version}.");
    }

    bail!("Could not download a Vodia manifest for version {version}. Last error: {last_error}");
}

fn collect_manifest_items(
    document: &roxmltree::Document<'_>,
    target: TargetPlatform,
) -> Result<Vec<ManifestItem>> {
    let mut items = Vec::new();

    for node in document
        .descendants()
        .filter(|node| node.has_tag_name("file"))
    {
        let name = match node.attribute("name") {
            Some(value) => value,
            None => continue,
        };

        let link = match node.attribute("link") {
            Some(value) => value,
            None => continue,
        };

        let os = node.attribute("os");

        if !file_matches_target(os, target) {
            continue;
        }

        let executable = node
            .attribute("type")
            .map(|value| value.eq_ignore_ascii_case("executable"))
            .unwrap_or(false);

        let safe_link = normalize_vodia_download_url(link)?;

        items.push(ManifestItem {
            name: name.to_string(),
            link: safe_link,
            executable,
        });
    }

    Ok(items)
}

fn file_matches_target(os: Option<&str>, target: TargetPlatform) -> bool {
    match os {
        None => true,
        Some(value) => value.eq_ignore_ascii_case(target.manifest_os()),
    }
}

fn normalize_vodia_download_url(link: &str) -> Result<String> {
    let normalized = if link.starts_with("http://portal.vodia.com/downloads/pbx/") {
        link.replacen("http://", "https://", 1)
    } else {
        link.to_string()
    };

    if !normalized.starts_with("https://portal.vodia.com/downloads/pbx/") {
        bail!("Manifest contained an unexpected download URL: {link}");
    }

    Ok(normalized)
}

fn safe_relative_path(name: &str) -> Result<PathBuf> {
    let mut path = PathBuf::new();

    for part in name.split('/') {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.contains(':')
            || part.contains('\\')
        {
            bail!("Unsafe path in manifest: {name}");
        }

        path.push(part);
    }

    Ok(path)
}

fn validate_required_files(target_dir: &Path, target: TargetPlatform) -> Result<()> {
    for required in target.required_files() {
        let required_path = target_dir.join(required);

        if !required_path.exists() {
            bail!(
                "Manifest download finished, but required file was not found: {}",
                required
            );
        }
    }

    Ok(())
}

fn version_has_patch(version: &str) -> bool {
    version.split('.').count() >= 3
}

async fn url_exists(client: &Client, url: &str) -> bool {
    if let Ok(response) = client.head(url).send().await {
        if response.status().is_success() {
            return true;
        }
    }

    match client.get(url).header(RANGE, "bytes=0-0").send().await {
        Ok(response) => response.status().is_success() || response.status().as_u16() == 206,
        Err(_) => false,
    }
}

fn make_executable_if_supported(_path: &Path, executable: bool) -> Result<()> {
    if !executable {
        return Ok(());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(_path)?.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(_path, permissions)?;
    }

    Ok(())
}
