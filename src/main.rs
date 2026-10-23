use anyhow::{Context, Result, anyhow, ensure};
use chardet::detect;
use clap::Parser;
use encoding_rs::Encoding;
use encoding_rs_rw::{DecodingReader, EncodingWriter};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Cursor, Read, Seek, Write};

const SAMPLE_SIZE: u64 = 2048;
type Enc = &'static Encoding;

#[derive(Parser)]
#[command(name = "transcode", about = "Translate text encoding.")]
struct Options {
    /// Set source encoding, default as auto-detection.
    #[arg(short, long, default_value = "auto")]
    source_encoding: String,
    /// Set target encoding, default as utf8.
    #[arg(short, long, default_value = "utf-8")]
    target_encoding: String,
    /// Detect encoding only.
    #[arg(short, long)]
    detect_encoding: bool,
    /// Overwrite source file.
    #[arg(short = 'w', long)]
    overwrite: bool,
    /// List supported encodings.
    #[arg(short, long)]
    list_encodings: bool,
    /// Show about.
    #[arg(long)]
    about: bool,
    #[arg(value_name = "FILE")]
    files: Vec<String>,
}

fn detect_encoding(data: &[u8]) -> Option<String> {
    let (name, ..) = detect(data);
    if name.eq_ignore_ascii_case("gb2312") {
        return Some("GB18030".into());
    }
    (!name.is_empty()).then_some(name)
}

fn encoding(name: &str) -> Result<Enc> {
    Encoding::for_label(name.as_bytes()).ok_or_else(|| anyhow!("unsupported encoding: {name}"))
}

fn proc(opts: &Options, file: &str) -> Result<()> {
    let target = encoding(&opts.target_encoding)?;
    let mut input: Box<dyn Read> = if file == "-" {
        Box::new(io::stdin())
    } else {
        let meta = fs::metadata(file)?;
        ensure!(meta.is_file(), "not a regular file");
        if meta.len() == 0 {
            eprintln!("no changes, source file {file} is empty");
            return Ok(());
        }
        Box::new(File::open(file)?)
    };

    let auto = opts.source_encoding.eq_ignore_ascii_case("auto");
    let mut sample = Vec::new();
    if auto || opts.detect_encoding {
        input.by_ref().take(SAMPLE_SIZE).read_to_end(&mut sample)?;
    }

    if opts.detect_encoding {
        let Some(name) = detect_encoding(&sample) else {
            eprintln!("detecting encoding of file {file} failed");
            return Ok(());
        };
        println!("encoding of file {file} is {name}");
        return Ok(());
    }

    let source = if auto {
        let name = detect_encoding(&sample).context("cannot determine source-encoding")?;
        encoding(&name)?
    } else {
        encoding(&opts.source_encoding)?
    };

    let in_place = file != "-" && opts.overwrite;
    if in_place && source == target {
        eprintln!("no changes, source file {file} is already in target encoding");
        return Ok(());
    }

    let input = BufReader::new(Cursor::new(sample).chain(input));
    if in_place {
        let mut tmp = tempfile::tempfile()?;
        transcode(input, &mut tmp, source, target)?;
        tmp.rewind()?;
        let mut output = File::options().write(true).truncate(true).open(file)?;
        io::copy(&mut tmp, &mut output)?;
        output.flush()?;
    } else {
        transcode(input, io::stdout().lock(), source, target)?;
    }

    Ok(())
}

fn transcode(input: impl BufRead, output: impl Write, src: Enc, dst: Enc) -> Result<()> {
    let mut decoder = DecodingReader::new(input, src.new_decoder());
    let mut enc = EncodingWriter::new(output, dst.new_encoder());
    let mut decoded = decoder.lossy();
    let mut out = enc.with_unmappable_handler(|e, w| write!(w, "&#{};", u32::from(e.value())));
    io::copy(&mut decoded, &mut out)?;
    out.flush()?;
    drop(out);
    ensure!(enc.finish().is_ok(), "failed to finish target encoding");
    Ok(())
}

fn main() -> Result<()> {
    let opts = Options::parse();

    if opts.about {
        println!("Visit https://github.com/gonejack/transcode-rust");
        return Ok(());
    }

    if opts.list_encodings {
        println!("Supported encodings:\n{}", ENCODINGS.join("\n"));
        return Ok(());
    }

    let fallback = opts.files.is_empty().then_some("-");
    let mut files = opts.files.iter().map(String::as_str).chain(fallback);
    files.try_for_each(|f| proc(&opts, f).with_context(|| format!("process {f} failed")))
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
