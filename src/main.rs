use chrono::prelude::*;
use clap::Parser;
use filetime::FileTime;
use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};
use zip::{write::FileOptions, ZipWriter};

fn main() {
    let args = Cli::parse();
    let out_string = args.output.clone().unwrap_or(String::from("output.zip"));
    let out_path = PathBuf::from(out_string);
    let file = File::create(out_path.clone()).expect("Failed to write to output file");
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let mut state = State {
        buffer: Vec::new(),
        floor_time: args.parse_last_modified(),
        current_dir: std::env::current_dir().unwrap(),
        output_file: out_path,
        zip: ZipWriter::new(file),
        options,
    };
    let _ = state.walk_files(&state.current_dir.clone());
}

#[derive(Parser)]
struct Cli {
    /// The string representing the last modified date. Date should be of the form
    /// "%Y-%m-%d %H:%H:%S"
    last_modifed_date: String,

    /// Optional output location for the .zip archive. If none is specified, this will default to
    /// output.zip in the working directory.
    #[arg(short, long)]
    output: Option<String>,
}

struct State<W: Write + Seek> {
    buffer: Vec<u8>,
    output_file: PathBuf,
    current_dir: PathBuf,
    floor_time: i64,
    zip: ZipWriter<W>,
    options: FileOptions,
}

impl<W> State<W>
where
    W: Write + Seek,
{
    fn zip_file(&mut self, path: &Path) {
        self.zip.start_file(path.to_string_lossy(), self.options);
        let mut f = File::open(path).unwrap();
        f.read_to_end(&mut self.buffer);
        self.zip.write_all(&self.buffer);
        self.buffer.clear();
    }

    fn walk_files(&mut self, dir: &Path) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.walk_files(&path)?;
            } else {
                if self.file_is_newer(&path) && path != self.output_file {
                    let p = path.strip_prefix(&self.current_dir).unwrap();
                    self.zip_file(p);
                }
            }
        }
        Ok(())
    }

    fn file_is_newer(&self, path: &PathBuf) -> bool {
        let metadata = std::fs::metadata(path).unwrap();
        let t = FileTime::from_last_modification_time(&metadata).unix_seconds();
        self.floor_time < t
    }
}

impl Cli {
    const TIMESTAMP_FMT: &str = "%Y-%m-%d %H:%M:%S";

    fn parse_last_modified(&self) -> i64 {
        let time = Utc
            .datetime_from_str(&self.last_modifed_date, Self::TIMESTAMP_FMT)
            .expect("Bad datetime format");
        time.timestamp()
    }
}
