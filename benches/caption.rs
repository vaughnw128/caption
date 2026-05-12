include!("bench_common.rs");

#[divan::bench(args = TEXTS)]
fn static_500x500(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/static_500x500.png",
        text,
        caption::OutputFormat::Same,
    );
}

#[divan::bench(args = TEXTS)]
fn static_750x500(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/static_750x500.png",
        text,
        caption::OutputFormat::Same,
    );
}

#[divan::bench(args = TEXTS)]
fn static_1280x720(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/static_1280x720.png",
        text,
        caption::OutputFormat::Same,
    );
}

#[divan::bench(args = TEXTS)]
fn gif_10f(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/gif_320x240_10f.gif",
        text,
        caption::OutputFormat::Same,
    );
}

#[divan::bench(args = TEXTS)]
fn gif_30f(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/gif_320x240_30f.gif",
        text,
        caption::OutputFormat::Same,
    );
}

#[divan::bench(args = TEXTS)]
fn gif_60f(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/gif_320x240_60f.gif",
        text,
        caption::OutputFormat::Same,
    );
}

#[divan::bench(args = TEXTS)]
fn gif_120f(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/gif_320x240_120f.gif",
        text,
        caption::OutputFormat::Same,
    );
}

#[divan::bench(args = TEXTS)]
fn gif_1280x720_10f(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/gif_1280x720_10f.gif",
        text,
        caption::OutputFormat::Same,
    );
}

#[divan::bench(args = TEXTS)]
fn gif_1280x720_30f(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/gif_1280x720_30f.gif",
        text,
        caption::OutputFormat::Same,
    );
}

#[divan::bench(args = TEXTS)]
fn gif_1280x720_60f(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/gif_1280x720_60f.gif",
        text,
        caption::OutputFormat::Same,
    );
}

#[divan::bench(args = TEXTS)]
fn gif_complex_24f(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/gif_complex_320x240_24f.gif",
        text,
        caption::OutputFormat::Same,
    );
}

#[divan::bench(args = TEXTS)]
fn gif_photoish_18f(b: divan::Bencher, (_name, text): &(&str, &str)) {
    bench_caption(
        b,
        "sources/gif_photoish_256x256_18f.gif",
        text,
        caption::OutputFormat::Same,
    );
}

fn main() {
    divan::main();
}
