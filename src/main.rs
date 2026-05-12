use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, ValueEnum};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::uptime;

use caption::{Captioner, OutputFormat};

/// Fast image and GIF captioning tool.
#[derive(Parser, Debug)]
#[command(name = "caption", version, about)]
struct Args {
    /// Path to the input image or GIF.
    #[arg(short, long)]
    input: PathBuf,

    /// Caption text to render above the image.
    #[arg(short, long)]
    text: String,

    /// Output file path. Defaults to ./<input_stem>_captioned.<ext>.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format. Defaults to the same format as the input.
    #[arg(short, long, value_enum, default_value_t = CliFormat::Same)]
    format: CliFormat,

    /// Path to a custom TTF/OTF font file to use instead of the default.
    #[arg(long)]
    font: Option<PathBuf>,

    /// Print timing spans for each pipeline stage to stderr.
    #[arg(long = "trace")]
    verbose: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy, Default)]
enum CliFormat {
    #[default]
    Same,
    Png,
    Jpeg,
    Webp,
}

impl From<CliFormat> for OutputFormat {
    fn from(f: CliFormat) -> Self {
        match f {
            CliFormat::Same => OutputFormat::Same,
            CliFormat::Png => OutputFormat::Png,
            CliFormat::Jpeg => OutputFormat::Jpeg,
            CliFormat::Webp => OutputFormat::WebP,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.verbose {
        tracing_subscriber::fmt()
            .pretty()
            .with_timer(uptime())
            .with_env_filter(trace_filter())
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
            .with_writer(std::io::stderr)
            .init();
    }

    let mut captioner = if let Some(font_path) = args.font {
        let font_data =
            std::fs::read(&font_path).with_context(|| format!("reading font {font_path:?}"))?;
        Captioner::with_font(font_data)
    } else {
        Captioner::new()
    };

    let data =
        std::fs::read(&args.input).with_context(|| format!("reading input {:?}", args.input))?;

    let output_format = OutputFormat::from(args.format);
    let out_bytes = captioner
        .caption_bytes(&data, &args.text, output_format)
        .with_context(|| "captioning failed")?;

    let output_path = args.output.unwrap_or_else(|| {
        let stem = args
            .input
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let ext = match output_format {
            OutputFormat::Same => detected_output_extension(&out_bytes)
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    args.input
                        .extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                }),
            fmt => fmt.extension().to_string(),
        };
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(format!("{stem}_captioned.{ext}"))
    });

    std::fs::write(&output_path, &out_bytes)
        .with_context(|| format!("writing output {output_path:?}"))?;

    println!("{}", output_path.display());

    Ok(())
}

fn trace_filter() -> EnvFilter {
    match std::env::var("RUST_LOG") {
        Ok(value) if !value.trim().is_empty() => {
            EnvFilter::try_new(value).unwrap_or_else(|_| EnvFilter::new("caption=trace"))
        }
        _ => EnvFilter::try_new("caption=trace").expect("hard-coded trace filter is valid"),
    }
}

fn detected_output_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
        Some("png")
    } else if bytes.starts_with(b"GIF8") {
        Some("gif")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("webp")
    } else {
        None
    }
}
