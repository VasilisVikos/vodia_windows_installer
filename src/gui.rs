use eframe::egui;
use regex::Regex;
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
    time::Duration,
};

const DEFAULT_INSTALL_DIR: &str = r"C:\Program Files\Vodia\PBX";
const LATEST_VERSION_URL: &str = "https://cdn.vodia.net/builds/latest.txt";
const FALLBACK_VERSION: &str = "68.0.37";

pub fn run_gui() -> anyhow::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([760.0, 540.0])
        .with_min_inner_size([640.0, 420.0]);

    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Vodia PBX Installer Wizard",
        native_options,
        Box::new(|cc| Ok(Box::new(VodiaInstallerGui::new(&cc.egui_ctx)))),
    )
    .map_err(|error| anyhow::anyhow!("GUI failed: {error}"))
}

struct VodiaInstallerGui {
    logo_texture: Option<egui::TextureHandle>,

    version_input: String,
    latest_version: Option<String>,
    version_manually_edited: bool,

    install_dir: String,
    install_running: bool,
    log: String,

    receiver: Option<Receiver<GuiMessage>>,
    latest_receiver: Option<Receiver<LatestMessage>>,
}

enum GuiMessage {
    Log(String),
    Done(i32),
    Failed(String),
}

enum LatestMessage {
    Loaded(String),
    Failed,
}

impl VodiaInstallerGui {
    fn new(ctx: &egui::Context) -> Self {
        let mut app = Self {
            logo_texture: load_logo_texture(ctx),

            version_input: FALLBACK_VERSION.to_string(),
            latest_version: None,
            version_manually_edited: false,

            install_dir: DEFAULT_INSTALL_DIR.to_string(),
            install_running: false,
            log: String::new(),

            receiver: None,
            latest_receiver: None,
        };

        app.refresh_latest_version();

        app
    }
    fn start_uninstall(&mut self) {
        if self.install_running {
            return;
        }
    
        self.install_running = true;
        self.log.clear();
        self.log.push_str("Starting Vodia PBX uninstaller...\n");
    
        let exe_path = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                self.install_running = false;
                self.log
                    .push_str(&format!("Could not locate current executable: {error}\n"));
                return;
            }
        };
    
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
    
        thread::spawn(move || {
            let _ = tx.send(GuiMessage::Log(
                "Starting Vodia PBX uninstaller...\n".to_string(),
            ));
    
            let mut child = match Command::new(exe_path)
                .arg("--uninstall")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(error) => {
                    let _ = tx.send(GuiMessage::Failed(format!(
                        "Failed to start uninstaller: {error}\n"
                    )));
                    return;
                }
            };
    
            if let Some(stdout) = child.stdout.take() {
                let tx_stdout = tx.clone();
    
                thread::spawn(move || {
                    let reader = BufReader::new(stdout);
    
                    for line in reader.lines().flatten() {
                        let _ = tx_stdout.send(GuiMessage::Log(format!("{line}\n")));
                    }
                });
            }
    
            if let Some(stderr) = child.stderr.take() {
                let tx_stderr = tx.clone();
    
                thread::spawn(move || {
                    let reader = BufReader::new(stderr);
    
                    for line in reader.lines().flatten() {
                        let _ = tx_stderr.send(GuiMessage::Log(format!("{line}\n")));
                    }
                });
            }
    
            match child.wait() {
                Ok(status) if status.success() => {
                    let _ = tx.send(GuiMessage::Log("Uninstaller completed.\n".to_string()));
                    let _ = tx.send(GuiMessage::Done(0));
                }
                Ok(status) => {
                    let _ = tx.send(GuiMessage::Failed(format!(
                        "Uninstaller exited with status: {status}\n"
                    )));
                }
                Err(error) => {
                    let _ = tx.send(GuiMessage::Failed(format!(
                        "Failed waiting for uninstaller: {error}\n"
                    )));
                }
            }
        });
    }
}

