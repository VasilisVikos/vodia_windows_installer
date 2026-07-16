use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn install_staged_windows_folder(
    staged_dir: &Path,
    install_dir: &Path,
) -> Result<()> {
    let script_path = staged_dir.join("install-vodia-pbx.ps1");
    let cmd_path = staged_dir.join("run-install-vodia-pbx.cmd");
    let log_path = staged_dir.join("install-vodia-pbx.log");

    let script = format!(
        r#"
$ErrorActionPreference = "Stop"

$SourceDir = "{source_dir}"
$InstallDir = "{install_dir}"
$ServiceName = "PBX"
$DisplayName = "Vodia PBX"
$LogPath = "{log_path}"

try {{
    Start-Transcript -Path $LogPath -Append
}} catch {{
    Write-Host "Could not start transcript:"
    Write-Host $_
}}

try {{
    Write-Host "=== Vodia PBX Windows Installation ==="
    Write-Host "Source: $SourceDir"
    Write-Host "Target: $InstallDir"
    Write-Host "Log: $LogPath"
    Write-Host ""

    if (!(Test-Path -LiteralPath $SourceDir)) {{
        throw "Source directory does not exist: $SourceDir"
    }}

    Write-Host "Creating install directory..."
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

    Write-Host "Stopping existing PBX service if present..."

    $service = $null
    try {{
        $service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    }} catch {{
        Write-Host "Could not query service with Get-Service. Continuing..."
        Write-Host $_
    }}

    if ($service) {{
        Write-Host "Existing PBX service found. Attempting to stop it..."

        try {{
            if ($service.Status -ne "Stopped") {{
                Stop-Service -Name $ServiceName -Force -ErrorAction Stop
                Start-Sleep -Seconds 3
            }}
        }} catch {{
            Write-Host "Stop-Service failed. Trying WMI service stop..."
            Write-Host $_

            try {{
                $wmiService = Get-WmiObject -Class Win32_Service -Filter "Name='$ServiceName'"
                if ($wmiService) {{
                    $stopResult = $wmiService.StopService()
                    Write-Host "WMI StopService return code: $($stopResult.ReturnValue)"
                    Start-Sleep -Seconds 3
                }}
            }} catch {{
                Write-Host "WMI stop also failed. Continuing..."
                Write-Host $_
            }}
        }}

        Write-Host "Deleting existing PBX service..."
        try {{
            $existing = Get-WmiObject -Class Win32_Service -Filter "Name='$ServiceName'"
            if ($existing) {{
                $deleteResult = $existing.Delete()
                Write-Host "WMI Delete return code: $($deleteResult.ReturnValue)"
                Start-Sleep -Seconds 3
            }}
        }} catch {{
            Write-Host "Could not delete existing service. Continuing..."
            Write-Host $_
        }}
    }} else {{
        Write-Host "No existing PBX service found."
    }}

    Write-Host "Copying PBX files..."

    Get-ChildItem -LiteralPath $SourceDir -Force | ForEach-Object {{
        if (
            $_.Name -eq "install-vodia-pbx.ps1" -or
            $_.Name -eq "run-install-vodia-pbx.cmd" -or
            $_.Name -eq "install-vodia-pbx.log" -or
            $_.Name -eq "installation.txt"
        ) {{
            return
        }}

        Copy-Item -LiteralPath $_.FullName -Destination $InstallDir -Recurse -Force
    }}

    $PbxExe = Join-Path $InstallDir "pbxctrl.exe"
    $PbxDat = Join-Path $InstallDir "pbxctrl.dat"
    $OpusDll = Join-Path $InstallDir "opus.dll"
    $PbxXml = Join-Path $InstallDir "pbx.xml"
    $InstallInfo = Join-Path $InstallDir "installation.txt"
    $StagedInstallInfo = Join-Path $SourceDir "installation.txt"
    $FirstRunOut = Join-Path $InstallDir "pbx-first-run.out.log"
    $FirstRunErr = Join-Path $InstallDir "pbx-first-run.err.log"

    if (!(Test-Path -LiteralPath $PbxExe)) {{
        Write-Host "Files currently in target folder:"
        Get-ChildItem -LiteralPath $InstallDir -Force | Format-Table Name, Length
        throw "pbxctrl.exe was not found after copy: $PbxExe"
    }}

    if (!(Test-Path -LiteralPath $PbxDat)) {{
        throw "pbxctrl.dat was not found after copy."
    }}

    if (!(Test-Path -LiteralPath $OpusDll)) {{
        throw "opus.dll was not found after copy."
    }}

Write-Host ""
Write-Host "Preparing first-run setup.json..."

$AdminUser = "vodia"
$AdminPassword = [guid]::NewGuid().ToString()

$Md5 = [System.Security.Cryptography.MD5]::Create()
$PasswordBytes = [System.Text.Encoding]::UTF8.GetBytes($AdminPassword)
$HashBytes = $Md5.ComputeHash($PasswordBytes)
$PasswordHash = ([System.BitConverter]::ToString($HashBytes)).Replace("-", "").ToLowerInvariant()

$SetupJsonPath = Join-Path $InstallDir "setup.json"

$SetupJsonContent = @"
{{
  "settings": {{
    "activation_key": "",
    "pw_pass": "$PasswordHash",
    "pw_user": "$AdminUser",
    "sys_name": "Vodia PBX",
    "email_global": "cloud",
    "email_list_performance": "pbx@vodia.com"
    }}
}}
"@

$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($SetupJsonPath, $SetupJsonContent, $Utf8NoBom)

Write-Host "setup.json written to:"
Write-Host $SetupJsonPath
Write-Host "Admin username: $AdminUser"
Write-Host "Generated admin password: $AdminPassword"
Write-Host "MD5 password hash written to setup.json: $PasswordHash"

Write-Host ""
Write-Host "Initializing Vodia PBX configuration..."
Write-Host "This first run should consume setup.json and create pbx.xml."

$FirstRunArgString = "--no-daemon --dir `"$InstallDir`""

    Write-Host "Starting foreground PBX initialization..."
    Write-Host "Output log: $FirstRunOut"
    Write-Host "Error log:  $FirstRunErr"

    $firstRunProcess = Start-Process `
        -FilePath $PbxExe `
        -ArgumentList $FirstRunArgString `
        -WorkingDirectory $InstallDir `
        -RedirectStandardOutput $FirstRunOut `
        -RedirectStandardError $FirstRunErr `
        -PassThru `
        -WindowStyle Hidden

    Start-Sleep -Seconds 15

    if (!$firstRunProcess.HasExited) {{
        Write-Host "Stopping foreground PBX initialization process..."
        Stop-Process -Id $firstRunProcess.Id -Force
        Start-Sleep -Seconds 2
    }} else {{
        Write-Host "Foreground PBX initialization process exited with code $($firstRunProcess.ExitCode)."
    }}

    if (Test-Path -LiteralPath $FirstRunOut) {{
        Write-Host ""
        Write-Host "Foreground PBX stdout:"
        Get-Content -LiteralPath $FirstRunOut | Write-Host
    }}

    if (Test-Path -LiteralPath $FirstRunErr) {{
        Write-Host ""
        Write-Host "Foreground PBX stderr:"
        Get-Content -LiteralPath $FirstRunErr | Write-Host
    }}

    if (!(Test-Path -LiteralPath $PbxXml)) {{
        throw "pbx.xml was not created during first-run initialization."
    }}

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

    $DisplayPassword = $AdminPassword
    if ($DisplayPassword -eq "") {{
        $DisplayPassword = "<blank>"
    }}

    Write-Host "Vodia first-run initialization completed."
    Write-Host "Detected login username: $AdminUser"
    Write-Host "Detected login password: $DisplayPassword"

    Write-Host ""
    Write-Host "Creating Windows service..."

    $BinPath = "`"$PbxExe`" --dir `"$InstallDir`""

    New-Service `
        -Name $ServiceName `
        -BinaryPathName $BinPath `
        -DisplayName $DisplayName `
        -StartupType Automatic

    Start-Sleep -Seconds 2

    $service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if (!$service) {{
        throw "Service was not created: $ServiceName"
    }}

    Write-Host "Adding Windows Firewall rule if needed..."

    try {{
        $rule = Get-NetFirewallRule -DisplayName "Vodia PBX" -ErrorAction SilentlyContinue

        if (!$rule) {{
            New-NetFirewallRule `
                -DisplayName "Vodia PBX" `
                -Direction Inbound `
                -Program $PbxExe `
                -Action Allow `
                -Profile Any | Out-Null
        }}
    }} catch {{
        Write-Host "Firewall rule step failed, but install will continue:"
        Write-Host $_
    }}

    Write-Host "Starting PBX service..."
    Start-Service -Name $ServiceName

    Start-Sleep -Seconds 5

    Write-Host "Service status:"
    Get-Service -Name $ServiceName | Format-Table Name, Status, DisplayName

    $VersionText = "Unknown"
    try {{
        $VersionOutput = & $PbxExe --version 2>&1
        if ($VersionOutput) {{
            $VersionText = ($VersionOutput | Out-String).Trim()
        }}
    }} catch {{
        $VersionText = "Unknown"
    }}

    $IpAddresses = @()
    try {{
        $IpAddresses = Get-NetIPAddress -AddressFamily IPv4 |
            Where-Object {{
                $_.IPAddress -ne "127.0.0.1" -and
                $_.IPAddress -notlike "169.254.*" -and
                $_.AddressState -eq "Preferred"
            }} |
            Select-Object -ExpandProperty IPAddress
    }} catch {{
        $IpAddresses = @()
    }}

    $UrlLines = New-Object System.Collections.Generic.List[string]
    $UrlLines.Add("  http://127.0.0.1")

    foreach ($ip in $IpAddresses) {{
        $UrlLines.Add("  http://$ip")
    }}

    $InstallText = @"
=== Vodia PBX Installation Complete ===
PBX Directory: $InstallDir
Version: $VersionText
Architecture: Windows x64

Your PBX URL is:
$($UrlLines -join "`r`n")

You can access the PBX web interface with:
Username: $AdminUser
Password: $DisplayPassword

Important notes:
- These credentials were generated by this installer and written to setup.json before first startup.
- The password was written to setup.json as an MD5 hash.
- Change this password after first login if required by your security policy.
- If this machine is remote, make sure Windows Firewall and cloud firewall/security groups allow HTTP/HTTPS access.
- Local service name: $ServiceName
- Install log: $LogPath
- First-run stdout log: $FirstRunOut
- First-run stderr log: $FirstRunErr

=== Enhanced Vodia PBX Setup Complete ===
Monitoring enabled for:
  - Windows service: $ServiceName
  - Service startup: Automatic
Automated backup enabled:
  - Not configured by this Windows wizard yet

To restore from backup in the future:
  Restore the PBX working directory and recreate the PBX service.
"@

    Set-Content -LiteralPath $InstallInfo -Value $InstallText -Encoding UTF8
    Set-Content -LiteralPath $StagedInstallInfo -Value $InstallText -Encoding UTF8

    Write-Host ""
    Write-Host $InstallText

    Write-Host ""
    Write-Host "Installation info written to:"
    Write-Host $InstallInfo
    Write-Host $StagedInstallInfo

    Write-Host ""
    Write-Host "Vodia PBX installation completed."
    Write-Host "Installed to: $InstallDir"
}}
catch {{
    Write-Host ""
    Write-Host "INSTALL FAILED:" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    Write-Host ""
    Write-Host "Full error:"
    Write-Host $_
}}
finally {{
    try {{
        Stop-Transcript | Out-Null
    }} catch {{}}

    Write-Host ""
    Write-Host "Log file:"
    Write-Host $LogPath
    Write-Host ""
    Write-Host "Press Enter to close this PowerShell section."
    Read-Host
}}
"#,
        source_dir = escape_powershell_string(staged_dir),
        install_dir = escape_powershell_string(install_dir),
        log_path = escape_powershell_string(&log_path),
    );

    fs::write(&script_path, script)?;

    let cmd = format!(
        r#"@echo off
title Vodia PBX Installer
echo Running Vodia PBX installer...
echo.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "{script_path}"
echo.
echo PowerShell exited with code %ERRORLEVEL%.
echo.
echo This CMD window is intentionally left open for debugging.
echo Log file:
echo {log_path}
echo.
pause
"#,
        script_path = script_path.display(),
        log_path = log_path.display(),
    );

    fs::write(&cmd_path, cmd)?;

    let cmd_exe = PathBuf::from(r"C:\Windows\System32\cmd.exe");
    let args = format!(r#"/k "{}""#, cmd_path.display());

    crate::windows_admin::run_elevated_with_args(&cmd_exe, &args)?;

    println!("Elevated installer command launched.");
    println!("Generated installer script:");
    println!("{}", script_path.display());
    println!("Generated installer CMD:");
    println!("{}", cmd_path.display());

    Ok(())
}

fn escape_powershell_string(path: &Path) -> String {
    path.display().to_string().replace('`', "``").replace('"', "`\"")
}