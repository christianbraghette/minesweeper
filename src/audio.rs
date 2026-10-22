use rodio::{OutputStream, Sink, Source, source::SineWave};
use std::time::Duration;

pub struct AudioEngine {
    _stream: Option<OutputStream>,
    pub sink: Option<Sink>,
}

impl AudioEngine {
    pub fn new() -> Self {
        if let Ok((stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                return AudioEngine {
                    _stream: Some(stream),
                    sink: Some(sink),
                };
            }
        }
        AudioEngine {
            _stream: None,
            sink: None,
        }
    }

    pub fn set_volume(&self, value: u8) {
        if let Some(ref sink) = self.sink {
            sink.set_volume(value as f32 / 255.0);
        }
    }

    pub fn play_reveal(&self) {
        if let Some(ref sink) = self.sink {
            if sink.empty() {
                sink.append(SineWave::new(440.0).take_duration(Duration::from_millis(50)));
            }
        }
    }

    pub fn play_flag(&self) {
        if let Some(ref sink) = self.sink {
            sink.append(SineWave::new(600.0).take_duration(Duration::from_millis(70)));
        }
    }

    pub fn play_explosion(&self) {
        if let Some(ref sink) = self.sink {
            sink.clear();
            sink.play();
            sink.append(SineWave::new(300.0).take_duration(Duration::from_millis(200)));
            sink.append(SineWave::new(100.0).take_duration(Duration::from_millis(200)));
        }
    }

    pub fn play_victory(&self) {
        if let Some(ref sink) = self.sink {
            sink.clear();
            sink.play();
            sink.append(SineWave::new(587.33).take_duration(Duration::from_millis(150)));
            sink.append(SineWave::new(880.0).take_duration(Duration::from_millis(300)));
        }
    }
}