impl eframe::App for VodiaInstallerGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_messages();
        self.drain_latest_messages();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(texture) = &self.logo_texture {
                    let original_size = texture.size_vec2();

                    if original_size.y > 0.0 {
                        let target_height = 44.0;
                        let scale = target_height / original_size.y;
                        let display_size = egui::vec2(original_size.x * scale, target_height);

                        ui.image((texture.id(), display_size));
                    }
                }

                ui.add_space(8.0);
                ui.heading("Vodia PBX Installer Wizard");
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.label("Version:");

                let version_response = ui.add_enabled(
                    !self.install_running,
                    egui::TextEdit::singleline(&mut self.version_input).desired_width(160.0),
                );

                if version_response.changed() {
                    self.version_manually_edited = true;
                }

                if ui
                    .add_enabled(!self.install_running, egui::Button::new("Use Latest"))
                    .clicked()
                {
                    self.version_manually_edited = false;

                    if let Some(latest) = &self.latest_version {
                        self.version_input = latest.clone();
                    }

                    self.refresh_latest_version();
                }
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.label("Install location:");

                ui.add_enabled(
                    !self.install_running,
                    egui::TextEdit::singleline(&mut self.install_dir).desired_width(450.0),
                );

                if ui
                    .add_enabled(!self.install_running, egui::Button::new("Browse..."))
                    .clicked()
                {
                    if let Some(folder) = rfd::FileDialog::new()
                        .set_directory(&self.install_dir)
                        .pick_folder()
                    {
                        self.install_dir = folder.display().to_string();
                    }
                }
            });

            ui.add_space(14.0);

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.install_running, egui::Button::new("Install"))
                    .clicked()
                {
                    self.start_backend(true);
                }
            
                if ui
                    .add_enabled(!self.install_running, egui::Button::new("Download Only"))
                    .clicked()
                {
                    self.start_backend(false);
                }
            
                if ui
                    .add_enabled(!self.install_running, egui::Button::new("Uninstall"))
                    .clicked()
                {
                    self.start_uninstall();
                }
            
                if ui
                    .add_enabled(!self.install_running, egui::Button::new("Clear Log"))
                    .clicked()
                {
                    self.log.clear();
                }
            });

            ui.add_space(8.0);

            if self.install_running {
                ui.label("Status: Running...");
            } else {
                ui.label("Status: Ready");
            }

            ui.separator();

            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.log)
                            .desired_width(f32::INFINITY)
                            .desired_rows(22)
                            .font(egui::TextStyle::Monospace)
                            .interactive(false),
                    );
                });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Open Install Folder").clicked() {
                    open_folder(&self.install_dir);
                }

                if ui.button("Open installation.txt").clicked() {
                    let path = PathBuf::from(&self.install_dir).join("installation.txt");
                    open_file(&path);
                }
            });
        });

        if self.install_running || self.latest_receiver.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

impl VodiaInstallerGui {
    fn refresh_latest_version(&mut self) {
        let (sender, receiver) = mpsc::channel::<LatestMessage>();
        self.latest_receiver = Some(receiver);

        thread::spawn(move || match fetch_latest_version() {
            Ok(version) => {
                let _ = sender.send(LatestMessage::Loaded(version));
            }

            Err(_error) => {
                let _ = sender.send(LatestMessage::Failed);
            }
        });
    }

