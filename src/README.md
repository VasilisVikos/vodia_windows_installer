# Vodia PBX Installer Wizard
A Windows installer wizard for downloading and installing Vodia PBX builds.

by Vasilis Vikos

## Features

- Native Windows GUI
- Fetches the latest Vodia PBX version when installing
- Allows manual version override, for example `70.5`, `69.5.22`, or `68.0.37`
- Downloads required PBX files from the Vodia manifest
- Installs PBX files into a selected folder
- Creates and starts the Windows service
- Generates first-run admin credentials through `setup.json`
- Writes installation details to `installation.txt`

## Default install location

``` C:\Program Files\Vodia\PBX ```

Usage

Download the latest release .exe from the GitHub Releases page and run it as Administrator if prompted.

The installer downloads Vodia PBX files during installation.

Build from source

Install Rust:

``` winget install Rustlang.Rustup ```

Build:

```rust cargo build --release ```

Run GUI:

.\target\release\vodia-pbx-wizard.exe

Run CLI:

.\target\release\vodia-pbx-wizard.exe 70.5 --install --install-dir "C:\Program Files\Vodia\PBX"

Notes

This installer creates a Windows service named PBX.

Uninstall: 

``` Stop-Service PBX -ErrorAction SilentlyContinue ```
``` sc.exe delete PBX ```
``` Start-Sleep -Seconds 3 ```
``` Remove-Item -Recurse -Force "C:\Program Files\Vodia\PBX" -ErrorAction SilentlyContinue ```

Security

The installer performs administrative installation tasks, including creating a service and adding firewall rules. Some antivirus tools may flag unsigned builds. For production distribution, code signing is recommended.

License

```text
MIT License

Copyright (c) 2026 Kyros Voice

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```