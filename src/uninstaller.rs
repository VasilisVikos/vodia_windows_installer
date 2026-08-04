use anyhow::{Context, Result};
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const DEFAULT_INSTALL_DIR: &str = r"C:\Program Files\Vodia\PBX";

pub fn uninstall_vodia_pbx(install_dir: Option<&Path>) -> Result<()> {
    let install_dir = install_dir.unwrap_or_else(|| Path::new(DEFAULT_INSTALL_DIR));

    let temp_dir = std::env::temp_dir();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let script_path = temp_dir.join(format!("uninstall-vodia-pbx-{timestamp}.ps1"));
    let cmd_path = temp_dir.join(format!("run-uninstall-vodia-pbx-{timestamp}.cmd"));

    let escaped_install_dir = escape_powershell_string(&install_dir.display().to_string());

    let script = format!(
        r#"$ErrorActionPreference = "Continue"

$InstallDir = "{escaped_install_dir}"
$ServiceName = "PBX"

Write-Host "Vodia PBX Uninstaller"
Write-Host "Install directory: $InstallDir"
Write-Host ""

Write-Host "Stopping PBX service if it exists..."
Stop-Service $ServiceName -ErrorAction SilentlyContinue

Start-Sleep -Seconds 3

Write-Host "Deleting PBX service if it exists..."
sc.exe delete $ServiceName | Out-Host

Write-Host "Removing Vodia PBX installation folder..."
Remove-Item -Recurse -Force $InstallDir -ErrorAction SilentlyContinue

Write-Host "Removing Vodia firewall rules..."
Get-NetFirewallRule -DisplayName "*Vodia*" -ErrorAction SilentlyContinue | Remove-NetFirewallRule

Write-Host ""
Write-Host "Uninstall complete."
Write-Host ""
Read-Host "Press Enter to continue"
"#
    );

    fs::write(&script_path, script)
        .with_context(|| format!("Failed to write uninstall script {}", script_path.display()))?;

    let cmd = format!(
        r#"@echo off
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "{}"
    exit /b %ERRORLEVEL%
    "#,
        script_path.display()
    );

    fs::write(&cmd_path, cmd)
        .with_context(|| format!("Failed to write uninstall launcher {}", cmd_path.display()))?;

    let cmd_exe = Path::new(r"C:\Windows\System32\cmd.exe");

    let args = format!(r#"/k "{}""#, cmd_path.display());

    crate::windows_admin::run_elevated_with_args(cmd_exe, &args)?;

    Ok(())
}

fn escape_powershell_string(value: &str) -> String {
    value.replace('`', "``").replace('"', "`\"")
}
