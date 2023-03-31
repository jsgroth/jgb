use eframe::Frame;
use egui::{menu, Button, Context, Key, KeyboardShortcut, Modifiers, TopBottomPanel, Widget};
use jgb_core::{EmulationError, InputConfig, RunConfig};
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

#[derive(Debug, Default)]
pub struct JgbApp {
    running_emulator: Option<EmulatorInstance>,
    last_open_dir: Option<PathBuf>,
}

impl JgbApp {
    fn handle_open(&mut self) {
        let mut file_dialog = FileDialog::new().add_filter("gb", &["gb"]);

        if let Some(last_open_dir) = self.last_open_dir.as_ref() {
            file_dialog = file_dialog.set_directory(last_open_dir);
        }

        let file = file_dialog.pick_file();

        if let Some(file) = file.and_then(|file| file.to_str().map(String::from)) {
            self.last_open_dir = Path::new(&file).parent().map(|parent| parent.to_path_buf());

            self.stop_emulator_if_running();

            self.running_emulator = Some(launch_emulator(&file));
        }
    }

    fn stop_emulator_if_running(&mut self) {
        if let Some(running_emulator) = self.running_emulator.take() {
            log::info!("Shutting down existing emulator instance");

            *running_emulator.quit_signal.lock().unwrap() = true;

            // TODO actually handle errors
            running_emulator.thread.join().unwrap().unwrap();
        }
    }
}

impl eframe::App for JgbApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        let open_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::O);
        if ctx.input_mut(|input| input.consume_shortcut(&open_shortcut)) {
            self.handle_open();
        }

        let quit_shortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Q);
        if ctx.input_mut(|input| input.consume_shortcut(&quit_shortcut)) {
            frame.close();
        }

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
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
            });
        });
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

fn launch_emulator(gb_file: &str) -> EmulatorInstance {
    log::info!("Launching emulator instance for file path '{gb_file}'");

    // TODO actually make things configurable
    let run_config = RunConfig {
        gb_file_path: gb_file.into(),
        audio_enabled: true,
        sync_to_audio: true,
        vsync_enabled: true,
        window_width: 4 * 160,
        window_height: 4 * 144,
        audio_debugging_enabled: false,
        audio_60hz: true,
        input_config: InputConfig::default(),
    };

    let quit_signal = Arc::new(Mutex::new(false));

    let quit_signal_clone = Arc::clone(&quit_signal);
    let thread = thread::spawn(move || jgb_core::run(&run_config, quit_signal_clone));

    EmulatorInstance {
        thread,
        quit_signal,
    }
}
