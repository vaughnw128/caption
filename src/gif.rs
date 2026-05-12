use std::io::{BufWriter, Cursor};

use gif::{DisposalMethod, Encoder, Frame, Repeat};
use image::codecs::gif::GifDecoder as ImageGifDecoder;
use image::{AnimationDecoder, Delay, RgbaImage};
use quantette::deps::palette::Srgb;
use quantette::deps::palette::cast::from_component_slice;
use quantette::{ImageRef, Pipeline, QuantizeMethod};
use rayon::prelude::*;
use tracing::{debug, info_span};
use webp_animation::WebPData;
use webp_animation::prelude::{
    Encoder as WebPEncoder, EncoderOptions, EncodingConfig, EncodingType, LossyEncodingConfig,
};

use crate::error::CaptionError;

pub struct GifData {
    pub width: u16,
    pub height: u16,
    frames: Vec<DecodedFrame>,
}

struct DecodedFrame {
    rgba: RgbaImage,
    delay: Delay,
}

struct PreparedRgbFrame {
    rgb: Vec<u8>,
    delay: u16,
}

struct GifDimensions {
    width: u16,
    source_height: u16,
    out_width: usize,
    out_height_usize: usize,
    bar_len: usize,
}

pub fn decode_gif(data: &[u8]) -> Result<GifData, CaptionError> {
    let _span = info_span!("decode_gif").entered();
    decode_gif_rgba(data)
}

pub fn decode_gif_for_webp(data: &[u8]) -> Result<GifData, CaptionError> {
    let _span = info_span!("decode_gif_for_webp").entered();
    decode_gif_rgba(data)
}

fn decode_gif_rgba(data: &[u8]) -> Result<GifData, CaptionError> {
    let (width, height) = gif_dimensions(data)?;
    let frames = decode_rgba(data)?;
    debug!(
        width,
        height,
        frame_count = frames.len(),
        "decoded GIF to full-canvas RGBA frames"
    );

    Ok(GifData {
        width,
        height,
        frames,
    })
}

fn decode_rgba(data: &[u8]) -> Result<Vec<DecodedFrame>, CaptionError> {
    let _span = info_span!("decode_rgba_image").entered();
    let frames = decode_rgba_image(data)?;
    debug!(
        frame_count = frames.len(),
        "image decoder produced RGBA GIF frames"
    );
    Ok(frames)
}

fn decode_rgba_image(data: &[u8]) -> Result<Vec<DecodedFrame>, CaptionError> {
    let decoder = ImageGifDecoder::new(Cursor::new(data))?;
    decoder
        .into_frames()
        .collect_frames()?
        .into_iter()
        .map(|frame| {
            Ok(DecodedFrame {
                delay: frame.delay(),
                rgba: frame.into_buffer(),
            })
        })
        .collect()
}

fn gif_dimensions(data: &[u8]) -> Result<(u16, u16), CaptionError> {
    if data.len() < 10 {
        return Err(CaptionError::UnsupportedFormat("GIF data too short".into()));
    }
    let width = u16::from_le_bytes([data[6], data[7]]);
    let height = u16::from_le_bytes([data[8], data[9]]);
    Ok((width, height))
}

pub fn encode_gif(
    gif_data: &GifData,
    bar_rgba: &[u8],
    _bar_width: u32,
    bar_height: u32,
) -> Result<Vec<u8>, CaptionError> {
    if gif_data.frames.is_empty() {
        return Err(CaptionError::EmptyImage);
    }

    let out_height = (gif_data.height as u32)
        .checked_add(bar_height)
        .ok_or_else(|| CaptionError::UnsupportedFormat("captioned GIF height overflow".into()))?;
    let out_height_u16 = u16::try_from(out_height).map_err(|_| {
        CaptionError::UnsupportedFormat("captioned GIF exceeds GIF height limit".into())
    })?;
    encode_gif_wu(gif_data, bar_rgba, bar_height, out_height_u16)
}

