use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPlatform {
    Windows64,
    MacOS,
}

impl TargetPlatform {
    pub fn from_user_value(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "windows" | "win" | "win64" | "windows64" => Ok(Self::Windows64),
            "macos" | "mac" | "darwin" => Ok(Self::MacOS),
            other => bail!("Unknown target platform: {other}. Use windows or macos."),
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Windows64 => "Windows 64-bit",
            Self::MacOS => "MacOS",
        }
    }

    pub fn folder_label(self) -> &'static str {
        match self {
            Self::Windows64 => "windows",
            Self::MacOS => "macos",
        }
    }

    pub fn manifest_os(self) -> &'static str {
        match self {
            Self::Windows64 => "Win64",
            Self::MacOS => "MacOS",
        }
    }

    pub fn required_files(self) -> &'static [&'static str] {
        match self {
            Self::Windows64 => &["pbxctrl.exe", "pbxctrl.dat", "opus.dll"],
            Self::MacOS => &["pbxctrl", "pbxctrl.dat"],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupPackageKind {
    Zip,
    Tar,
    TarGz,
    Tgz,
    Exe,
}

impl SetupPackageKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Zip => "ZIP",
            Self::Tar => "TAR",
            Self::TarGz => "TAR.GZ",
            Self::Tgz => "TGZ",
            Self::Exe => "EXE",
        }
    }

    pub fn temp_label(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Tar => "tar",
            Self::TarGz => "tar-gz",
            Self::Tgz => "tgz",
            Self::Exe => "exe",
        }
    }

    pub fn is_archive(self) -> bool {
        matches!(self, Self::Zip | Self::Tar | Self::TarGz | Self::Tgz)
    }
}

#[derive(Debug, Clone)]
pub struct SetupPackageCandidate {
    pub kind: SetupPackageKind,
    pub url: String,
    pub filename: String,
}

pub fn candidate_windows_setup_packages(version: &str) -> Vec<SetupPackageCandidate> {
    let base = format!("https://portal.vodia.com/downloads/pbx/win64/pbxctrl-v{version}-64");

    let specs = [
        (SetupPackageKind::Zip, "zip"),
        (SetupPackageKind::Tar, "tar"),
        (SetupPackageKind::TarGz, "tar.gz"),
        (SetupPackageKind::Tgz, "tgz"),
        (SetupPackageKind::Exe, "exe"),
    ];

    specs
        .into_iter()
        .map(|(kind, extension)| SetupPackageCandidate {
            kind,
            url: format!("{base}.{extension}"),
            filename: format!("pbxctrl-v{version}-64.{extension}"),
        })
        .collect()
}

pub fn manifest_url(version: &str) -> String {
    format!("https://portal.vodia.com/downloads/pbx/version-{version}.xml")
}

pub fn candidate_manifest_urls(version: &str) -> Vec<String> {
    let mut urls = Vec::new();

    let exact_url = manifest_url(version);
    urls.push(exact_url);

    let parts: Vec<&str> = version.split('.').collect();

    if parts.len() >= 3 {
        let major_minor = format!("{}.{}", parts[0], parts[1]);
        let major_minor_url = manifest_url(&major_minor);

        if !urls.contains(&major_minor_url) {
            urls.push(major_minor_url);
        }
    }

    urls
}