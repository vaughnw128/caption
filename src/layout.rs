use cosmic_text::{Align, Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache};
use tracing::info_span;

/// Presets for text layout and font sizing.
const WRAP_FRACTION: f32 = 0.7;
const MIN_FONT_SIZE: f32 = 12.0;
const LINE_SPACING: i32 = 4;

pub struct LayoutResult {
    pub bar_width: u32,
    pub bar_height: u32,
    pub buffer: Buffer,
    pub line_positions: Vec<(i32, i32)>,
}

/// Compute optimal font size, shape text, return layout ready for rasterization.
pub fn build_layout(
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    text: &str,
    image_width: u32,
) -> LayoutResult {
    let _span = info_span!("build_layout", image_width).entered();

    let max_line_width = image_width as f32 * WRAP_FRACTION;
    let base_size = (image_width as f32 / 8.0).floor().max(MIN_FONT_SIZE);

    let font_size = {
        let _span = info_span!("binary_search_font_size", base_size).entered();
        binary_search_font_size(font_system, text, max_line_width, base_size)
    };

    let buffer = {
        let _span = info_span!("shape_text", font_size).entered();
        make_buffer(font_system, text, max_line_width, font_size)
    };

    let line_ink = {
        let _span = info_span!("measure_ink").entered();
        measure_per_line_ink(font_system, swash_cache, &buffer)
    };

    let n = line_ink.len() as i32;
    let total_text_height: i32 =
        line_ink.iter().map(|(h, _)| *h as i32).sum::<i32>() + LINE_SPACING * (n - 1).max(0);
    let half_pad = (font_size as i32) / 4;
    let bar_height = (total_text_height + half_pad * 2).max(1) as u32;

    let mut line_positions = Vec::with_capacity(line_ink.len());
    let mut cursor = half_pad;
    for (ink_h, ink_min_y) in &line_ink {
        line_positions.push((cursor, *ink_min_y));
        cursor += *ink_h as i32 + LINE_SPACING;
    }

    LayoutResult {
        bar_width: image_width,
        bar_height,
        buffer,
        line_positions,
    }
}

pub fn rasterize_bar(
    layout: &mut LayoutResult,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
) -> Vec<u8> {
    let _span = info_span!(
        "rasterize_bar",
        width = layout.bar_width,
        height = layout.bar_height
    )
    .entered();
    let w = layout.bar_width as usize;
    let h = layout.bar_height as usize;
    let mut buf = vec![255u8; w * h * 4];

    let margin = ((layout.bar_width as f32 * (1.0 - WRAP_FRACTION)) / 2.0).round() as i32;

    for (run_idx, run) in layout.buffer.layout_runs().enumerate() {
        let Some(&(ink_top_in_bar, ink_min_y)) = layout.line_positions.get(run_idx) else {
            continue;
        };
        let y_shift = ink_top_in_bar - ink_min_y;

        for glyph in run.glyphs.iter() {
            let physical = glyph.physical((margin as f32, 0.0), 1.0);
            let glyph_color = glyph.color_opt.unwrap_or(Color::rgb(0, 0, 0));

            swash_cache.with_pixels(
                font_system,
                physical.cache_key,
                glyph_color,
                |px, py, color| {
                    let x = physical.x + px;
                    let y = run.line_y as i32 + physical.y + py + y_shift;

                    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                        return;
                    }

                    let a = color.a() as u32;
                    if a == 0 {
                        return;
                    }

                    let idx = (y as usize * w + x as usize) * 4;
                    let inv_a = 255 - a;
                    buf[idx] = ((color.r() as u32 * a + buf[idx] as u32 * inv_a) / 255) as u8;
                    buf[idx + 1] =
                        ((color.g() as u32 * a + buf[idx + 1] as u32 * inv_a) / 255) as u8;
                    buf[idx + 2] =
                        ((color.b() as u32 * a + buf[idx + 2] as u32 * inv_a) / 255) as u8;
                    buf[idx + 3] = 255;
                },
            );
        }
    }

    buf
}

fn binary_search_font_size(
    font_system: &mut FontSystem,
    text: &str,
    max_line_width: f32,
    base_size: f32,
) -> f32 {
    if fits(font_system, text, max_line_width, base_size) {
        return base_size;
    }

    let min_size = (base_size / 4.0).max(MIN_FONT_SIZE);
    let mut lo = min_size;
    let mut hi = base_size;
    let mut best = min_size;

    while hi - lo > 0.5 {
        let mid = (lo + hi) / 2.0;
        if fits(font_system, text, max_line_width, mid) {
            best = mid;
            lo = mid;
        } else {
            hi = mid;
        }
    }

    best
}

fn fits(font_system: &mut FontSystem, text: &str, max_line_width: f32, size: f32) -> bool {
    let buf = make_buffer(font_system, text, max_line_width, size);
    for run in buf.layout_runs() {
        if run.line_w > max_line_width {
            return false;
        }
    }
    true
}

fn make_buffer(font_system: &mut FontSystem, text: &str, max_width: f32, font_size: f32) -> Buffer {
    let metrics = Metrics::new(font_size, font_size * 1.2);
    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_size(Some(max_width), None);

    let attrs = Attrs::new().family(Family::Name("Futura XBlkCn BT"));
    buffer.set_text(text, &attrs, Shaping::Advanced, Some(Align::Center));

    buffer.shape_until_scroll(font_system, false);

    buffer
}

fn measure_per_line_ink(
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    buffer: &Buffer,
) -> Vec<(u32, i32)> {
    let mut result = Vec::new();
    for run in buffer.layout_runs() {
        let line_y = run.line_y as i32;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;
        for glyph in run.glyphs.iter() {
            let physical = glyph.physical((0.0, 0.0), 1.0);
            if let Some(image) = swash_cache.get_image(font_system, physical.cache_key) {
                if image.placement.height == 0 {
                    continue;
                }
                let top = line_y - image.placement.top;
                let bottom = top + image.placement.height as i32 - 1;
                if top < min_y {
                    min_y = top;
                }
                if bottom > max_y {
                    max_y = bottom;
                }
            }
        }
        if min_y != i32::MAX {
            result.push(((max_y - min_y + 1).max(0) as u32, min_y));
        }
    }
    result
}
