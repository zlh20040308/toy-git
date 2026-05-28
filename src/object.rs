// 对象存储 — blob、tree、commit 的创建与读取
//
// Git 所有对象都以「类型 长度\0内容」格式拼接，SHA1 哈希后 zlib 压缩存入
// .git/objects/<前2位>/<后38位>。对象一旦写入就不可变。

use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use flate2::read::ZlibDecoder;
use flate2::read::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest, Sha1};

use crate::index::IndexEntry;

/// 解压 zlib 压缩的对象数据，返回文本形式
pub fn decode_reader(bytes: Vec<u8>) -> io::Result<String> {
    let mut z = ZlibDecoder::new(&bytes[..]);
    let mut s = String::new();
    z.read_to_string(&mut s)?;
    Ok(s)
}

/// 通用的对象存储函数。
///
/// Git 所有对象（blob、tree、commit、tag）存储方式都一样：
///   1. 拼接 "<type> <字节长度>\0<内容>"
///   2. 计算 SHA1
///   3. zlib 压缩后写入 .git/objects/<前2位>/<后38位>
///
/// 返回 (十六进制hash, 原始20字节SHA1)
pub fn store_object(object_type: &str, object_content: &[u8]) -> (String, [u8; 20]) {
    // 1. 拼接头部："<type> <size>\0"
    let header = format!("{} {}\0", object_type, object_content.len());
    let mut full_data = header.into_bytes();
    full_data.extend_from_slice(object_content);

    // 2. 计算 SHA1
    let mut hasher = Sha1::new();
    hasher.update(&full_data);
    let digest = hasher.finalize();

    let mut sha1_bytes = [0u8; 20];
    sha1_bytes.copy_from_slice(&digest);

    // 3. 转十六进制 hash
    let oid_tail: String = digest[1..]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    let oid = format!("{:02x}{}", digest[0], oid_tail);

    // 4. 存入 .git/objects/<前2位>/<后38位>
    let object_dir = format!(".git/objects/{:02x}", digest[0]);
    let _ = fs::create_dir(&object_dir);

    let object_path = format!("{}/{}", object_dir, oid_tail);
    if !Path::new(&object_path).exists() {
        let mut encoder = ZlibEncoder::new(&full_data[..], Compression::fast());
        let mut compressed = Vec::new();
        encoder.read_to_end(&mut compressed).unwrap();

        let mut object_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&object_path)
            .unwrap();
        object_file.write_all(&compressed).unwrap();
    }

    (oid, sha1_bytes)
}

/// 将 Unix stat 返回的模式值，转换为 tree 对象中使用的规范化模式。
///
/// Git 在 tree 对象中只使用以下几种模式：
///   100644 — 普通文件（不可执行）
///   100755 — 普通文件（可执行）
///   40000  — 目录
pub fn normalize_tree_mode(mode: u32) -> String {
    if mode & 0o111 != 0 {
        "100755".to_string()
    } else {
        "100644".to_string()
    }
}

/// 计算文件的 blob hash，并存入 .git/objects。
/// 返回 (十六进制hash, 原始20字节SHA1) —— index 和 tree 都需要原始字节。
pub fn hash_and_store_blob(file_path: &str) -> io::Result<(String, [u8; 20])> {
    let mut file = fs::File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Ok(store_object("blob", content.as_bytes()))
}

/// 将 index 条目列表构建为一棵 tree 对象，存入 .git/objects。
///
/// Tree 对象内容为一组排序后的 entry，每个 entry 格式：
///   <模式> <路径>\0<20字节原始SHA1>
///
/// entry 之间没有分隔符，读取时靠 \0 定位。
pub fn write_tree(entries: &[IndexEntry]) -> (String, [u8; 20]) {
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

    store_object("tree", &tree_content)
}

/// 构建并存储一个 commit 对象。
///
/// Commit 对象的内容是纯文本，格式：
///   tree <tree-hash>
///   parent <parent-hash>        ← 仅首次提交时省略此行
///   author <name> <email> <timestamp> <timezone>
///   committer <name> <email> <timestamp> <timezone>
///
///   <commit message>
pub fn create_commit(
    tree_hash: &str,
    parent_hash: Option<&str>,
    message: &str,
) -> (String, [u8; 20]) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut content = format!("tree {}\n", tree_hash);
    if let Some(parent) = parent_hash {
        content.push_str(&format!("parent {}\n", parent));
    }
    content.push_str(&format!(
        "author toy-git <toy@git> {} +0800\n",
        timestamp
    ));
    content.push_str(&format!(
        "committer toy-git <toy@git> {} +0800\n",
        timestamp
    ));
    content.push('\n');
    content.push_str(message);
    content.push('\n');

    store_object("commit", content.as_bytes())
}
