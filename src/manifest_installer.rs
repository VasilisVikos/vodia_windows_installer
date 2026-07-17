use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn install_staged_windows_folder(staged_dir: &Path, install_dir: &Path) -> Result<()> {
    if !staged_dir.exists() {
        anyhow::bail!("Staged folder does not exist: {}", staged_dir.display());
    }

    let pbx_exe = staged_dir.join("pbxctrl.exe");
    let pbx_dat = staged_dir.join("pbxctrl.dat");

    if !pbx_exe.exists() {
        anyhow::bail!(
            "Staged folder is missing pbxctrl.exe: {}",
            pbx_exe.display()
        );
    }

    if !pbx_dat.exists() {
        anyhow::bail!(
            "Staged folder is missing pbxctrl.dat: {}",
            pbx_dat.display()
        );
    }

    let temp_dir = std::env::temp_dir();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let script_path = temp_dir.join(format!("install-vodia-pbx-{timestamp}.ps1"));
    let cmd_path = temp_dir.join(format!("run-install-vodia-pbx-{timestamp}.cmd"));

    let escaped_staged_dir = escape_powershell_string(&staged_dir.display().to_string());
    let escaped_install_dir = escape_powershell_string(&install_dir.display().to_string());

    let script = format!(
        r#"$ErrorActionPreference = "Stop"

$StagedDir = "{escaped_staged_dir}"
$InstallDir = "{escaped_install_dir}"
$ServiceName = "PBX"
$DisplayName = "Vodia PBX"
$PbxExe = Join-Path $InstallDir "pbxctrl.exe"
$PbxDat = Join-Path $InstallDir "pbxctrl.dat"
$OpusDll = Join-Path $InstallDir "opus.dll"
$InstallLog = Join-Path $InstallDir "installation.txt"

Write-Host "Vodia PBX Installer"
Write-Host "Staged folder: $StagedDir"
Write-Host "Install folder: $InstallDir"
Write-Host ""

Write-Host "Checking for existing Vodia PBX installation..."

$service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue

if ($service) {{
    Write-Host "ERROR: A PBX service already exists."
    Write-Host "Service name: $ServiceName"
    Write-Host "Service status: $($service.Status)"
    Write-Host ""
    Write-Host "This installer will not install over an existing PBX service."
    Write-Host "Please uninstall the existing PBX installation first, then run this installer again."
    exit 10
}}

if (Test-Path $InstallDir) {{
    $ExistingPbxExe = Join-Path $InstallDir "pbxctrl.exe"

    if (Test-Path $ExistingPbxExe) {{
        Write-Host "ERROR: A Vodia PBX installation already exists at:"
        Write-Host $InstallDir
        Write-Host ""
        Write-Host "This installer will not install over an existing PBX folder."
        Write-Host "Please uninstall the existing PBX installation first, then run this installer again."
        exit 11
    }}
}}

Write-Host ""
Write-Host "Creating install directory..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

Write-Host "Copying staged files to install directory..."

Get-ChildItem -LiteralPath $StagedDir -Force | ForEach-Object {{
    $name = $_.Name

    if ($name -eq "install-vodia-pbx.ps1" -or
        $name -eq "run-install-vodia-pbx.cmd" -or
        $name -eq "installation.txt" -or
        $name -like "*.log") {{
        return
    }}

    $destination = Join-Path $InstallDir $name

    if ($_.PSIsContainer) {{
        Copy-Item -LiteralPath $_.FullName -Destination $destination -Recurse -Force
    }} else {{
        Copy-Item -LiteralPath $_.FullName -Destination $destination -Force
    }}
}}

Write-Host ""
Write-Host "Validating installed files..."

if (!(Test-Path $PbxExe)) {{
    Write-Host "ERROR: pbxctrl.exe was not installed."
    exit 20
}}

if (!(Test-Path $PbxDat)) {{
    Write-Host "ERROR: pbxctrl.dat was not installed."
    exit 21
}}

if (!(Test-Path $OpusDll)) {{
    Write-Host "WARNING: opus.dll was not found. Continuing anyway."
}}

Write-Host "Installed pbxctrl.exe: $PbxExe"
Write-Host "Installed pbxctrl.dat: $PbxDat"

Write-Host ""
Write-Host "Preparing first-run setup.json..."

$AdminUser = "vodia"

$RandomPart = -join ((48..57) + (65..90) + (97..122) | Get-Random -Count 16 | ForEach-Object {{ [char]$_ }})
$AdminPassword = "$RandomPart!"

$Md5 = [System.Security.Cryptography.MD5]::Create()
$PasswordBytes = [System.Text.Encoding]::UTF8.GetBytes($AdminPassword)
$HashBytes = $Md5.ComputeHash($PasswordBytes)
$PasswordHash = ([System.BitConverter]::ToString($HashBytes)).Replace("-", "").ToLowerInvariant()

$SetupJsonPath = Join-Path $InstallDir "setup.json"

$SetupJsonContent = @"
{{
  "settings": {{
    "pw_pass": "$PasswordHash",
    "pw_user": "$AdminUser",
    "sys_name": "Vodia PBX"
  }}
}}
"@

$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($SetupJsonPath, $SetupJsonContent, $Utf8NoBom)

Write-Host "setup.json written to: $SetupJsonPath"

Write-Host ""
Write-Host "Starting PBX once to generate the initial configuration..."
Write-Host "This can take up to 30 seconds."

$FirstRunOut = Join-Path $InstallDir "first-run.stdout.log"
$FirstRunErr = Join-Path $InstallDir "first-run.stderr.log"

$FirstRunArgString = "--no-daemon --dir `"$InstallDir`""

$process = Start-Process `
    -FilePath $PbxExe `
    -ArgumentList $FirstRunArgString `
    -WorkingDirectory $InstallDir `
    -RedirectStandardOutput $FirstRunOut `
    -RedirectStandardError $FirstRunErr `
    -PassThru `
    -WindowStyle Hidden

Start-Sleep -Seconds 15

if (!$process.HasExited) {{
    Write-Host "Stopping first-run PBX process..."
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
}}

$PbxXml = Join-Path $InstallDir "pbx.xml"

if (!(Test-Path $PbxXml)) {{
    Write-Host "WARNING: pbx.xml was not created during first run."
    Write-Host "First-run stdout:"
    if (Test-Path $FirstRunOut) {{ Get-Content $FirstRunOut -ErrorAction SilentlyContinue | Out-Host }}
    Write-Host "First-run stderr:"
    if (Test-Path $FirstRunErr) {{ Get-Content $FirstRunErr -ErrorAction SilentlyContinue | Out-Host }}
}} else {{
    Write-Host "pbx.xml created: $PbxXml"

    $PbxXmlText = Get-Content -LiteralPath $PbxXml -Raw

    $DetectedUser = $AdminUser

    if ($PbxXmlText -match "<pw_user>(.*?)</pw_user>") {{
        $DetectedUser = $Matches[1]
        Write-Host "Detected admin username in pbx.xml: $DetectedUser"
    }} else {{
        Write-Host "No pw_user found in pbx.xml."
    }}

    if ($DetectedUser -ne $AdminUser) {{
        Write-Host "WARNING: setup.json requested username '$AdminUser', but pbx.xml shows '$DetectedUser'."
        Write-Host "The PBX may have ignored setup.json or reused an existing configuration."
    }}

    if ($PbxXmlText -match '<pw_pass encrypted="true">(.*?)</pw_pass>') {{
        Write-Host "Detected encrypted admin password in pbx.xml."
        Write-Host "Assuming setup.json password was accepted."
    }} else {{
        Write-Host "WARNING: No encrypted pw_pass found in pbx.xml."
        Write-Host "The generated password may not have been applied."
    }}

    $AdminUser = $DetectedUser
}}

Write-Host ""
Write-Host "Creating Windows service..."

$BinPath = "`"$PbxExe`" --dir `"$InstallDir`""

New-Service `
    -Name $ServiceName `
    -BinaryPathName $BinPath `
    -DisplayName $DisplayName `
    -StartupType Automatic

Write-Host "Service created: $ServiceName"

Write-Host ""
Write-Host "Configuring Windows Firewall..."

try {{
    $ExistingRule = Get-NetFirewallRule -DisplayName "Vodia PBX" -ErrorAction SilentlyContinue

    if (!$ExistingRule) {{
        New-NetFirewallRule `
            -DisplayName "Vodia PBX" `
            -Direction Inbound `
            -Program $PbxExe `
            -Action Allow `
            -Profile Any | Out-Null

        Write-Host "Firewall rule created."
    }} else {{
        Write-Host "Firewall rule already exists."
    }}
}} catch {{
    Write-Host "WARNING: Failed to create firewall rule."
    Write-Host $_
}}

Write-Host ""
Write-Host "Starting PBX service..."

Start-Service -Name $ServiceName

Start-Sleep -Seconds 3

$service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue

if ($service) {{
    Write-Host "Service status: $($service.Status)"
}} else {{
    Write-Host "WARNING: Service was not found after creation."
}}

Write-Host ""
Write-Host "Writing installation details..."

$LocalUrl = "http://localhost"
$HttpsLocalUrl = "https://localhost"

$InstallText = @"
Vodia PBX Installation

Install directory:
$InstallDir

Service name:
$ServiceName

Service display name:
$DisplayName

Access URLs:
$LocalUrl
$HttpsLocalUrl

Admin username:
$AdminUser

Admin password:
$AdminPassword

Important notes:
- These credentials were generated by this installer and written to setup.json before first startup.
- The password was written to setup.json as an MD5 hash.
- Change this password after first login if required by your security policy.
- Keep this file secure. It contains the generated admin password.

Useful commands:

Stop service:
Stop-Service PBX

Start service:
Start-Service PBX

Delete service:
sc.exe delete PBX

Remove install folder:
Remove-Item -Recurse -Force "$InstallDir"

Uninstall manually:
Stop-Service PBX -ErrorAction SilentlyContinue
sc.exe delete PBX
Start-Sleep -Seconds 3
Remove-Item -Recurse -Force "$InstallDir" -ErrorAction SilentlyContinue
Get-NetFirewallRule -DisplayName "*Vodia*" -ErrorAction SilentlyContinue | Remove-NetFirewallRule
"@

[System.IO.File]::WriteAllText($InstallLog, $InstallText, $Utf8NoBom)

$StagedInstallText = Join-Path $StagedDir "installation.txt"
[System.IO.File]::WriteAllText($StagedInstallText, $InstallText, $Utf8NoBom)

Write-Host ""
Write-Host "Cleaning up staged download folder..."

try {{
    Remove-Item -Recurse -Force $StagedDir -ErrorAction Stop
    Write-Host "Cleaned up staged download folder:"
    Write-Host $StagedDir
}} catch {{
    Write-Host "WARNING: Installed successfully, but could not remove staged download folder:"
    Write-Host $StagedDir
    Write-Host $_
}}

Write-Host ""
Write-Host "Installation complete."
Write-Host ""
Write-Host "Open:"
Write-Host $LocalUrl
Write-Host ""
Write-Host "Username: $AdminUser"
Write-Host "Password: $AdminPassword"
Write-Host ""
Write-Host "Installation details saved to:"
Write-Host $InstallLog
Write-Host ""
"#
    );

    fs::write(&script_path, script)
        .with_context(|| format!("Failed to write install script {}", script_path.display()))?;

        let cmd = format!(
            r#"@echo off
        powershell.exe -NoProfile -ExecutionPolicy Bypass -File "{}"
        set EXITCODE=%ERRORLEVEL%
        echo.
        echo Installer finished with exit code %EXITCODE%.
        echo.
        pause
        exit /b %EXITCODE%
        "#,
            script_path.display()
        );

    fs::write(&cmd_path, cmd)
        .with_context(|| format!("Failed to write install launcher {}", cmd_path.display()))?;

    crate::windows_admin::run_elevated_with_args(&cmd_path, "")?;

    Ok(())
}

fn escape_powershell_string(value: &str) -> String {
    value.replace('`', "``").replace('"', "`\"")
}