    fn start_backend(&mut self, install: bool) {
        self.log.clear();

        let requested_version = self.version_input.clone();
        let use_latest = !self.version_manually_edited;
        let install_dir = self.install_dir.clone();

        if self.version_manually_edited {
            if let Err(error) = normalize_version(&requested_version) {
                self.log.push_str("Cannot start installer.\n");
                self.log.push_str(&format!("{error}\n"));
                return;
            }
        }

        self.install_running = true;

        let (sender, receiver) = mpsc::channel::<GuiMessage>();
        self.receiver = Some(receiver);

        thread::spawn(move || {
            let version = if use_latest {
                let _ = sender.send(GuiMessage::Log(
                    "Checking latest Vodia PBX version...".to_string(),
                ));

                match fetch_latest_version() {
                    Ok(version) => {
                        let _ = sender.send(GuiMessage::Log(format!(
                            "Using latest Vodia PBX version: v{version}"
                        )));

                        version
                    }

                    Err(error) => {
                        let _ = sender.send(GuiMessage::Failed(format!(
                            "Could not fetch latest Vodia PBX version: {error}"
                        )));
                        return;
                    }
                }
            } else {
                match normalize_version(&requested_version) {
                    Ok(version) => {
                        let _ = sender.send(GuiMessage::Log(format!(
                            "Using custom Vodia PBX version: v{version}"
                        )));

                        version
                    }

                    Err(error) => {
                        let _ = sender.send(GuiMessage::Failed(error));
                        return;
                    }
                }
            };

            let exe = match std::env::current_exe() {
                Ok(path) => path,

                Err(error) => {
                    let _ = sender.send(GuiMessage::Failed(format!(
                        "Could not find current executable: {error}"
                    )));
                    return;
                }
            };

            let mut command = Command::new(exe);

            command.arg(version);
            command.arg("--yes");

            if install {
                command.arg("--install");
            }

            command.arg("--install-dir");
            command.arg(install_dir);

            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());

            let mut child = match command.spawn() {
                Ok(child) => child,

                Err(error) => {
                    let _ = sender.send(GuiMessage::Failed(format!(
                        "Failed to start installer backend: {error}"
                    )));
                    return;
                }
            };

            if let Some(stdout) = child.stdout.take() {
                let sender_clone = sender.clone();

                thread::spawn(move || {
                    let reader = BufReader::new(stdout);

                    for line in reader.lines().map_while(Result::ok) {
                        let _ = sender_clone.send(GuiMessage::Log(line));
                    }
                });
            }

            if let Some(stderr) = child.stderr.take() {
                let sender_clone = sender.clone();

                thread::spawn(move || {
                    let reader = BufReader::new(stderr);

                    for line in reader.lines().map_while(Result::ok) {
                        let _ = sender_clone.send(GuiMessage::Log(line));
                    }
                });
            }

            match child.wait() {
                Ok(status) => {
                    let code = status.code().unwrap_or(-1);
                    let _ = sender.send(GuiMessage::Done(code));
                }

                Err(error) => {
                    let _ = sender.send(GuiMessage::Failed(format!(
                        "Backend wait failed: {error}"
                    )));
                }
            }
        });
    }

    fn drain_messages(&mut self) {
        let mut finished = false;

        if let Some(receiver) = &self.receiver {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    GuiMessage::Log(line) => {
                        self.log.push_str(&line);
                        self.log.push('\n');
                    }

                    GuiMessage::Done(code) => {
                        if code == 0 {
                            self.log.push_str("\nCompleted successfully.\n");
                        } else {
                            self.log.push_str(&format!(
                                "\nInstaller exited with error code {code}.\n"
                            ));
                        }

                        finished = true;
                    }

                    GuiMessage::Failed(error) => {
                        self.log.push_str(&format!("\nERROR: {error}\n"));
                        finished = true;
                    }
                }
            }
        }

        if finished {
            self.install_running = false;
            self.receiver = None;
        }
    }

    fn drain_latest_messages(&mut self) {
        let mut finished = false;

        if let Some(receiver) = &self.latest_receiver {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    LatestMessage::Loaded(version) => {
                        self.latest_version = Some(version.clone());

                        if !self.version_manually_edited {
                            self.version_input = version;
                        }

                        finished = true;
                    }

                    LatestMessage::Failed => {
                        if !self.version_manually_edited {
                            self.version_input = FALLBACK_VERSION.to_string();
                        }

                        finished = true;
                    }
                }
            }
        }

        if finished {
            self.latest_receiver = None;
        }
    }
}

fn fetch_latest_version() -> anyhow::Result<String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("VodiaPBXWizard/1.0")
        .timeout(Duration::from_secs(15))
        .build()?;

    let body = client
        .get(LATEST_VERSION_URL)
        .send()?
        .error_for_status()?
        .text()?;

    extract_first_version(&body)
        .ok_or_else(|| anyhow::anyhow!("No valid version number found in latest.txt"))
}

fn extract_first_version(text: &str) -> Option<String> {
    let version_regex =
        Regex::new(r"\bv?(\d{2}\.\d+(?:\.\d+)?)\b").expect("version regex should compile");

    let captures = version_regex.captures(text)?;
    let version = captures.get(1)?.as_str();

    Some(version.to_string())
}

fn normalize_version(input: &str) -> Result<String, String> {
    let trimmed = input.trim();

    let version_regex =
        Regex::new(r"^v?(\d{2}\.\d+(?:\.\d+)?)$").expect("version regex should compile");

    let captures = version_regex.captures(trimmed).ok_or_else(|| {
        "Invalid version. Use a version like 70.5, 69.5.22, or 68.0.37.".to_string()
    })?;

    let version = captures
        .get(1)
        .map(|value| value.as_str().to_string())
        .ok_or_else(|| {
            "Invalid version. Use a version like 70.5, 69.5.22, or 68.0.37.".to_string()
        })?;

    Ok(version)
}

fn load_logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let bytes = include_bytes!("../assets/vodia-logo.png");

    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_raw();

    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);

    Some(ctx.load_texture(
        "vodia_logo",
        color_image,
        egui::TextureOptions::LINEAR,
    ))
}

fn load_window_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/favicon.png");

    let image = image::load_from_memory(bytes).ok()?.to_rgba8();

    Some(egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}

fn open_folder(path: &str) {
    #[cfg(windows)]
    {
        let _ = Command::new("explorer.exe").arg(path).spawn();
    }
}

fn open_file(path: &PathBuf) {
    #[cfg(windows)]
    {
        if path.exists() {
            let _ = Command::new("notepad.exe").arg(path).spawn();
        }
    }
}