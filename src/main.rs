#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gui;
mod installer;
mod manifest_installer;
mod releases;
mod setup_runner;
mod windows_admin;
<<<<<<< HEAD
mod uninstaller;
=======
>>>>>>> 5e82ce2cfacd3d7127f5cf438cff8ece980bdd3c

use anyhow::{bail, Result};
use releases::TargetPlatform;
use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Debug, Clone)]
struct AppOptions {
    version: String,
    target: TargetPlatform,
    prefer_setup_package: bool,
    install_after_download: bool,
    install_dir: PathBuf,
    assume_yes: bool,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

<<<<<<< HEAD
    if args.iter().any(|arg| arg == "--uninstall") {
        return crate::uninstaller::uninstall_vodia_pbx(None);
    }

=======
>>>>>>> 5e82ce2cfacd3d7127f5cf438cff8ece980bdd3c
    if args.is_empty() || args.iter().any(|arg| arg == "--gui") {
        return gui::run_gui();
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(run_cli())
}

async fn run_cli() -> Result<()> {
    println!("Vodia PBX Installer Wizard");
    println!("--------------------------");

    let options = get_options_from_args_or_prompt()?;

    if options.version.trim().is_empty() {
        bail!("No version was provided.");
    }

    println!();
    println!("Selected version: {}", options.version);
    println!("Target platform: {}", options.target.display_name());
    println!("Install location: {}", options.install_dir.display());

    if options.install_after_download {
        println!("Install mode: ON");
        println!("After staging files, the wizard will launch an elevated install step.");
    } else {
        println!("Install mode: OFF");
        println!("The wizard will stage/download files only.");
        println!("Use --install to install in the same step.");
    }

    if options.target == TargetPlatform::Windows64 && options.prefer_setup_package {
        println!("This will check for a Windows package first: ZIP, TAR, TGZ, or EXE.");
        println!("If no full setup package is found, it will use the XML manifest.");
    } else {
        println!("This will use XML manifest mode.");
    }

    println!();

    if !options.assume_yes {
        print!("Continue? [y/N]: ");
        io::stdout().flush()?;

        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm)?;

        let confirm = confirm.trim().to_ascii_lowercase();

        if confirm != "y" && confirm != "yes" {
            println!("Cancelled.");
            return Ok(());
        }
    }

    println!();
    println!("Starting Vodia download flow...");
    println!();

    setup_runner::run_vodia_setup(
        &options.version,
        options.target,
        options.prefer_setup_package,
        options.install_after_download,
        &options.install_dir,
    )
    .await?;

    println!();
    println!("Done.");

    Ok(())
}

fn get_options_from_args_or_prompt() -> Result<AppOptions> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        std::process::exit(0);
    }

    let mut version: Option<String> = None;
    let mut target = TargetPlatform::Windows64;
    let mut prefer_setup_package = true;
    let mut install_after_download = false;
    let mut install_dir = default_install_dir();
    let mut assume_yes = false;

    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];

        match arg.as_str() {
            "--target" => {
                index += 1;

                if index >= args.len() {
                    bail!("Missing value after --target. Use windows or macos.");
                }

                target = TargetPlatform::from_user_value(&args[index])?;
            }

            "--install-dir" => {
                index += 1;

                if index >= args.len() {
                    bail!("Missing value after --install-dir.");
                }

                install_dir = PathBuf::from(&args[index]);
            }

            "--windows" | "--win64" => {
                target = TargetPlatform::Windows64;
            }

            "--macos" | "--mac" => {
                target = TargetPlatform::MacOS;
            }

            "--no-setup" => {
                prefer_setup_package = false;
            }

            "--install" => {
                install_after_download = true;
            }

            "--yes" | "-y" => {
                assume_yes = true;
            }

            "--gui" => {
                // Handled before CLI flow starts.
            }

            value if value.starts_with('-') => {
                bail!("Unknown option: {value}");
            }

            value => {
                if version.is_some() {
                    bail!("Too many version values were provided.");
                }

                version = Some(value.trim().to_string());
            }
        }

        index += 1;
    }

    let version = match version {
        Some(value) => value,
        None => prompt_for_version()?,
    };

    Ok(AppOptions {
        version,
        target,
        prefer_setup_package,
        install_after_download,
        install_dir,
        assume_yes,
    })
}

fn prompt_for_version() -> Result<String> {
    print!("Enter Vodia PBX version, for example 68.0.37: ");
    io::stdout().flush()?;

    let mut version = String::new();
    io::stdin().read_line(&mut version)?;

    Ok(version.trim().to_string())
}

fn default_install_dir() -> PathBuf {
    PathBuf::from(r"C:\Program Files\Vodia\PBX")
}

fn print_usage() {
    println!("Usage:");
    println!("  vodia-pbx-wizard.exe <version> [options]");
    println!();
    println!("Options:");
    println!("  --gui                 Launch graphical installer.");
    println!("  --target windows      Download Windows 64-bit files. This is the default.");
    println!("  --target macos        Download MacOS files from the manifest.");
    println!("  --windows             Same as --target windows.");
    println!("  --macos               Same as --target macos.");
    println!("  --no-setup            Skip ZIP/TAR/EXE package probing and use XML manifest only.");
    println!("  --install             After staging files, launch elevated install step.");
    println!("  --install-dir <path>  Install location. Default: C:\\Program Files\\Vodia\\PBX");
    println!("  --yes, -y             Do not ask for confirmation.");
    println!();
    println!("Examples:");
    println!("  vodia-pbx-wizard.exe --gui");
    println!("  vodia-pbx-wizard.exe 68.0.37 --install");
    println!("  vodia-pbx-wizard.exe 68.0.37 --install --install-dir \"C:\\Program Files\\Vodia\\PBX\"");
    println!("  vodia-pbx-wizard.exe 68.0.37 --no-setup");
    println!("  vodia-pbx-wizard.exe 68.0.4 --target macos --no-setup");
}