mod config;

use eframe::epaint::Color32;
use eframe::Frame;
use egui::{
    menu, Button, Key, KeyboardShortcut, Modifiers, TextEdit, TopBottomPanel, Widget, Window,
};
use jgb_core::{EmulationError, InputConfig, RunConfig};
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

pub use config::AppConfig;

#[derive(Debug, Default)]
struct AppState {
    running_emulator: Option<EmulatorInstance>,
    settings_open: bool,
    window_width_text: String,
    window_width_invalid: bool,
    window_height_text: String,
    window_height_invalid: bool,
}

impl AppState {
    fn from_config(app_config: &AppConfig) -> Self {
        Self {
            window_width_text: app_config.window_width.to_string(),
            window_height_text: app_config.window_height.to_string(),
            ..Self::default()
        }
    }

    fn is_emulator_running(&self) -> bool {
        match &self.running_emulator {
            Some(running_emulator) => !running_emulator.thread.is_finished(),
            None => false,
        }
    }
}

#[derive(Debug, Default)]
pub struct JgbApp {
    config: AppConfig,
    config_path: PathBuf,
    state: AppState,
}

impl JgbApp {
    pub fn new(config: AppConfig, config_path: PathBuf) -> Self {
        let state = AppState::from_config(&config);
        Self {
            config,
            config_path,
            state,
        }
    }

    fn handle_open(&mut self) {
        let file = FileDialog::new().add_filter("gb", &["gb"]).pick_file();

        if let Some(file) = file.and_then(|file| file.to_str().map(String::from)) {
            self.stop_emulator_if_running();

            self.state.running_emulator = Some(launch_emulator(&file, &self.config));
        }
    }

    fn stop_emulator_if_running(&mut self) {
        if let Some(running_emulator) = self.state.running_emulator.take() {
            log::info!("Shutting down existing emulator instance");

            *running_emulator.quit_signal.lock().unwrap() = true;

            // TODO actually handle errors
            running_emulator.thread.join().unwrap().unwrap();
        }
    }

    fn save_config(&self) {
        // TODO actually handle errors
        self.config.save_to_file(&self.config_path).unwrap();
    }
}

impl eframe::App for JgbApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        let prev_config = self.config.clone();

        let open_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::O);
        if ctx.input_mut(|input| input.consume_shortcut(&open_shortcut)) {
            self.handle_open();
        }

        let quit_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Q);
        if ctx.input_mut(|input| input.consume_shortcut(&quit_shortcut)) {
            frame.close();
        }

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.set_enabled(!self.state.settings_open && !self.state.is_emulator_running());

            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    let open_button = Button::new("Open GB ROM")
                        .shortcut_text(ctx.format_shortcut(&open_shortcut))
                        .ui(ui);
                    if open_button.clicked() {
                        self.handle_open();
                        ui.close_menu();
                    }

                    let quit_button = Button::new("Quit")
                        .shortcut_text(ctx.format_shortcut(&quit_shortcut))
                        .ui(ui);
                    if quit_button.clicked() {
                        frame.close();
                    }
                });

                ui.menu_button("Options", |ui| {
                    if ui.button("Settings").clicked() {
                        self.state.settings_open = true;
                        ui.close_menu();
                    }
                });
            });
        });

        if self.state.settings_open {
            // Create a temp bool to pass to open() because we can't modify self.state.settings_open
            // if it is mutably borrowed by the window
            let mut settings_open = true;
            Window::new("Settings")
                .id("settings".into())
                .open(&mut settings_open)
                .show(ctx, |ui| {
                    ui.checkbox(&mut self.config.vsync_enabled, "VSync enabled");

                    ui.checkbox(&mut self.config.audio_enabled, "Audio enabled");

                    ui.checkbox(&mut self.config.audio_sync_enabled, "Sync emulation speed to audio");

                    ui.checkbox(&mut self.config.audio_60hz_hack_enabled, "Audio 60Hz hack enabled")
                        .on_hover_text("Very slightly increases audio frequency to time audio to 60Hz display speed instead of ~59.7Hz");

                    ui.horizontal(|ui| {
                        if !TextEdit::singleline(&mut self.state.window_width_text)
                            .id("window_width".into())
                            .desired_width(60.0)
                            .ui(ui)
                            .has_focus()
                        {
                            match self.state.window_width_text.parse::<u32>() {
                                Ok(window_width) => {
                                    self.config.window_width = window_width;
                                    self.state.window_width_invalid = false;
                                }
                                Err(_) => {
                                    self.state.window_width_invalid = true;
                                }
                            }
                        }
                        ui.label("Window width in pixels");
                    });
                    if self.state.window_width_invalid {
                        ui.colored_label(Color32::RED, "Window width is not a valid number");
                    }

                    ui.horizontal(|ui| {
                        if !TextEdit::singleline(&mut self.state.window_height_text)
                            .id("window_height".into())
                            .desired_width(60.0)
                            .ui(ui)
                            .has_focus()
                        {
                            match self.state.window_height_text.parse::<u32>() {
                                Ok(window_height) => {
                                    self.config.window_height = window_height;
                                    self.state.window_height_invalid = false;
                                }
                                Err(_) => {
                                    self.state.window_height_invalid = true;
                                }
                            }
                        }
                        ui.label("Window height in pixels");
                    });
                    if self.state.window_height_invalid {
                        ui.colored_label(Color32::RED, "Window height is not a valid number");
                    }

                    if ui.button("Close").clicked() {
                        self.state.settings_open = false;
                    }
                });
            self.state.settings_open &= settings_open;
        }

        if prev_config != self.config {
            // Save config immediately on changes
            self.save_config();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop_emulator_if_running();
    }
}

#[derive(Debug)]
struct EmulatorInstance {
    thread: JoinHandle<Result<(), EmulationError>>,
    quit_signal: Arc<Mutex<bool>>,
}

#[must_use]
fn launch_emulator(gb_file: &str, app_config: &AppConfig) -> EmulatorInstance {
    log::info!("Launching emulator instance for file path '{gb_file}'");

    // TODO actually make input configurable
    let run_config = RunConfig {
        gb_file_path: gb_file.into(),
        audio_enabled: app_config.audio_enabled,
        sync_to_audio: app_config.audio_sync_enabled,
        vsync_enabled: app_config.vsync_enabled,
        window_width: app_config.window_width,
        window_height: app_config.window_height,
        audio_debugging_enabled: false,
        audio_60hz: app_config.audio_60hz_hack_enabled,
        input_config: InputConfig::default(),
    };

    let quit_signal = Arc::new(Mutex::new(false));

    let quit_signal_clone = Arc::clone(&quit_signal);
    let thread = thread::spawn(move || {
        let result = jgb_core::run(&run_config, quit_signal_clone);
        if let Err(err) = &result {
            log::error!("Emulator terminated unexpectedly: {err}");
        }
        result
    });

    EmulatorInstance {
        thread,
        quit_signal,
    }
}
