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
use std::path::Path;

use clap::{Parser, Subcommand};

use object::{create_commit, decode_reader, hash_and_store_blob, write_tree};
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
    /// [plumbing] 创建一个 commit 对象（不更新分支引用）
    CommitTree {
        /// tree 对象的 hash
        #[arg(short)]
        t: String,
        /// 父 commit 的 hash（首次提交可省略）
        #[arg(short)]
        p: Option<String>,
        /// 提交信息
        #[arg(short)]
        m: String,
    },
    /// [porcelain] 将暂存区创建为一次提交（= write-tree + commit-tree + 更新 HEAD）
    Commit {
        /// 提交信息
        #[arg(short)]
        m: String,
    },
}

// ===== HEAD / 分支引用 =====
//
// .git/HEAD 的内容是一个符号引用，指向当前分支：
//   ref: refs/heads/main
//
// .git/refs/heads/main 存储的是该分支最新的 commit hash。
// 如果这个文件不存在，说明还没有任何提交。

/// 读取 HEAD 指向的引用路径（如 "refs/heads/main"），不包含 "ref: " 前缀
fn read_head_ref_path() -> String {
    let head = fs::read_to_string(".git/HEAD").unwrap();
    head.trim()
        .strip_prefix("ref: ")
        .unwrap()
        .to_string()
}

/// 读取当前分支的最新 commit hash。如果还没有提交，返回 None。
fn get_parent_commit() -> Option<String> {
    let ref_path = read_head_ref_path();
    fs::read_to_string(format!(".git/{}", ref_path))
        .ok()
        .map(|s| s.trim().to_string())
}

/// 更新当前分支的引用，指向新的 commit hash。
fn update_head_ref(commit_hash: &str) {
    let ref_path = read_head_ref_path();
    let full_path = format!(".git/{}", ref_path);
    if let Some(parent_dir) = Path::new(&full_path).parent() {
        fs::create_dir_all(parent_dir).unwrap();
    }
    fs::write(&full_path, format!("{}\n", commit_hash)).unwrap();
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
            let (_, sha1_bytes) = hash_and_store_blob(&file).unwrap();

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

            let (oid, _) = write_tree(&entries);
            println!("{}", oid);
        }

        // --- commit-tree (plumbing) ---
        //
        // 只做一件事：把已有的 tree hash 包装成 commit 对象。
        // 不做 write-tree，不更新分支引用。这是最纯粹的 commit 创建操作。
        //
        // 真实 git 中，commit-tree 通常从 stdin 或环境变量读取 author 信息，
        // 这里简化为固定的 "toy-git"。
        Some(Commands::CommitTree { t, p, m }) => {
            let parent = p.as_deref();
            let (commit_hash, _) = create_commit(&t, parent, &m);

            // 判断是否首次提交（没有父提交就是 root commit）
            let root_msg = if p.is_none() { " (root-commit)" } else { "" };
            println!("[commit{root_msg}] {}", commit_hash);
        }

        // --- commit (porcelain) ---
        //
        // 高层命令，组合了三个底层操作：
        //   1. write-tree     → 将暂存区拍成 tree 对象
        //   2. commit-tree    → 将 tree 包装成 commit 对象
        //   3. update-ref     → 更新分支引用，指向新的 commit
        Some(Commands::Commit { m }) => {
            // 1. write-tree：把暂存区写成 tree
            let entries = read_index().unwrap();

            if entries.is_empty() {
                eprintln!("nothing to commit");
                std::process::exit(1);
            }

            let (tree_hash, _) = write_tree(&entries);

            // 2. commit-tree：创建 commit 对象
            let parent_hash = get_parent_commit();
            let (commit_hash, _) = create_commit(&tree_hash, parent_hash.as_deref(), &m);

            // 3. update-ref：更新分支引用
            update_head_ref(&commit_hash);

            let root_msg = if parent_hash.is_none() { " (root-commit)" } else { "" };
            println!("[main{root_msg}] {}", commit_hash);
        }

        None => {
            println!("unknown command");
        }
    }
}
