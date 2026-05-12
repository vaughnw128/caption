use caption::Captioner;

#[allow(dead_code)]
const TEXTS: &[(&str, &str)] = &[
    ("short", "hello world"),
    (
        "long",
        "this is a much longer caption that should wrap across multiple lines",
    ),
    ("emoji", "hello 🔥 world 🎉"),
    ("japanese", "こんにちは世界"),
];

#[allow(dead_code)]
fn bench_caption(
    b: divan::Bencher,
    fixture: &str,
    text: &str,
    output_format: caption::OutputFormat,
) {
    let data = source(fixture);
    let mut c = Captioner::new();
    b.bench_local(|| c.caption_bytes(&data, text, output_format).unwrap());
}

#[allow(dead_code)]
fn source(name: &str) -> Vec<u8> {
    let path = fixture_path(name);
    std::fs::read(&path).unwrap_or_else(|_| panic!("missing fixture: {}", path.display()))
}

#[allow(dead_code)]
fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
