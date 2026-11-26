#[cfg(test)]
#[path = "../../../backend/librespot-vam/src/free_pcm.rs"]
mod free_pcm;
#[cfg(test)]
mod tests {
    use super::free_pcm;
    use std::fs::File;
    use symphonia::core::{audio::SampleBuffer, errors::Error};
    #[test]
    fn decode_additional_formats() {
        // CI generates synthetic, unprotected tones with FFmpeg.
        let Ok(dir) = std::env::var("VAM_CODEC_FIXTURES") else { return; };
        for ext in ["mp3", "ogg", "flac", "wav"] {
            let path = std::path::Path::new(&dir).join(format!("tone.{ext}"));
            let (mut format, mut decoder, id) = free_pcm::open(File::open(path).unwrap()).unwrap();
            let mut count = 0;
            let mut peak = 0.0f64;
            let mut resampler = free_pcm::Resampler::default();
            loop {
                let packet = match format.next_packet() {
                    Ok(p) => p,
                    Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => panic!("{ext}: {e}"),
                };
                if packet.track_id() != id { continue; }
                let decoded = decoder.decode(&packet).unwrap();
                let spec = *decoded.spec();
                let mut buf = SampleBuffer::<f64>::new(decoded.capacity() as u64, spec);
                buf.copy_interleaved_ref(decoded);
                let pcm = resampler.convert(buf.samples(), spec.channels.count(), spec.rate);
                count += pcm.len();
                for sample in pcm { assert!(sample.is_finite()); peak = peak.max(sample.abs()); }
            }
            assert!(count > 20000 && peak > 0.005, "{ext}: count={count} peak={peak}");
        }
    }
    #[test]
    fn decode_real_aac_fixture() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../backend/librespot-vam/tests/fixtures/free-tone.m4a");
        let (mut format, mut decoder, id) = free_pcm::open(File::open(path).unwrap()).unwrap();
        let mut resampler = free_pcm::Resampler::default();
        let mut samples = 0;
        let mut peak = 0.0f64;
        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => panic!("{e}"),
            };
            if packet.track_id() != id { continue; }
            let decoded = decoder.decode(&packet).unwrap();
            let spec = *decoded.spec();
            assert_eq!(spec.rate, 48000);
            let channels = spec.channels.count();
            assert_eq!(channels, 1);
            let mut buf = SampleBuffer::<f64>::new(decoded.capacity() as u64, spec);
            buf.copy_interleaved_ref(decoded);
            let pcm = resampler.convert(buf.samples(), channels, spec.rate);
            samples += pcm.len();
            for x in pcm { assert!(x.is_finite()); peak = peak.max(x.abs()); }
        }
        assert!((35000..50000).contains(&samples), "samples={samples}");
        assert!(peak > 0.005 && peak < 0.5, "peak={peak}");
        println!("AAC fixture decoded and resampled: {samples} stereo samples, peak={peak}");
    }
}
