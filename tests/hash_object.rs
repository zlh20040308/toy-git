use std::fs;
use std::process::Command;

use tempfile::TempDir;

/// 获取 toy-git 可执行文件路径（cargo test 自动注入）
fn toy_git() -> &'static str {
    env!("CARGO_BIN_EXE_toy-git")
}

#[test]
fn hash_object_creates_correct_object_file() {
    // 1. 创建一个临时仓库目录
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    // 2. 先 init
    let status = Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .expect("failed to run toy-git init");

    assert!(status.success());

    // 3. 创建一个测试文件
    let file_path = repo.join("hello.txt");
    fs::write(&file_path, b"hello\n").unwrap();

    // 4. 调用 hash-object
    let output = Command::new(toy_git())
        .current_dir(repo)
        .args(["hash-object", "-w", file_path.to_str().unwrap()])
        .output()
        .expect("failed to run toy-git hash-object");

    assert!(output.status.success());

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // git 官方结果：echo "hello" | git hash-object --stdin
    assert_eq!(hash, "ce013625030ba8dba906f756967f9e9ca394464a");

    // 5. 检查 object 文件是否存在
    let obj_dir = repo.join(".git").join("objects").join(&hash[0..2]);
    let obj_file = obj_dir.join(&hash[2..]);

    assert!(
        obj_file.exists(),
        "object file {:?} does not exist",
        obj_file
    );
}

#[test]
fn hash_object_same_content_same_hash() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();

    let a = repo.join("a.txt");
    let b = repo.join("b.txt");

    fs::write(&a, b"same content\n").unwrap();
    fs::write(&b, b"same content\n").unwrap();

    let hash_a = Command::new(toy_git())
        .current_dir(repo)
        .args(["hash-object", a.to_str().unwrap()])
        .output()
        .unwrap();

    let hash_b = Command::new(toy_git())
        .current_dir(repo)
        .args(["hash-object", b.to_str().unwrap()])
        .output()
        .unwrap();

    let ha = String::from_utf8_lossy(&hash_a.stdout).trim().to_string();
    let hb = String::from_utf8_lossy(&hash_b.stdout).trim().to_string();

    assert_eq!(ha, hb);
}

#[test]
fn hash_object_missing_file_should_fail() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();

    let output = Command::new(toy_git())
        .current_dir(repo)
        .args(["hash-object", "no_such_file"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "hash-object should fail on missing file"
    );
}