fn encode_gif_wu(
    gif_data: &GifData,
    bar_rgba: &[u8],
    bar_height: u32,
    out_height: u16,
) -> Result<Vec<u8>, CaptionError> {
    let _span = info_span!("encode_gif_wu", frame_count = gif_data.frames.len()).entered();
    let width = gif_data.width;
    let out_width = width as usize;
    let out_height_usize = out_height as usize;
    let bar_len = out_width * bar_height as usize * 4;

    if bar_rgba.len() < bar_len {
        return Err(CaptionError::UnsupportedFormat(
            "caption bar does not match GIF width".into(),
        ));
    }

    let dims = GifDimensions {
        width,
        source_height: gif_data.height,
        out_width,
        out_height_usize,
        bar_len,
    };
    let prepared = prepare_rgb_frames(&gif_data.frames, dims, bar_rgba)?;

    let mut quantized: Vec<Frame<'static>> = {
        let _span = info_span!("quantize_frames", frame_count = prepared.len()).entered();
        prepared
            .par_iter()
            .map(|frame| {
                let mut gif_frame = quantize_frame_wu(width, out_height, frame);
                gif_frame.make_lzw_pre_encoded();
                gif_frame
            })
            .collect()
    };

    let estimated_size = gif_data.width as usize * out_height as usize * quantized.len() / 2;
    let mut out = Vec::with_capacity(estimated_size);

    {
        let _span = info_span!("lzw_encode", frame_count = quantized.len()).entered();
        let writer = BufWriter::new(&mut out);
        let mut encoder = Encoder::new(writer, width, out_height, &[])?;
        encoder.set_repeat(Repeat::Infinite)?;

        for frame in &mut quantized {
            encoder.write_lzw_pre_encoded_frame(frame)?;
        }
    }

    Ok(out)
}

fn prepare_rgb_frames(
    frames: &[DecodedFrame],
    dims: GifDimensions,
    bar_rgba: &[u8],
) -> Result<Vec<PreparedRgbFrame>, CaptionError> {
    let _span = info_span!("prepare_rgb_frames", frame_count = frames.len()).entered();
    frames
        .par_iter()
        .map(|decoded| {
            if decoded.rgba.width() != dims.width as u32
                || decoded.rgba.height() != dims.source_height as u32
            {
                return Err(CaptionError::UnsupportedFormat(
                    "decoded GIF frame size did not match canvas".into(),
                ));
            }

            let mut rgb = Vec::with_capacity(dims.out_width * dims.out_height_usize * 3);
            rgb.extend(
                bar_rgba[..dims.bar_len]
                    .chunks_exact(4)
                    .flat_map(|px| [px[0], px[1], px[2]]),
            );
            rgb.extend(
                decoded
                    .rgba
                    .as_raw()
                    .chunks_exact(4)
                    .flat_map(|px| [px[0], px[1], px[2]]),
            );

            debug_assert_eq!(rgb.len(), dims.out_width * dims.out_height_usize * 3);
            debug_assert_eq!(
                decoded.rgba.as_raw().len(),
                dims.out_width * dims.source_height as usize * 4
            );

            Ok(PreparedRgbFrame {
                rgb,
                delay: delay_to_centiseconds(decoded.delay),
            })
        })
        .collect()
}

fn quantize_frame_wu(width: u16, out_height: u16, frame: &PreparedRgbFrame) -> Frame<'static> {
    let pixels: &[Srgb<u8>] = from_component_slice(&frame.rgb);
    let img_ref =
        ImageRef::new(width as u32, out_height as u32, pixels).expect("pixel count mismatch");
    let indexed = Pipeline::new()
        .quantize_method(QuantizeMethod::Wu)
        .ditherer(None)
        .input_image(img_ref)
        .output_srgb8_indexed_image();

    let palette: Vec<u8> = indexed
        .palette()
        .iter()
        .flat_map(|c| [c.red, c.green, c.blue])
        .collect();
    let indices: Vec<u8> = indexed.indices().to_vec();

    let mut gif_frame = Frame::from_palette_pixels(width, out_height, indices, palette, None);
    gif_frame.delay = frame.delay;
    gif_frame.dispose = DisposalMethod::Keep;
    gif_frame
}

const WEBP_QUALITY: f32 = 90.0;
const WEBP_FAST_METHOD: usize = 0;
const WEBP_SAFE_METHOD: usize = 4;

