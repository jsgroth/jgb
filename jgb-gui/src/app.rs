mod config;
mod input;

use eframe::epaint::Color32;
use eframe::Frame;
use egui::{
    menu, Align, Button, CentralPanel, Context, Direction, Key, KeyboardShortcut, Layout,
    Modifiers, TextEdit, TopBottomPanel, Widget, Window,
};
use egui_extras::{Column, TableBuilder};
use jgb_core::{ControllerConfig, EmulationError, RunConfig};
use rfd::FileDialog;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::{fs, thread};

use crate::app::config::FullscreenMode;
use crate::app::input::{HotkeySettingsWidget, InputSettingsWidget, KeyInputThread};
pub use config::AppConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenSettingsWindow {
    General,
    Input,
    Hotkey,
}

#[derive(Debug, Clone)]
struct RomSearchResult {
    full_path: String,
    file_name_no_ext: String,
    file_size_kb: u64,
}

#[derive(Debug, Default)]
struct AppState {
    running_emulator: Option<EmulatorInstance>,
    open_settings_window: Option<OpenSettingsWindow>,
    key_input_thread: Option<KeyInputThread>,
    emulation_error: Option<EmulationError>,
    window_width_text: String,
    window_width_invalid: bool,
    window_height_text: String,
    window_height_invalid: bool,
    rom_search_results: Vec<RomSearchResult>,
}

impl AppState {
    fn from_config(app_config: &AppConfig) -> Self {
        let mut state = Self {
            window_width_text: app_config.window_width.to_string(),
            window_height_text: app_config.window_height.to_string(),
            ..Self::default()
        };

        state.refresh_rom_search_results(app_config.rom_search_dir.as_ref());

        state
    }

    fn is_emulator_running(&self) -> bool {
        match &self.running_emulator {
            Some(running_emulator) => !running_emulator.thread.is_finished(),
            None => false,
        }
    }

