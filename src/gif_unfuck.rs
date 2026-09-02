use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::{Result, bail};
use gif::{ColorOutput, DecodeOptions, Repeat};
use gpui::RenderImage;
use image::{AnimationDecoder, codecs::gif::GifDecoder};

/// Delays in a gif are stored in centiseconds (10ms units)
/// The 90s were fuckin weird idk what to tell you
const MIN_DURATION_CS: u32 = 100;

pub struct Unfucked {
    pub path: PathBuf,
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

pub fn process(path: &Path) -> Result<Processed> {
    let mut options = DecodeOptions::new();
    options.set_color_output(ColorOutput::Indexed);
    let mut decoder = options.read_info(BufReader::new(File::open(path)?))?;
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
    let out_path = unique_temp_path();
    let mut encoder = gif::Encoder::new(
        BufWriter::new(File::create(&out_path)?),
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
    drop(encoder);

    Ok(Processed {
        original_secs,
        unfucked: Some(Unfucked {
            path: out_path,
            secs: f64::from(duration_cs * loops) / 100.0,
        }),
    })
}

pub fn decode_preview(path: &Path) -> Result<RenderImage> {
    let decoder = GifDecoder::new(BufReader::new(File::open(path)?))?;
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

fn unique_temp_path() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    std::env::temp_dir().join(format!(
        "bsky-gif-unfucker-{}-{}.gif",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}
