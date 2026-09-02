use std::io::Cursor;

use anyhow::{Result, bail};
use gif::{ColorOutput, DecodeOptions, Repeat};
use gpui::RenderImage;
use image::{AnimationDecoder, codecs::gif::GifDecoder};

/// Delays in a gif are stored in centiseconds (10ms units)
/// The 90s were fuckin weird idk what to tell you
const MIN_DURATION_CS: u32 = 100;

pub struct Unfucked {
    pub bytes: Vec<u8>,
    pub secs: f64,
}

pub struct Processed {
    pub original_secs: f64,
    pub unfucked: Option<Unfucked>,
}

/// Gif renderers treat delays under 20ms as 100ms
fn effective_delay_cs(delay: u16) -> u32 {
    if delay < 2 { 10 } else { u32::from(delay) }
}

pub fn process(input: &[u8]) -> Result<Processed> {
    let mut options = DecodeOptions::new();
    options.set_color_output(ColorOutput::Indexed);
    let mut decoder = options.read_info(Cursor::new(input))?;
    let width = decoder.width();
    let height = decoder.height();
    let global_palette = decoder.global_palette().map(<[u8]>::to_vec);

    let mut frames = Vec::new();
    while let Some(frame) = decoder.read_next_frame()? {
        let mut frame = frame.clone();
        frame.interlaced = false;
        frames.push(frame);
    }
    if frames.is_empty() {
        bail!("gif contains no frames");
    }

    let duration_cs: u32 = frames.iter().map(|f| effective_delay_cs(f.delay)).sum();
    let original_secs = f64::from(duration_cs) / 100.0;
    if duration_cs >= MIN_DURATION_CS {
        return Ok(Processed {
            original_secs,
            unfucked: None,
        });
    }

    let loops = MIN_DURATION_CS.div_ceil(duration_cs);
    let mut encoder = gif::Encoder::new(
        Vec::new(),
        width,
        height,
        global_palette.as_deref().unwrap_or(&[]),
    )?;
    encoder.set_repeat(Repeat::Infinite)?;
    for _ in 0..loops {
        for frame in &frames {
            encoder.write_frame(frame)?;
        }
    }
    let bytes = encoder.into_inner()?;

    Ok(Processed {
        original_secs,
        unfucked: Some(Unfucked {
            bytes,
            secs: f64::from(duration_cs * loops) / 100.0,
        }),
    })
}

pub fn decode_preview(input: &[u8]) -> Result<RenderImage> {
    let decoder = GifDecoder::new(Cursor::new(input))?;
    let mut frames = Vec::new();
    for frame in decoder.into_frames() {
        let mut frame = frame?;
        for pixel in frame.buffer_mut().chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        frames.push(frame);
    }
    if frames.is_empty() {
        bail!("gif contains no frames");
    }
    Ok(RenderImage::new(frames))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gif_with_delays(delays: &[u16]) -> Vec<u8> {
        let mut encoder = gif::Encoder::new(Vec::new(), 1, 1, &[0, 0, 0]).unwrap();
        for &delay in delays {
            let mut frame = gif::Frame::from_indexed_pixels(1, 1, vec![0], None);
            frame.delay = delay;
            encoder.write_frame(&frame).unwrap();
        }
        encoder.into_inner().unwrap()
    }

    fn frame_count(bytes: &[u8]) -> usize {
        let mut decoder = DecodeOptions::new().read_info(Cursor::new(bytes)).unwrap();
        let mut count = 0;
        while decoder.read_next_frame().unwrap().is_some() {
            count += 1;
        }
        count
    }

    #[test]
    fn short_gif_is_looped_to_at_least_one_second() {
        let processed = process(&gif_with_delays(&[3, 3])).unwrap();
        assert_eq!(processed.original_secs, 0.06);
        let unfucked = processed.unfucked.expect("short gif should be unfucked");
        assert_eq!(unfucked.secs, 1.02);
        assert_eq!(frame_count(&unfucked.bytes), 34);
    }

    #[test]
    fn sub_20ms_delays_count_as_100ms() {
        let processed = process(&gif_with_delays(&[0, 1])).unwrap();
        assert_eq!(processed.original_secs, 0.2);
        assert_eq!(processed.unfucked.unwrap().secs, 1.0);
    }

    #[test]
    fn long_gif_is_left_alone() {
        let processed = process(&gif_with_delays(&[50, 50])).unwrap();
        assert_eq!(processed.original_secs, 1.0);
        assert!(processed.unfucked.is_none());
    }

    #[test]
    fn empty_gif_is_an_error() {
        assert!(process(&gif_with_delays(&[])).is_err());
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(process(b"definitely not a gif").is_err());
    }
}