    fn refresh_rom_search_results(&mut self, rom_search_dir: Option<&String>) {
        let Some(rom_search_dir) = rom_search_dir
        else {
            self.rom_search_results = Vec::new();
            return;
        };

        let Ok(mut rom_search_results) = fs::read_dir(Path::new(rom_search_dir)).map(|read_dir| {
            read_dir
                .filter_map(Result::ok)
                .filter_map(|dir_entry| {
                    let is_gb_file = dir_entry.path().extension() == Some(OsStr::new("gb"));
                    let Ok(metadata) = dir_entry.metadata() else { return None };

                    if is_gb_file && metadata.is_file() {
                        let full_path = dir_entry.path().to_str()?.into();
                        let file_name_no_ext = dir_entry
                            .path()
                            .with_extension("")
                            .file_name()?
                            .to_str()?
                            .into();
                        let file_size_kb = metadata.len() / 1024;

                        Some(RomSearchResult {
                            full_path,
                            file_name_no_ext,
                            file_size_kb,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        }) else {
            log::error!("Unable to refresh ROM list using path {rom_search_dir}");
            return;
        };

        rom_search_results.sort_by_key(|search_result| search_result.file_name_no_ext.clone());

        self.rom_search_results = rom_search_results;
    }
}

#[derive(Debug, Default)]
pub struct JgbApp {
    config: AppConfig,
    config_path: PathBuf,
    state: AppState,
}

impl JgbApp {
    #[must_use]
    pub fn new(config: AppConfig, config_path: PathBuf) -> Self {
        let state = AppState::from_config(&config);
        Self {
            config,
            config_path,
            state,
        }
    }

    fn handle_open(&mut self) {
        let mut file_dialog = FileDialog::new().add_filter("gb", &["gb"]);
        if let Some(rom_search_dir) = &self.config.rom_search_dir {
            file_dialog = file_dialog.set_directory(Path::new(rom_search_dir));
        }
        let file = file_dialog.pick_file();

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

    fn render_error_window(&mut self, ctx: &Context) {
        if let Some(emulation_error) = self
            .state
            .emulation_error
            .as_ref()
            .map(EmulationError::to_string)
        {
            let mut error_open = true;
            Window::new("Error")
                .id("error".into())
                .resizable(false)
                .open(&mut error_open)
                .show(ctx, |ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        ui.label("Emulator terminated with unexpected error");
                        ui.colored_label(Color32::RED, emulation_error);
                    });
                });
            if !error_open {
                self.state.emulation_error = None;
            }
        }
    }

    fn render_menu(&mut self, ctx: &Context, frame: &mut Frame) {
        let open_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::O);
        if ctx.input_mut(|input| input.consume_shortcut(&open_shortcut)) {
            self.handle_open();
        }

        let quit_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Q);
        if ctx.input_mut(|input| input.consume_shortcut(&quit_shortcut)) {
            frame.close();
        }

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.set_enabled(
                self.state.open_settings_window.is_none()
                    && self.state.key_input_thread.is_none()
                    && self.state.emulation_error.is_none(),
            );
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

                ui.set_enabled(!self.state.is_emulator_running());
                ui.menu_button("Options", |ui| {
                    if ui.button("General Settings").clicked() {
                        self.state.open_settings_window = Some(OpenSettingsWindow::General);
                        ui.close_menu();
                    }

                    if ui.button("Input Settings").clicked() {
                        self.state.open_settings_window = Some(OpenSettingsWindow::Input);
                        ui.close_menu();
                    }

                    if ui.button("Hotkey Settings").clicked() {
                        self.state.open_settings_window = Some(OpenSettingsWindow::Hotkey);
                        ui.close_menu();
                    }
                });
            });
        });
    }

    fn render_general_settings_window(&mut self, ctx: &Context) {
        let mut settings_open = true;
        Window::new("General Settings")
            .id("general_settings".into())
            .resizable(false)
            .open(&mut settings_open)
            .show(ctx, |ui| {
                ui.checkbox(&mut self.config.vsync_enabled, "VSync enabled");

                ui.checkbox(&mut self.config.launch_in_fullscreen, "Launch in fullscreen");

                ui.group(|ui| {
                    ui.label("Fullscreen mode");
                    ui.radio_value(&mut self.config.fullscreen_mode, FullscreenMode::Exclusive, "Exclusive");
                    ui.radio_value(&mut self.config.fullscreen_mode, FullscreenMode::Borderless, "Borderless");
                });

                ui.checkbox(&mut self.config.force_integer_scaling, "Force integer scaling")
                    .on_hover_text("Always display emulator output in the highest possible integer scale");

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

                ui.horizontal(|ui| {
                    let search_dir_text = match &self.config.rom_search_dir {
                        Some(rom_search_dir) => rom_search_dir.clone(),
                        None => "<None>".into(),
                    };
                    if ui.button(search_dir_text).clicked() {
                        if let Some(new_search_dir) = FileDialog::new().pick_folder() {
                            if let Some(new_search_dir) = new_search_dir.to_str().map(String::from) {
                                self.config.rom_search_dir = Some(new_search_dir);
                            }
                        }
                    }

                    ui.label("ROM search directory");

                    if ui.button("Clear").clicked() {
                        self.config.rom_search_dir = None;
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    if ui.button("Close").clicked() {
                        self.state.open_settings_window = None;
                    }
                });
            });
        if !settings_open {
            self.state.open_settings_window = None;
        };
    }

    fn render_input_settings_window(&mut self, ctx: &Context) {
        let mut settings_open = true;
        Window::new("Input Settings")
            .id("input_settings".into())
            .resizable(false)
            .open(&mut settings_open)
            .show(ctx, |ui| {
                ui.set_enabled(self.state.key_input_thread.is_none());

                if let Some(thread) = InputSettingsWidget::new(&self.config.input).ui(ui) {
                    self.state.key_input_thread = Some(thread);
                }
            });
        if !settings_open {
            self.state.open_settings_window = None;
        }
    }

    fn render_hotkey_settings_window(&mut self, ctx: &Context) {
        let mut settings_open = true;
        Window::new("Hotkey Settings")
            .id("hotkey_settings".into())
            .resizable(false)
            .open(&mut settings_open)
            .show(ctx, |ui| {
                ui.set_enabled(self.state.key_input_thread.is_none());

                if let Some(thread) = HotkeySettingsWidget::new(&mut self.config.hotkeys).ui(ui) {
                    self.state.key_input_thread = Some(thread);
                }
            });
        if !settings_open {
            self.state.open_settings_window = None;
        }
    }

    fn render_rom_list(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(
                self.state.open_settings_window.is_none() && self.state.key_input_thread.is_none(),
            );

            if self.state.rom_search_results.is_empty() {
                ui.with_layout(
                    Layout::centered_and_justified(Direction::LeftToRight),
                    |ui| {
                        ui.label("Configure a search path to see ROMs here");
                    },
                );
            } else {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    TableBuilder::new(ui)
                        .resizable(false)
                        .auto_shrink([false; 2])
                        .striped(true)
                        .cell_layout(Layout::left_to_right(Align::Center))
                        .column(Column::auto())
                        .column(Column::remainder())
                        .header(30.0, |mut row| {
                            row.col(|ui| {
                                ui.heading("Name");
                            });
                            row.col(|ui| {
                                ui.heading("Size");
                            });
                        })
                        .body(|mut body| {
                            for search_result in self.state.rom_search_results.clone() {
                                body.row(40.0, |mut row| {
                                    row.col(|ui| {
                                        if ui.button(&search_result.file_name_no_ext).clicked() {
                                            self.stop_emulator_if_running();
                                            self.state.running_emulator = Some(launch_emulator(
                                                &search_result.full_path,
                                                &self.config,
                                            ));
                                        }
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{}KB", search_result.file_size_kb));
                                    });
                                });
                            }
                        });
                });
            }
        });
    }
}

