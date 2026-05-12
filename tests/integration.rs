use caption::{Captioner, OutputFormat};
use std::io::Cursor;
use std::process::Command;

type GifFrameRect = (u32, u32, u32, u32);

#[test]
fn static_caption_preserves_width_and_adds_bar() {
    let src = fixture("static_750x500.png");
    let mut captioner = Captioner::new();

    let out = captioner
        .caption_bytes(&src, "hello world", OutputFormat::Same)
        .unwrap();
    let img = image::load_from_memory(&out).unwrap();

    assert_eq!(img.width(), 750);
    assert!(img.height() > 500, "caption should add height");
    assert!(img.height() < 1000, "caption bar should stay reasonable");
}

#[test]
fn static_text_layout_responds_to_multiline_and_width() {
    let mut captioner = Captioner::new();

    let single = captioned_png_height(&mut captioner, "static_750x500.png", "hello world");
    let multiline = captioned_png_height(&mut captioner, "static_750x500.png", "hello\nworld");
    assert!(multiline > single, "explicit newline should add bar height");

    let small = captioned_png_height(&mut captioner, "static_500x500.png", "hello 🔥 world 🎉");
    let large = captioned_png_height(&mut captioner, "static_1280x720.png", "hello 🔥 world 🎉");
    assert!(
        large - 720 > small - 500,
        "larger images should use larger text"
    );
}

#[test]
fn gif_caption_preserves_animation_and_uses_full_canvas_frames() {
    let src = fixture("gif_complex_320x240_24f.gif");
    let mut captioner = Captioner::new();

    let out = captioner
        .caption_bytes(&src, "こんにちは\nworld", OutputFormat::Same)
        .unwrap();

    assert_eq!(&out[..6], b"GIF89a");
    let (canvas_w, canvas_h, descriptors) = gif_frame_descriptors(&out);
    assert_eq!(canvas_w, 320);
    assert!(canvas_h > 240);

    assert_eq!(descriptors.len(), 24, "frame count should be preserved");
    assert!(
        descriptors
            .iter()
            .all(|descriptor| *descriptor == (0, 0, canvas_w, canvas_h)),
        "all output frames should be full-canvas for Discord"
    );
}

#[test]
fn gif_can_be_output_as_animated_webp() {
    let src = fixture("gif_320x240_10f.gif");
    let mut captioner = Captioner::new();

    let out = captioner
        .caption_bytes(&src, "hello webp", OutputFormat::WebP)
        .unwrap();

    assert!(out.starts_with(b"RIFF"));
    assert_eq!(out.get(8..12), Some(&b"WEBP"[..]));
    assert!(image::load_from_memory(&out).is_ok());
}

#[test]
fn cli_default_output_is_written_to_current_directory() {
    let source = fixture_path("static_500x500.png");
    let work_dir = std::env::temp_dir().join(format!(
        "caption-image-cwd-output-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&work_dir).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_caption-image"))
        .current_dir(&work_dir)
        .args(["--input", source.to_str().unwrap(), "--text", "cwd output"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = work_dir.join("static_500x500_captioned.png");
    assert!(expected.exists(), "expected {}", expected.display());
    assert!(
        !source
            .with_file_name("static_500x500_captioned.png")
            .exists()
    );

    std::fs::remove_file(expected).unwrap();
    std::fs::remove_dir(work_dir).unwrap();
}

fn captioned_png_height(captioner: &mut Captioner, fixture_name: &str, text: &str) -> u32 {
    let out = captioner
        .caption_bytes(&fixture(fixture_name), text, OutputFormat::Same)
        .unwrap();
    image::load_from_memory(&out).unwrap().height()
}

fn fixture(name: &str) -> Vec<u8> {
    let path = fixture_path(name);
    std::fs::read(&path).unwrap_or_else(|_| panic!("missing fixture: {}", path.display()))
}

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sources")
        .join(name)
}

fn gif_frame_descriptors(data: &[u8]) -> (u32, u32, Vec<GifFrameRect>) {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::Indexed);

    let mut decoder = options.read_info(Cursor::new(data)).unwrap();
    let canvas_w = decoder.width() as u32;
    let canvas_h = decoder.height() as u32;
    let mut descriptors = Vec::new();

    while let Some(frame) = decoder.read_next_frame().unwrap() {
        descriptors.push((
            frame.left as u32,
            frame.top as u32,
            frame.width as u32,
            frame.height as u32,
        ));
    }

    (canvas_w, canvas_h, descriptors)
}
