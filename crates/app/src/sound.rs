use gtk::glib;
use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink};
use std::cell::RefCell;
use std::num::NonZero;
use std::time::Duration;
use tinyhand_core::catalog::TapSound;
use tinyhand_core::paths;

const IDLE_CLOSE: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct Wav {
    pub channels: u16,
    pub rate: u32,
    pub samples: Vec<f32>,
}

pub fn parse_wav(bytes: &[u8]) -> Option<Wav> {
    if bytes.get(..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let u16_at = |at: usize| bytes.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |at: usize| bytes.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let mut format: Option<(u16, u32)> = None;
    let mut at = 12;
    while let (Some(id), Some(size)) = (bytes.get(at..at + 4), u32_at(at + 4)) {
        let body = at + 8;
        let end = body.checked_add(size as usize)?.min(bytes.len());
        match id {
            b"fmt " => {
                let pcm16 = u16_at(body)? == 1 && u16_at(body + 14)? == 16;
                format = pcm16.then_some((u16_at(body + 2)?, u32_at(body + 4)?));
            }
            b"data" => {
                let (channels, rate) = format.filter(|&(c, r)| c > 0 && r > 0)?;
                let samples = bytes[body..end]
                    .chunks_exact(2)
                    .map(|s| f32::from(i16::from_le_bytes([s[0], s[1]])) / 32768.0)
                    .collect();
                return Some(Wav { channels, rate, samples });
            }
            _ => {}
        }
        at = end + (size as usize & 1);
    }
    None
}

#[derive(Default)]
pub struct Player {
    sounds: RefCell<Vec<(TapSound, Option<Wav>)>>,
    out: RefCell<Option<MixerDeviceSink>>,
    closing: RefCell<Option<glib::SourceId>>,
}

impl Player {
    fn wav(&self, sound: TapSound) -> Option<Wav> {
        let mut sounds = self.sounds.borrow_mut();
        if let Some((_, wav)) = sounds.iter().find(|(s, _)| *s == sound) {
            return wav.clone();
        }
        let wav = paths::find_data(&format!("sounds/{}.wav", sound.as_str()))
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| parse_wav(&bytes));
        sounds.push((sound, wav.clone()));
        wav
    }

    pub fn warm(&self) {
        if let Some(id) = self.closing.take() {
            id.remove();
        }
        let mut out = self.out.borrow_mut();
        if out.is_none() {
            *out = DeviceSinkBuilder::open_default_sink().ok().map(|mut sink| {
                sink.log_on_drop(false);
                sink
            });
        }
    }

    pub fn cool(self: &std::rc::Rc<Self>) {
        if self.out.borrow().is_none() || self.closing.borrow().is_some() {
            return;
        }
        let this = self.clone();
        let id = glib::timeout_add_local_once(IDLE_CLOSE, move || {
            this.closing.take();
            this.out.take();
        });
        self.closing.replace(Some(id));
    }

    pub fn play(&self, sound: TapSound, volume: f64) {
        let Some(wav) = self.wav(sound) else {
            return;
        };
        self.warm();
        let (Some(channels), Some(rate)) = (NonZero::new(wav.channels), NonZero::new(wav.rate)) else {
            return;
        };
        if let Some(out) = self.out.borrow().as_ref() {
            let gain = volume as f32;
            let samples: Vec<f32> = wav.samples.iter().map(|s| s * gain).collect();
            out.mixer().add(SamplesBuffer::new(channels, rate, samples));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_sounds_are_mono_pcm16() {
        for sound in TapSound::ALL {
            let path = format!("{}/../../assets/sounds/{}.wav", env!("CARGO_MANIFEST_DIR"), sound.as_str());
            let wav = parse_wav(&std::fs::read(&path).unwrap()).unwrap_or_else(|| panic!("{path}"));
            assert_eq!((wav.channels, wav.rate), (1, 44100));
            assert!(wav.samples.len() > 1000);
            assert!(wav.samples.iter().any(|s| s.abs() > 0.05));
        }
    }

    #[test]
    fn rejects_things_that_are_not_pcm16_wav() {
        assert!(parse_wav(b"").is_none());
        assert!(parse_wav(b"RIFF\0\0\0\0WAVEdata\x04\0\0\0\0\0\0\0").is_none());
        assert!(parse_wav(b"OggS and so on, long enough to pass").is_none());
    }
}