impl eframe::App for JgbApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        let prev_config = self.config.clone();

        if self
            .state
            .running_emulator
            .as_ref()
            .map_or(false, |emulator_instance| {
                emulator_instance.thread.is_finished()
            })
        {
            let thread = self.state.running_emulator.take().unwrap().thread;
            self.state.emulation_error = thread.join().unwrap().err();
        }

        if self
            .state
            .key_input_thread
            .as_ref()
            .map_or(false, KeyInputThread::is_finished)
        {
            let thread = self.state.key_input_thread.take().unwrap();

            input::handle_key_input_thread_result(thread, &mut self.config);

            self.state.key_input_thread = None;
        }

        self.render_menu(ctx, frame);

        self.render_rom_list(ctx);

        match self.state.open_settings_window {
            Some(OpenSettingsWindow::General) => {
                self.render_general_settings_window(ctx);
            }
            Some(OpenSettingsWindow::Input) => {
                self.render_input_settings_window(ctx);
            }
            Some(OpenSettingsWindow::Hotkey) => {
                self.render_hotkey_settings_window(ctx);
            }
            None => {}
        }

        if self.state.emulation_error.is_some() {
            self.render_error_window(ctx);
        }

        if prev_config != self.config {
            // Save config immediately on changes
            self.save_config();

            self.state
                .refresh_rom_search_results(self.config.rom_search_dir.as_ref());
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

    let run_config = RunConfig {
        gb_file_path: gb_file.into(),
        audio_enabled: app_config.audio_enabled,
        sync_to_audio: app_config.audio_sync_enabled,
        vsync_enabled: app_config.vsync_enabled,
        launch_fullscreen: app_config.launch_in_fullscreen,
        borderless_fullscreen: app_config.fullscreen_mode == FullscreenMode::Borderless,
        force_integer_scaling: app_config.force_integer_scaling,
        window_width: app_config.window_width,
        window_height: app_config.window_height,
        audio_debugging_enabled: false,
        audio_60hz: app_config.audio_60hz_hack_enabled,
        input_config: app_config.input.clone(),
        hotkey_config: app_config.hotkeys.clone(),
        controller_config: ControllerConfig::default(),
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