pub fn encode_webp(
    gif_data: &GifData,
    bar_rgba: &[u8],
    _bar_width: u32,
    bar_height: u32,
) -> Result<Vec<u8>, CaptionError> {
    if gif_data.frames.is_empty() {
        return Err(CaptionError::EmptyImage);
    }

    let width = gif_data.width as u32;
    let height = (gif_data.height as u32)
        .checked_add(bar_height)
        .ok_or_else(|| CaptionError::UnsupportedFormat("captioned WebP height overflow".into()))?;

    let frame_w = gif_data.width as usize;
    let frame_h = gif_data.height as usize;
    let bar_h = bar_height as usize;
    let bar_len = frame_w * bar_h * 4;
    let frame_len = frame_w * frame_h * 4;

    if bar_rgba.len() < bar_len {
        return Err(CaptionError::UnsupportedFormat(
            "caption bar does not match GIF width".into(),
        ));
    }

    for decoded in &gif_data.frames {
        if decoded.rgba.width() != width || decoded.rgba.height() != gif_data.height as u32 {
            return Err(CaptionError::UnsupportedFormat(
                "decoded GIF frame size did not match canvas".into(),
            ));
        }
    }

    match encode_webp_frames(
        &gif_data.frames,
        (width, height),
        bar_rgba,
        bar_len,
        frame_len,
        WEBP_FAST_METHOD,
    ) {
        Ok(data) => Ok(data),
        Err(fast_error) => {
            debug!(
                method = WEBP_FAST_METHOD,
                fallback_method = WEBP_SAFE_METHOD,
                reason = %fast_error,
                "Fast WebP method failed; retrying with safer libwebp method"
            );
            encode_webp_frames(
                &gif_data.frames,
                (width, height),
                bar_rgba,
                bar_len,
                frame_len,
                WEBP_SAFE_METHOD,
            )
        }
    }
}

fn encode_webp_frames(
    frames: &[DecodedFrame],
    canvas_size: (u32, u32),
    bar_rgba: &[u8],
    bar_len: usize,
    frame_len: usize,
    method: usize,
) -> Result<Vec<u8>, CaptionError> {
    let fast_config = EncodingConfig {
        quality: WEBP_QUALITY,
        method,
        encoding_type: EncodingType::Lossy(LossyEncodingConfig::default()),
    };
    debug!(
        quality = fast_config.quality,
        method = fast_config.method,
        "Using global lossy WebP encoding config"
    );

    let _span = info_span!("webp_encode", frame_count = frames.len()).entered();
    let mut encoder = WebPEncoder::new_with_options(
        canvas_size,
        EncoderOptions {
            encoding_config: Some(fast_config),
            ..EncoderOptions::default()
        },
    )
    .map_err(webp_error)?;
    let mut timestamp_ms = 0i32;

    let mut canvas = vec![0u8; bar_len + frame_len];
    canvas[..bar_len].copy_from_slice(&bar_rgba[..bar_len]);

    for decoded in frames {
        canvas[bar_len..].copy_from_slice(decoded.rgba.as_raw());
        encoder
            .add_frame(&canvas, timestamp_ms)
            .map_err(webp_error)?;
        timestamp_ms = timestamp_ms.saturating_add(delay_to_milliseconds(decoded.delay));
    }

    let data: WebPData = encoder.finalize(timestamp_ms.max(1)).map_err(webp_error)?;
    Ok(data.to_vec())
}

fn delay_to_centiseconds(delay: Delay) -> u16 {
    (delay_to_milliseconds(delay) / 10)
        .try_into()
        .unwrap_or(u16::MAX)
}

fn delay_to_milliseconds(delay: Delay) -> i32 {
    let (numerator, denominator) = delay.numer_denom_ms();
    let denominator = denominator.max(1);
    let millis = ((numerator as u64) + (denominator as u64 / 2)) / denominator as u64;
    millis.try_into().unwrap_or(i32::MAX)
}

fn webp_error(error: webp_animation::Error) -> CaptionError {
    CaptionError::UnsupportedFormat(error.to_string())
}
