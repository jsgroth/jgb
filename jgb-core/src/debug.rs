use crate::apu;
use crate::apu::ApuDebugOutput;
use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};
use std::path::Path;

pub struct SampleFileWriter {
    writer: RefCell<BufWriter<File>>,
}

impl SampleFileWriter {
    fn new(filename: &str) -> Result<Self, io::Error> {
        let file = File::create(Path::new(filename))?;
        let writer = BufWriter::new(file);

        Ok(Self {
            writer: RefCell::new(writer),
        })
    }

    fn write_i16(&self, samples: &[i16]) -> Result<(), io::Error> {
        let mut writer = self.writer.borrow_mut();
        for &sample in samples {
            writer.write_all(&sample.to_le_bytes())?;
        }

        Ok(())
    }

    fn write_f64(&self, samples: &[f64]) -> Result<(), io::Error> {
        let mut writer = self.writer.borrow_mut();
        for &sample in samples {
            let sample = (sample * 30000.0) as i16;
            writer.write_all(&sample.to_le_bytes())?;
        }

        Ok(())
    }
}

pub struct FileApuDebugSink {
    channel_1: SampleFileWriter,
    channel_2: SampleFileWriter,
    channel_3: SampleFileWriter,
    channel_4: SampleFileWriter,
    master: SampleFileWriter,
}

impl FileApuDebugSink {
    pub fn new() -> Result<Self, io::Error> {
        let channel_1 = SampleFileWriter::new("channel1.pcm")?;
        let channel_2 = SampleFileWriter::new("channel2.pcm")?;
        let channel_3 = SampleFileWriter::new("channel3.pcm")?;
        let channel_4 = SampleFileWriter::new("channel4.pcm")?;
        let master = SampleFileWriter::new("master.pcm")?;

        Ok(Self {
            channel_1,
            channel_2,
            channel_3,
            channel_4,
            master,
        })
    }
}

impl apu::DebugSink for FileApuDebugSink {
    fn collect_samples(&self, samples: &ApuDebugOutput) {
        self.channel_1
            .write_f64(&[samples.ch1_l, samples.ch1_r])
            .expect("audio debug write failed");
        self.channel_2
            .write_f64(&[samples.ch2_l, samples.ch2_r])
            .expect("audio debug write failed");
        self.channel_3
            .write_f64(&[samples.ch3_l, samples.ch3_r])
            .expect("audio debug write failed");
        self.channel_4
            .write_f64(&[samples.ch4_l, samples.ch4_r])
            .expect("audio debug write failed");
        self.master
            .write_i16(&[samples.master_l, samples.master_r])
            .expect("audio debug write failed");
    }
}
