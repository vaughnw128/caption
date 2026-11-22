use std::io::Cursor;

use cosmic_text::SwashCache;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageFormat, RgbaImage};
use tracing::info_span;

use crate::error::CaptionError;
use crate::font::build_font_system;
use crate::gif::{decode_gif, decode_gif_for_webp, encode_gif, encode_webp};
use crate::layout::{build_layout, rasterize_bar};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Same,
    Png,
    Jpeg,
    WebP,
    Gif,
}

impl OutputFormat {
    fn resolve(self, input: ImageFormat) -> ImageFormat {
        match self {
            Self::Same => input,
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
            Self::WebP => ImageFormat::WebP,
            Self::Gif => ImageFormat::Gif,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Same => "",
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
            Self::Gif => "gif",
        }
    }
}

pub struct Captioner {
    font_system: cosmic_text::FontSystem,
    swash_cache: SwashCache,
}

#[derive(Debug, Clone)]
struct CaptionBar {
    rgba: Vec<u8>,
    height: u32,
}

impl Captioner {
    pub fn new() -> Self {
        Self {
            font_system: build_font_system(&[]),
            swash_cache: SwashCache::new(),
        }
    }

    pub fn with_font(font_data: Vec<u8>) -> Self {
        Self {
            font_system: build_font_system(&[font_data]),
            swash_cache: SwashCache::new(),
        }
    }

    pub fn caption_bytes(
        &mut self,
        data: &[u8],
        text: &str,
        output_format: OutputFormat,
    ) -> Result<Vec<u8>, CaptionError> {
        match detect_format(data)? {
            Format::Gif => self.caption_gif(data, output_format, text),
            Format::Static(input_fmt) => {
                self.caption_static(data, output_format.resolve(input_fmt), text)
            }
        }
    }

    fn caption_static(
        &mut self,
        data: &[u8],
        format: ImageFormat,
        text: &str,
    ) -> Result<Vec<u8>, CaptionError> {
        let _span = info_span!("caption_static").entered();

        let img = {
            let _span = info_span!("decode_image").entered();
            image::load_from_memory(data)?.into_rgba8()
        };
        let (img_w, img_h) = img.dimensions();

        if img_w == 0 || img_h == 0 {
            return Err(CaptionError::EmptyImage);
        }

        let bar = self.render_bar(text, img_w);
        let out_img = {
            let _span = info_span!("stack_bar").entered();
            stack_bar_over_image(img, &bar.rgba, bar.height).ok_or(CaptionError::EmptyImage)?
        };

        let _span = info_span!("encode_image", format = ?format).entered();
        encode_image(&out_img, format)
    }

    fn caption_gif(
        &mut self,
        data: &[u8],
        output_format: OutputFormat,
        text: &str,
    ) -> Result<Vec<u8>, CaptionError> {
        let _span = info_span!("caption_gif").entered();

        match output_format {
            OutputFormat::WebP => {
                let gif_data = decode_gif_for_webp(data)?;
                let img_w = gif_data.width as u32;
                let bar = self.render_bar(text, img_w);
                encode_webp(&gif_data, &bar.rgba, img_w, bar.height)
            }
            OutputFormat::Same | OutputFormat::Gif => {
                let gif_data = decode_gif(data)?;
                let img_w = gif_data.width as u32;
                let bar = self.render_bar(text, img_w);
                encode_gif(&gif_data, &bar.rgba, img_w, bar.height)
            }
            _ => Err(CaptionError::UnsupportedFormat(
                "GIF cannot be output as PNG".into(),
            )),
        }
    }

    fn render_bar(&mut self, text: &str, image_width: u32) -> CaptionBar {
        let _span = info_span!("render_bar", image_width).entered();

        let mut layout = build_layout(
            &mut self.font_system,
            &mut self.swash_cache,
            text,
            image_width,
        );

        let bar_h = layout.bar_height;
        let bar_rgba = rasterize_bar(&mut layout, &mut self.font_system, &mut self.swash_cache);

        CaptionBar {
            rgba: bar_rgba,
            height: bar_h,
        }
    }
}

impl Default for Captioner {
    fn default() -> Self {
        Self::new()
    }
}

enum Format {
    Gif,
    Static(ImageFormat),
}

fn detect_format(data: &[u8]) -> Result<Format, CaptionError> {
    if data.starts_with(b"GIF8") {
        return Ok(Format::Gif);
    }
    let fmt = image::guess_format(data)
        .map_err(|_| CaptionError::UnsupportedFormat("could not detect image format".into()))?;
    match fmt {
        ImageFormat::Gif => Ok(Format::Gif),
        other => Ok(Format::Static(other)),
    }
}

fn encode_image(img: &RgbaImage, format: ImageFormat) -> Result<Vec<u8>, CaptionError> {
    let mut buf = Cursor::new(Vec::new());
    let raw = img.as_raw();
    let (width, height) = img.dimensions();

    match format {
        ImageFormat::Png => {
            PngEncoder::new(&mut buf).write_image(raw, width, height, ExtendedColorType::Rgba8)?;
        }
        ImageFormat::Jpeg => {
            let mut rgb = Vec::with_capacity((width * height * 3) as usize);
            for px in raw.as_chunks::<4>().0 {
                rgb.extend_from_slice(&px[..3]);
            }
            JpegEncoder::new(&mut buf).encode(&rgb, width, height, ExtendedColorType::Rgb8)?;
        }
        ImageFormat::WebP => {
            WebPEncoder::new_lossless(&mut buf).write_image(
                raw,
                width,
                height,
                ExtendedColorType::Rgba8,
            )?;
        }
        other => {
            return Err(CaptionError::UnsupportedFormat(format!(
                "unsupported output format: {other:?}"
            )));
        }
    }

    Ok(buf.into_inner())
}

fn stack_bar_over_image(img: RgbaImage, bar_rgba: &[u8], bar_h: u32) -> Option<RgbaImage> {
    let (img_w, img_h) = img.dimensions();
    let mut canvas = vec![255u8; (img_w * (bar_h + img_h)) as usize * 4];
    canvas[..bar_rgba.len()].copy_from_slice(bar_rgba);

    let bar_offset = (img_w * bar_h) as usize * 4;
    let img_raw = img.into_raw();
    canvas[bar_offset..bar_offset + img_raw.len()].copy_from_slice(&img_raw);

    RgbaImage::from_raw(img_w, bar_h + img_h, canvas)
}
