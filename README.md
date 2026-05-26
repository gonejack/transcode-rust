# transcode

This command line tool does text file encoding conversions.

## Installation

```bash
cargo install --path .
```

## Usage

By arguments:
```bash
transcode source.txt
transcode -s gbk -t utf-8 source.txt
```

By stdin:
```bash
cat source.txt | transcode
```

## Flags

```
  -s, --source-encoding <SOURCE_ENCODING>  Set source encoding, default as auto-detection [default: auto]
  -t, --target-encoding <TARGET_ENCODING>  Set target encoding, default as utf-8 [default: utf-8]
  -d, --detect-encoding                    Detect encoding only
  -w, --overwrite                          Overwrite source file
  -l, --list-encodings                     List supported encodings
      --about                              Show about
  -h, --help                               Print help
```
