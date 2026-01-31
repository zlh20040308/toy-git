use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::fmt;

use clap::{Parser, Subcommand};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::read::ZlibEncoder;
use sha1::{Digest, Sha1};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    CatFile {
        #[arg(short)]
        p: String,
    },
    HashObject {
        #[arg(short)]
        w: String,
    },
}

fn decode_reader(bytes: Vec<u8>) -> io::Result<String> {
    let mut z = ZlibDecoder::new(&bytes[..]);
    let mut s = String::new();
    z.read_to_string(&mut s)?;
    Ok(s)
}

struct Blob<'a> {
    size: u64,
    content: &'a str,
}

impl<'a> Blob<'a> {
    fn new(size: u64, content: &'a str) -> Self {
        Blob { size, content }
    }

}

impl<'a> fmt::Display for Blob<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "blob {}\0{}", self.size, self.content)
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Init) => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory")
        }
        Some(Commands::CatFile { p }) => {
            let mut file_path_string = ".git/objects/".to_owned();
            file_path_string.push_str(&p[0..2]);
            file_path_string.push('/');
            file_path_string.push_str(&p[2..]);
            let file_path = Path::new(&file_path_string);
            let mut buffer = vec![];
            let _ = fs::File::open(file_path).unwrap().read_to_end(&mut buffer);
            let decode = decode_reader(buffer).unwrap();
            let decode_content: Vec<&str> = decode.split('\0').collect();
            print!("{}", decode_content[1]);
        }
        Some(Commands::HashObject { w }) => {
            let mut file = fs::File::open(w).unwrap();

            let mut file_content = String::new();
            file.read_to_string(&mut file_content).unwrap();

            let blob = Blob::new(file.metadata().unwrap().len(), &file_content);

            // ---- compute object id (sha1) ----
            let mut sha1 = Sha1::new();
            sha1.update(blob.to_string());
            let digest = sha1.finalize();

            // hex object id
            let oid_tail = digest[1..]
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();

            let oid = format!("{:02x}{}", digest[0], oid_tail);
            println!("{}", oid);

            // ---- object storage path ----
            let object_dir = format!(".git/objects/{:02x}", digest[0]);

            match fs::create_dir(&object_dir) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                    std::process::exit(0);
                }
                Err(e) => panic!("{}", e),
            }

            let object_path = format!("{}/{}", object_dir, oid_tail);

            // ---- write compressed object ----
            let mut object_file = match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&object_path)
            {
                Ok(f) => f,
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                    std::process::exit(0);
                }
                Err(e) => panic!("{}", e),
            };

            let blob_data = blob.to_string();
            let mut encoder = ZlibEncoder::new(blob_data.as_bytes(), Compression::fast());

            let mut compressed = Vec::new();
            encoder.read_to_end(&mut compressed).unwrap();

            let _ = object_file.write(&compressed);
        }
        None => {
            println!("unknown command")
        }
    }
}
