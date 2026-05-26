use chardet::detect;
use clap::Parser;
use encoding_rs::Encoding;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};

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
    let result = detect(data);
    if result.0.is_empty() {
        None
    } else {
        Some(result.0.to_string())
    }
}

fn parse_encoding(name: &str) -> Option<&'static Encoding> {
    Encoding::for_label(name.as_bytes())
}

fn proc(
    file: &str,
    source_encoding: &str,
    target_encoding: &str,
    detect_only: bool,
    overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_enc = parse_encoding(target_encoding)
        .ok_or_else(|| format!("invalid target encoding: {target_encoding}"))?;

    let data = if file == "-" {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        buf
    } else {
        let meta = fs::metadata(file)?;
        if !meta.is_file() {
            return Err("not a regular file".into());
        }
        if meta.len() == 0 {
            eprintln!("no changes, source file {file} is empty");
            return Ok(());
        }
        fs::read(file)?
    };

    if detect_only {
        match detect_encoding(&data[..data.len().min(2048)]) {
            Some(enc) => println!("encoding of file {file} is {enc}"),
            None => eprintln!("detecting encoding of file {file} failed"),
        }
        return Ok(());
    }

    let source_enc = if source_encoding.eq_ignore_ascii_case("auto") {
        let name = detect_encoding(&data[..data.len().min(2048)])
            .ok_or("cannot determine source-encoding")?;
        parse_encoding(&name).ok_or_else(|| format!("unsupported detected encoding: {name}"))?
    } else {
        parse_encoding(source_encoding)
            .ok_or_else(|| format!("invalid source encoding: {source_encoding}"))?
    };

    if file != "-" && overwrite && source_enc == target_enc {
        eprintln!("no changes, source file {file} is already in target encoding");
        return Ok(());
    }

    let (decoded, _, _) = source_enc.decode(&data);
    let (encoded, _, _) = target_enc.encode(&decoded);

    if file != "-" && overwrite {
        let mut f = OpenOptions::new().write(true).truncate(true).open(file)?;
        f.write_all(&encoded)?;
    } else {
        io::stdout().write_all(&encoded)?;
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let files = if opts.files.is_empty() {
        vec!["-".to_string()]
    } else {
        opts.files.clone()
    };

    for file in &files {
        proc(
            file,
            &opts.source_encoding,
            &opts.target_encoding,
            opts.detect_encoding,
            opts.overwrite,
        )
        .map_err(|e| format!("process {file} failed: {e}"))?;
    }

    Ok(())
}

#[rustfmt::skip]
static ENCODINGS: &[&str] = &[
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
    "x-user-defined",
];
