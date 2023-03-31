use eframe::NativeOptions;
use jgb_gui::JgbApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    eframe::run_native(
        "jgb",
        NativeOptions::default(),
        Box::new(|_| Box::<JgbApp>::default()),
    )
}
