// 这是一个玩具 Git 实现，用于理解 Git 的核心原理。
//
// Git 本质上是一个「内容寻址文件系统」—— 任何内容存入 Git 后，都会生成一个
// 唯一的 SHA1 哈希值作为"键"，之后可以用这个哈希值取出内容。
//
// Git 的核心对象类型：
//   blob   — 文件内容（不含文件名）
//   tree   — 目录快照（文件名 + 模式 + 指向 blob/subtree 的指针）
//   commit — 一次提交（指向 tree + 作者 + 提交信息 + 父 commit）
//
// 对象存储格式："<类型> <字节长度>\0<内容>" → SHA1 → zlib 压缩 → .git/objects/

mod object;
mod index;

use std::fs;
use std::io::Read;

use clap::{Parser, Subcommand};

use object::{decode_reader, hash_and_store_blob, normalize_tree_mode, store_object};
use index::{read_index, write_index, IndexEntry};

// ===== CLI =====

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 初始化一个新的 Git 仓库
    Init,
    /// 根据 hash 读取并打印对象内容
    CatFile {
        #[arg(short)]
        p: String,
    },
    /// 计算文件的 blob hash 并存入 .git/objects
    HashObject {
        #[arg(short)]
        w: String,
    },
    /// 将文件加入暂存区（等同于 git add 的底层操作）
    UpdateIndex {
        file: String,
    },
    /// 将暂存区的内容写为一个 tree 对象
    WriteTree,
}

// ===== main =====

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Init) => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory");
        }

        Some(Commands::CatFile { p }) => {
            // 对象路径：.git/objects/<前2位>/<后38位>
            let mut path = ".git/objects/".to_owned();
            path.push_str(&p[0..2]);
            path.push('/');
            path.push_str(&p[2..]);

            let mut buffer = vec![];
            let _ = fs::File::open(&path).unwrap().read_to_end(&mut buffer);
            let decode = decode_reader(buffer).unwrap();

            // 解压后格式：<type> <size>\0<content>
            let parts: Vec<&str> = decode.split('\0').collect();
            print!("{}", parts[1]);
        }

        Some(Commands::HashObject { w }) => {
            let (oid, _) = hash_and_store_blob(&w).unwrap();
            println!("{}", oid);
        }

        Some(Commands::UpdateIndex { file }) => {
            // 1. 存 blob
            let (_oid, sha1_bytes) = hash_and_store_blob(&file).unwrap();

            // 2. 收集文件元数据
            use std::os::unix::fs::MetadataExt;
            let meta = fs::metadata(&file).unwrap();

            let flags = {
                let len = file.len();
                if len > 0xFFF { 0xFFF } else { len as u16 }
            };

            let entry = IndexEntry {
                ctime_sec: meta.ctime() as u32,
                ctime_nsec: meta.ctime_nsec() as u32,
                mtime_sec: meta.mtime() as u32,
                mtime_nsec: meta.mtime_nsec() as u32,
                dev: meta.dev() as u32,
                ino: meta.ino() as u32,
                mode: meta.mode(),
                uid: meta.uid(),
                gid: meta.gid(),
                size: meta.len() as u32,
                sha1: sha1_bytes,
                flags,
                path: file.clone(),
            };

            // 3. 更新或追加到 index
            let mut entries = read_index().unwrap();
            if let Some(existing) = entries.iter_mut().find(|e| e.path == file) {
                *existing = entry;
            } else {
                entries.push(entry);
            }
            write_index(&entries).unwrap();
        }

        Some(Commands::WriteTree) => {
            let entries = read_index().unwrap();

            if entries.is_empty() {
                eprintln!("nothing to write");
                std::process::exit(1);
            }

            // 按路径名排序，构建 tree entry
            let mut sorted: Vec<&IndexEntry> = entries.iter().collect();
            sorted.sort_by(|a, b| a.path.cmp(&b.path));

            let mut tree_content: Vec<u8> = Vec::new();
            for entry in &sorted {
                let mode_str = normalize_tree_mode(entry.mode);
                tree_content.extend_from_slice(mode_str.as_bytes());
                tree_content.push(b' ');
                tree_content.extend_from_slice(entry.path.as_bytes());
                tree_content.push(0);
                tree_content.extend_from_slice(&entry.sha1);
            }

            let (oid, _) = store_object("tree", &tree_content);
            println!("{}", oid);
        }

        None => {
            println!("unknown command");
        }
    }
}
