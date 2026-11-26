//! Shared multi-format decoder initialization and stereo resampling.
use std::fs::File;
use symphonia::core::{
    codecs::{Decoder, DecoderOptions}, errors::{Error, Result},
    formats::{FormatOptions, FormatReader},
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::MetadataOptions, probe::Hint,
};
pub fn open(input: File) -> Result<(Box<dyn FormatReader>, Box<dyn Decoder>, u32)> {
    let stream = MediaSourceStream::new(Box::new(input), MediaSourceStreamOptions::default());
    let hint = Hint::new(); // Probe the actual bytes rather than trusting the file extension.
    let format = symphonia::default::get_probe().format(&hint, stream, &FormatOptions::default(), &MetadataOptions::default())?.format;
    let track = format.default_track().ok_or(Error::Unsupported("no audio track"))?;
    let id = track.id;
    let decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;
    Ok((format, decoder, id))
}
// Linear stereo resampling with continuous phase across decoded packets.
// Native sink runs at 44.1kHz; AAC commonly arrives at 44.1 or 48kHz.
#[derive(Default)]
pub struct Resampler { prev: Option<[f64; 2]>, phase: f64 }
impl Resampler {
    pub fn convert(&mut self, input: &[f64], channels: usize, rate: u32) -> Vec<f64> {
        let step = rate as f64 / 44100.0;
        let mut out = Vec::with_capacity(input.len()*2+8);
        for frame in input.chunks_exact(channels) {
            let current = [frame[0], if channels == 2 { frame[1] } else { frame[0] }];
            if let Some(previous) = self.prev {
                while self.phase <= 1.0 {
                    for ch in 0..2 { out.push(previous[ch]+(current[ch]-previous[ch])*self.phase); }
                    self.phase += step;
                }
                self.phase -= 1.0;
            } else { out.extend_from_slice(&current); self.phase = step; }
            self.prev = Some(current);
        }
        out
    }
}
