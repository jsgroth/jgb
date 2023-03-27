use crate::apu;

pub struct LoggingApuDebugSink;

impl apu::DebugSink for LoggingApuDebugSink {
    fn collect_samples(&self, samples: &apu::DebugOutput) {
        log::debug!("{samples:?}");
    }
}
