// 暂存区（staging area）— .git/index 文件的读写
//
// .git/index 是一个二进制文件，记录当前暂存了哪些文件及其 blob hash。
// 它不是 git 对象，而是"下一步要提交的快照清单"。
//
// index 二进制格式（大端序 / network byte order）：
//
// ┌────────────────────────────────┐
// │  HEADER (12 bytes)             │
// │  - 签名 "DIRC" (4 bytes)       │
// │  - version (4 bytes, 通常是 2)  │
// │  - entry 数量  (4 bytes)       │
// ├────────────────────────────────┤
// │  ENTRY (每条长度可变, 8字节对齐) │
// │  - ctime 秒   (4)              │
// │  - ctime 纳秒 (4)              │
// │  - mtime 秒   (4)              │
// │  - mtime 纳秒 (4)              │
// │  - dev  (4)                    │
// │  - ino  (4)                    │
// │  - mode (4)  ← 文件权限/类型    │
// │  - uid  (4)                    │
// │  - gid  (4)                    │
// │  - size (4)  ← 文件大小         │
// │  - sha1 (20) ← blob 的原始hash  │
// │  - flags(2)  ← 路径长度等标志    │
// │  - path (变长, \0 结尾)         │
// │  - padding (补齐到8的倍数)       │
// └────────────────────────────────┘

use std::fs;
use std::io;
use std::io::ErrorKind;
use std::path::Path;

/// 暂存区中的一条记录，对应一个被 git add 的文件
pub struct IndexEntry {
    pub ctime_sec: u32,
    pub ctime_nsec: u32,
    pub mtime_sec: u32,
    pub mtime_nsec: u32,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub sha1: [u8; 20],
    pub flags: u16,
    pub path: String,
}

/// 从字节流中读取大端序的 u32
fn read_u32_be(data: &[u8], pos: &mut usize) -> u32 {
    let bytes: [u8; 4] = data[*pos..*pos + 4].try_into().unwrap();
    *pos += 4;
    u32::from_be_bytes(bytes)
}

/// 从字节流中读取大端序的 u16
fn read_u16_be(data: &[u8], pos: &mut usize) -> u16 {
    let bytes: [u8; 2] = data[*pos..*pos + 2].try_into().unwrap();
    *pos += 2;
    u16::from_be_bytes(bytes)
}

/// 从 .git/index 读取暂存区。文件不存在时返回空列表。
pub fn read_index() -> io::Result<Vec<IndexEntry>> {
    let path = Path::new(".git/index");
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };

    let mut pos = 0;

    // 解析 Header
    if &data[pos..pos + 4] != b"DIRC" {
        return Err(io::Error::new(ErrorKind::InvalidData, "bad index signature"));
    }
    pos += 4;

    let _version = read_u32_be(&data, &mut pos);
    let entry_count = read_u32_be(&data, &mut pos) as usize;

    let mut entries = Vec::with_capacity(entry_count);

    for _ in 0..entry_count {
        let entry_start = pos;

        let entry = IndexEntry {
            ctime_sec: read_u32_be(&data, &mut pos),
            ctime_nsec: read_u32_be(&data, &mut pos),
            mtime_sec: read_u32_be(&data, &mut pos),
            mtime_nsec: read_u32_be(&data, &mut pos),
            dev: read_u32_be(&data, &mut pos),
            ino: read_u32_be(&data, &mut pos),
            mode: read_u32_be(&data, &mut pos),
            uid: read_u32_be(&data, &mut pos),
            gid: read_u32_be(&data, &mut pos),
            size: read_u32_be(&data, &mut pos),
            sha1: {
                let mut sha1 = [0u8; 20];
                sha1.copy_from_slice(&data[pos..pos + 20]);
                pos += 20;
                sha1
            },
            flags: read_u16_be(&data, &mut pos),
            path: {
                let path_start = pos;
                while pos < data.len() && data[pos] != 0 {
                    pos += 1;
                }
                let p = String::from_utf8_lossy(&data[path_start..pos]).to_string();
                pos += 1; // 跳过 \0
                p
            },
        };

        entries.push(entry);

        // 8 字节对齐
        let entry_bytes = pos - entry_start;
        let padded = (entry_bytes + 7) & !7;
        pos = entry_start + padded;
    }

    Ok(entries)
}

/// 将暂存区写入 .git/index（大端序，8 字节对齐）
pub fn write_index(entries: &[IndexEntry]) -> io::Result<()> {
    let mut data: Vec<u8> = Vec::new();

    // Header
    data.extend_from_slice(b"DIRC");
    data.extend_from_slice(&2u32.to_be_bytes());
    data.extend_from_slice(&(entries.len() as u32).to_be_bytes());

    for entry in entries {
        let entry_start = data.len();

        data.extend_from_slice(&entry.ctime_sec.to_be_bytes());
        data.extend_from_slice(&entry.ctime_nsec.to_be_bytes());
        data.extend_from_slice(&entry.mtime_sec.to_be_bytes());
        data.extend_from_slice(&entry.mtime_nsec.to_be_bytes());
        data.extend_from_slice(&entry.dev.to_be_bytes());
        data.extend_from_slice(&entry.ino.to_be_bytes());
        data.extend_from_slice(&entry.mode.to_be_bytes());
        data.extend_from_slice(&entry.uid.to_be_bytes());
        data.extend_from_slice(&entry.gid.to_be_bytes());
        data.extend_from_slice(&entry.size.to_be_bytes());
        data.extend_from_slice(&entry.sha1);
        data.extend_from_slice(&entry.flags.to_be_bytes());
        data.extend_from_slice(entry.path.as_bytes());
        data.push(0);

        // 8 字节对齐
        let entry_bytes = data.len() - entry_start;
        let padded = (entry_bytes + 7) & !7;
        for _ in entry_bytes..padded {
            data.push(0);
        }
    }

    fs::write(".git/index", data)
}
