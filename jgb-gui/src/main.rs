use eframe::NativeOptions;
use env_logger::Env;
use jgb_gui::JgbApp;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    eframe::run_native(
        "jgb",
        NativeOptions::default(),
        Box::new(|_| Box::<JgbApp>::default()),
    )
}
