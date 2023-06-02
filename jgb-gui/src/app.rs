mod config;
mod input;

use anyhow::Context;
use eframe::epaint::Color32;
use eframe::Frame;
use egui::{
    menu, Align, Button, CentralPanel, Direction, Key, KeyboardShortcut, Layout, Modifiers,
    TextEdit, TopBottomPanel, Widget, Window,
};
use egui_extras::{Column, TableBuilder};
use jgb_core::{EmulationError, GbColorScheme, GbcColorCorrection, HardwareMode, RunConfig};
use rfd::FileDialog;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{fs, thread};

use crate::app::config::FullscreenMode;
use crate::app::input::{
    ControllerSettingsWidget, HotkeySettingsWidget, InputThread, KeyboardSettingsWidget,
};
pub use config::AppConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenWindow {
    General,
    Keyboard,
    Controller,
    Hotkey,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CgbSupportType {
    DmgOnly,
    CgbEnhanced,
    CgbOnly,
}

impl CgbSupportType {
    fn from_byte(byte: u8) -> Self {
        match byte & 0xC0 {
            0xC0 => Self::CgbOnly,
            0x80 => Self::CgbEnhanced,
            0x40 | 0x00 => Self::DmgOnly,
            _ => panic!("{byte} & 0xC0 was not 0xC0/0x80/0x40/0x00"),
        }
    }

    fn default_hardware_mode(self) -> HardwareMode {
        match self {
            Self::DmgOnly => HardwareMode::GameBoy,
            Self::CgbEnhanced | Self::CgbOnly => HardwareMode::GameBoyColor,
        }
    }
}

#[derive(Debug, Clone)]
struct RomSearchResult {
    full_path: String,
    file_name_no_ext: String,
    file_size_kb: u64,
    cgb_support_type: CgbSupportType,
}

#[derive(Debug, Default)]
struct AppState {
    running_emulator: Option<EmulatorInstance>,
    open_window: Option<OpenWindow>,
    input_thread: Option<InputThread>,
    emulation_error: Option<EmulationError>,
    window_width_text: String,
    window_width_invalid: bool,
    window_height_text: String,
    window_height_invalid: bool,
    deadzone_text: String,
    rom_search_results: Vec<RomSearchResult>,
}

impl AppState {
    fn from_config(app_config: &AppConfig) -> Self {
        let mut state = Self {
            window_width_text: app_config.window_width.to_string(),
            window_height_text: app_config.window_height.to_string(),
            deadzone_text: app_config.controller.axis_deadzone.to_string(),
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

    #[allow(clippy::if_then_some_else_none)]
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
                    let path = dir_entry.path();
                    let extension = path.extension();
                    let is_gb_file = extension == Some(OsStr::new("gb")) || extension == Some(OsStr::new("gbc"));
                    let Ok(metadata) = dir_entry.metadata() else { return None };

                    if is_gb_file && metadata.is_file() {
                        let cgb_support_type = match determine_cgb_support_type(&path) {
                            Ok(cgb_support_type) => cgb_support_type,
                            Err(err) => {
                                log::error!("Error determining CGB support type: {err}");
                                return None;
                            }
                        };

                        let full_path = path.to_str()?.into();
                        let file_name_no_ext = path
                            .with_extension("")
                            .file_name()?
                            .to_str()?
                            .into();
                        let file_size_kb = metadata.len() / 1024;

                        Some(RomSearchResult {
                            full_path,
                            file_name_no_ext,
                            file_size_kb,
                            cgb_support_type,
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

fn determine_cgb_support_type<P>(path: P) -> Result<CgbSupportType, anyhow::Error>
where
    P: AsRef<Path>,
{
    let mut file = File::open(path.as_ref())
        .with_context(|| format!("error opening GB file: {}", path.as_ref().display()))?;

    file.seek(SeekFrom::Start(0x0143)).with_context(|| {
        format!(
            "error seeking to address 0x0143 in GB file: {}",
            path.as_ref().display()
        )
    })?;

    let mut buffer = [0; 1];
    file.read_exact(&mut buffer).with_context(|| {
        format!(
            "error reading byte at address 0x0143 in GB file: {}",
            path.as_ref().display()
        )
    })?;

    Ok(CgbSupportType::from_byte(buffer[0]))
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

    fn handle_open(&mut self, hardware_mode: HardwareMode) {
        let mut file_dialog = FileDialog::new().add_filter("gb/gbc", &["gb", "gbc"]);
        if let Some(rom_search_dir) = &self.config.rom_search_dir {
            file_dialog = file_dialog.set_directory(Path::new(rom_search_dir));
        }
        let file = file_dialog.pick_file();

        if let Some(file) = file.and_then(|file| file.to_str().map(String::from)) {
            self.stop_emulator_if_running();

            self.state.running_emulator = Some(launch_emulator(&file, &self.config, hardware_mode));
        }
    }

    fn stop_emulator_if_running(&mut self) {
        if let Some(running_emulator) = self.state.running_emulator.take() {
            log::info!("Shutting down existing emulator instance");

            running_emulator.quit_signal.store(true, Ordering::Relaxed);

            // TODO actually handle errors
            running_emulator.thread.join().unwrap().unwrap();
        }
    }

    fn save_config(&self) {
        // TODO actually handle errors
        self.config.save_to_file(&self.config_path).unwrap();
    }

    fn render_error_window(&mut self, ctx: &egui::Context) {
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

    fn render_menu(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        let gb_open_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::O);
        if ctx.input_mut(|input| input.consume_shortcut(&gb_open_shortcut)) {
            self.handle_open(HardwareMode::GameBoy);
        }

        let gbc_open_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::P);
        if ctx.input_mut(|input| input.consume_shortcut(&gbc_open_shortcut)) {
            self.handle_open(HardwareMode::GameBoyColor);
        }

        let quit_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Q);
        if ctx.input_mut(|input| input.consume_shortcut(&quit_shortcut)) {
            frame.close();
        }

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.set_enabled(
                self.state.open_window.is_none()
                    && self.state.input_thread.is_none()
                    && self.state.emulation_error.is_none(),
            );
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    let gb_open_button = Button::new("Open (GB)")
                        .shortcut_text(ctx.format_shortcut(&gb_open_shortcut))
                        .ui(ui);
                    if gb_open_button.clicked() {
                        self.handle_open(HardwareMode::GameBoy);
                        ui.close_menu();
                    }

                    let gbc_open_button = Button::new("Open (GBC)")
                        .shortcut_text(ctx.format_shortcut(&gbc_open_shortcut))
                        .ui(ui);
                    if gbc_open_button.clicked() {
                        self.handle_open(HardwareMode::GameBoyColor);
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
                        self.state.open_window = Some(OpenWindow::General);
                        ui.close_menu();
                    }

                    if ui.button("Keyboard Settings").clicked() {
                        self.state.open_window = Some(OpenWindow::Keyboard);
                        ui.close_menu();
                    }

                    if ui.button("Controller Settings").clicked() {
                        self.state.open_window = Some(OpenWindow::Controller);
                        ui.close_menu();
                    }

                    if ui.button("Hotkey Settings").clicked() {
                        self.state.open_window = Some(OpenWindow::Hotkey);
                        ui.close_menu();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.state.open_window = Some(OpenWindow::About);
                        ui.close_menu();
                    }
                });
            });
        });
    }

    fn render_general_settings_window(&mut self, ctx: &egui::Context) {
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
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.config.fullscreen_mode, FullscreenMode::Exclusive, "Exclusive");
                        ui.radio_value(&mut self.config.fullscreen_mode, FullscreenMode::Borderless, "Borderless");
                    });
                });

                ui.group(|ui| {
                    ui.label("GB color palette");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.config.color_scheme, GbColorScheme::BlackAndWhite, "Black & white");
                        ui.radio_value(&mut self.config.color_scheme, GbColorScheme::GreenTint, "Green tint");
                        ui.radio_value(&mut self.config.color_scheme, GbColorScheme::LimeGreen, "Lime-green");
                    });
                });

                ui.group(|ui| {
                    ui.label("GBC color correction");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.config.gbc_color_correction, GbcColorCorrection::None, "None")
                            .on_hover_text("Render raw RGB color values, similar to the backlit Game Boy Advance SP LCD");
                        ui.radio_value(&mut self.config.gbc_color_correction, GbcColorCorrection::GbcLcd, "GBC LCD")
                            .on_hover_text("Mangle color values to create a somewhat desaturated look, similar to the Game Boy Color LCD");
                    });
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

                ui.add_space(20.0);

                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        self.state.open_window = None;
                    }
                });
            });
        if !settings_open {
            self.state.open_window = None;
        };
    }

    fn render_keyboard_settings_window(&mut self, ctx: &egui::Context) {
        let mut settings_open = true;
        Window::new("Keyboard Settings")
            .id("keyboard_settings".into())
            .resizable(false)
            .open(&mut settings_open)
            .show(ctx, |ui| {
                ui.set_enabled(self.state.input_thread.is_none());

                if let Some(thread) = KeyboardSettingsWidget::new(&self.config.input).ui(ui) {
                    self.state.input_thread = Some(thread);
                }
            });
        if !settings_open {
            self.state.open_window = None;
        }
    }

    fn render_controller_settings_window(&mut self, ctx: &egui::Context) {
        let mut settings_open = true;
        Window::new("Controller Settings")
            .id("controller_settings".into())
            .resizable(false)
            .open(&mut settings_open)
            .show(ctx, |ui| {
                ui.set_enabled(self.state.input_thread.is_none());

                if let Some(thread) = ControllerSettingsWidget::new(
                    &mut self.config.controller,
                    &mut self.state.deadzone_text,
                )
                .ui(ui)
                {
                    self.state.input_thread = Some(thread);
                }
            });
        if !settings_open {
            self.state.open_window = None;
        }
    }

    fn render_hotkey_settings_window(&mut self, ctx: &egui::Context) {
        let mut settings_open = true;
        Window::new("Hotkey Settings")
            .id("hotkey_settings".into())
            .resizable(false)
            .open(&mut settings_open)
            .show(ctx, |ui| {
                ui.set_enabled(self.state.input_thread.is_none());

                if let Some(thread) = HotkeySettingsWidget::new(&mut self.config.hotkeys).ui(ui) {
                    self.state.input_thread = Some(thread);
                }
            });
        if !settings_open {
            self.state.open_window = None;
        }
    }

    fn render_about_window(&mut self, ctx: &egui::Context) {
        let mut about_open = true;
        Window::new("About")
            .id("about".into())
            .resizable(false)
            .open(&mut about_open)
            .show(ctx, |ui| {
                ui.heading("jgb");

                ui.add_space(10.0);
                ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));

                ui.add_space(15.0);
                ui.label("Copyright © 2022-2023 James Groth");

                ui.add_space(15.0);
                ui.horizontal(|ui| {
                    ui.label("Source code:");
                    ui.hyperlink("https://www.github.com/jsgroth/jgb");
                });
            });
        if !about_open {
            self.state.open_window = None;
        }
    }

    fn render_rom_list(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(self.state.open_window.is_none() && self.state.input_thread.is_none());

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
                        .columns(Column::auto(), 2)
                        .column(Column::remainder())
                        .header(30.0, |mut row| {
                            row.col(|ui| {
                                ui.heading("Name");
                            });
                            row.col(|ui| {
                                ui.heading("Type");
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
                                                search_result
                                                    .cgb_support_type
                                                    .default_hardware_mode(),
                                            ));
                                        }
                                    });
                                    row.col(|ui| {
                                        let type_text = match search_result.cgb_support_type {
                                            CgbSupportType::DmgOnly => "GB",
                                            CgbSupportType::CgbEnhanced => "GB/GBC",
                                            CgbSupportType::CgbOnly => "GBC",
                                        };
                                        ui.label(type_text);
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
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        let prev_config = self.config.clone();

        if self
            .state
            .running_emulator
            .as_ref()
            .is_some_and(|emulator_instance| emulator_instance.thread.is_finished())
        {
            let thread = self.state.running_emulator.take().unwrap().thread;
            self.state.emulation_error = thread.join().unwrap().err();
        }

        if self
            .state
            .input_thread
            .as_ref()
            .is_some_and(InputThread::is_finished)
        {
            let thread = self.state.input_thread.take().unwrap();

            input::handle_input_thread_result(thread, &mut self.config);

            self.state.input_thread = None;
        }

        self.render_menu(ctx, frame);

        self.render_rom_list(ctx);

        match self.state.open_window {
            Some(OpenWindow::General) => {
                self.render_general_settings_window(ctx);
            }
            Some(OpenWindow::Keyboard) => {
                self.render_keyboard_settings_window(ctx);
            }
            Some(OpenWindow::Controller) => {
                self.render_controller_settings_window(ctx);
            }
            Some(OpenWindow::Hotkey) => {
                self.render_hotkey_settings_window(ctx);
            }
            Some(OpenWindow::About) => {
                self.render_about_window(ctx);
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
    quit_signal: Arc<AtomicBool>,
}

#[must_use]
fn launch_emulator(
    gb_file: &str,
    app_config: &AppConfig,
    hardware_mode: HardwareMode,
) -> EmulatorInstance {
    log::info!("Launching emulator instance for file path '{gb_file}'");

    let run_config = RunConfig {
        gb_file_path: gb_file.into(),
        hardware_mode,
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
        color_scheme: app_config.color_scheme,
        gbc_color_correction: app_config.gbc_color_correction,
        input_config: app_config.input.clone(),
        hotkey_config: app_config.hotkeys.clone(),
        controller_config: app_config.controller.clone(),
    };

    let quit_signal = Arc::new(AtomicBool::new(false));

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
