use anyhow::{Result, anyhow, bail};
use chardet::detect;
use clap::Parser;
use encoding_rs::Encoding;
use encoding_rs_rw::{DecodingReader, EncodingWriter};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Cursor, Read, Seek, Write};

const DETECTION_SAMPLE_SIZE: u64 = 2048;

#[derive(Parser)]
#[command(name = "transcode", about = "Translate text encoding.")]
struct Options {
    /// Set source encoding, default as auto-detection.
    #[arg(short = 's', long, default_value = "auto")]
    source_encoding: String,

    /// Set target encoding, default as utf8.
    #[arg(short = 't', long, default_value = "utf-8")]
    target_encoding: String,

    /// Detect encoding only.
    #[arg(short = 'd', long)]
    detect_encoding: bool,

    /// Overwrite source file.
    #[arg(short = 'w', long)]
    overwrite: bool,

    /// List supported encodings.
    #[arg(short = 'l', long)]
    list_encodings: bool,

    /// Show about.
    #[arg(long)]
    about: bool,

    #[arg(value_name = "FILE")]
    files: Vec<String>,
}

fn detect_encoding(data: &[u8]) -> Option<String> {
    let (name, _, _) = detect(data);
    if name.is_empty() {
        return None;
    }

    Some(if name.eq_ignore_ascii_case("GB2312") { "GB18030".into() } else { name })
}

fn parse_encoding(name: &str) -> Option<&'static Encoding> {
    Encoding::for_label(name.as_bytes())
}

fn proc(file: &str, source_encoding: &str, target_encoding: &str, detect_only: bool, overwrite: bool) -> Result<()> {
    let target_enc = parse_encoding(target_encoding).ok_or_else(|| anyhow!("invalid target encoding: {target_encoding}"))?;

    let mut input: Box<dyn Read> = if file == "-" {
        Box::new(io::stdin())
    } else {
        let meta = fs::metadata(file)?;
        if !meta.is_file() {
            bail!("not a regular file");
        }
        if meta.len() == 0 {
            eprintln!("no changes, source file {file} is empty");
            return Ok(());
        }
        Box::new(File::open(file)?)
    };

    let mut prefix = Vec::new();
    if detect_only || source_encoding.eq_ignore_ascii_case("auto") {
        input.by_ref().take(DETECTION_SAMPLE_SIZE).read_to_end(&mut prefix)?;
    }

    if detect_only {
        let Some(encoding) = detect_encoding(&prefix) else {
            eprintln!("detecting encoding of file {file} failed");
            return Ok(());
        };

        println!("encoding of file {file} is {encoding}");
        return Ok(());
    }

    let source_enc = if source_encoding.eq_ignore_ascii_case("auto") {
        let Some(name) = detect_encoding(&prefix) else {
            bail!("cannot determine source-encoding");
        };

        parse_encoding(&name).ok_or_else(|| anyhow!("unsupported detected encoding: {name}"))?
    } else {
        parse_encoding(source_encoding).ok_or_else(|| anyhow!("invalid source encoding: {source_encoding}"))?
    };

    if file != "-" && overwrite && source_enc == target_enc {
        eprintln!("no changes, source file {file} is already in target encoding");
        return Ok(());
    }

    let input = BufReader::new(Cursor::new(prefix).chain(input));

    if file != "-" && overwrite {
        let mut tmp = tempfile::tempfile()?;
        transcode_stream(input, &mut tmp, source_enc, target_enc)?;
        tmp.rewind()?;

        let mut output = OpenOptions::new().write(true).truncate(true).open(file)?;
        io::copy(&mut tmp, &mut output)?;
        output.flush()?;
    } else {
        let stdout = io::stdout();
        transcode_stream(input, stdout.lock(), source_enc, target_enc)?;
    }

    Ok(())
}

fn transcode_stream(
    input: impl BufRead,
    output: impl Write,
    source_enc: &'static Encoding,
    target_enc: &'static Encoding,
) -> Result<()> {
    let mut decoder = DecodingReader::new(input, source_enc.new_decoder());
    let mut encoder = EncodingWriter::new(output, target_enc.new_encoder());

    {
        let mut decoded = decoder.lossy();
        let mut encoded = encoder.with_unmappable_handler(|error, output| {
            write!(output, "&#{};", u32::from(error.value()))
        });
        io::copy(&mut decoded, &mut encoded)?;
        encoded.flush()?;
    }

    encoder.finish().map_err(|_| anyhow!("failed to finish target encoding"))?;
    Ok(())
}

fn main() -> Result<()> {
    let opts = Options::parse();

    if opts.about {
        println!("Visit https://github.com/gonejack/transcode-rust");
        return Ok(());
    }

    if opts.list_encodings {
        println!("Supported encodings:");
        for name in ENCODINGS {
            println!("{name}");
        }
        return Ok(());
    }

    let files = opts.files.iter().map(String::as_str).chain(opts.files.is_empty().then_some("-"));

    for file in files {
        proc(file, &opts.source_encoding, &opts.target_encoding, opts.detect_encoding, opts.overwrite)
            .map_err(|e| anyhow!("process {file} failed: {e}"))?;
    }

    Ok(())
}

#[rustfmt::skip]
const ENCODINGS: &[&str] = &[
    "unicode-1-1-utf-8", "utf-8", "utf8",
    "866", "cp866", "ibm866",
    "iso-8859-2", "iso-8859-3", "iso-8859-4", "iso-8859-5",
    "iso-8859-6", "iso-8859-7", "iso-8859-8", "iso-8859-8-i",
    "iso-8859-10", "iso-8859-13", "iso-8859-14", "iso-8859-15", "iso-8859-16",
    "koi8-r", "koi8-u",
    "macintosh", "x-mac-cyrillic",
    "windows-874", "windows-1250", "windows-1251", "windows-1252",
    "windows-1253", "windows-1254", "windows-1255", "windows-1256",
    "windows-1257", "windows-1258",
    "gbk", "gb18030",
    "big5",
    "euc-jp", "iso-2022-jp", "shift_jis",
    "euc-kr",
    "utf-16be", "utf-16le", "utf-16",
];
