use std::fs;
use std::process::Command;

use tempfile::TempDir;

/// 获取 toy-git 可执行文件路径
fn toy_git() -> &'static str {
    env!("CARGO_BIN_EXE_toy-git")
}

#[test]
fn cat_file_prints_blob_content() {
    // 1. 创建临时仓库
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    // 2. init
    let status = Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .expect("failed to run toy-git init");

    assert!(status.success());

    // 3. 创建测试文件
    let file_path = repo.join("hello.txt");
    let content = b"hello from cat-file\n";
    fs::write(&file_path, content).unwrap();

    // 4. hash-object -w
    let output = Command::new(toy_git())
        .current_dir(repo)
        .args([
            "hash-object",
            "-w",
            file_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run toy-git hash-object");

    assert!(
        output.status.success(),
        "hash-object failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let hash = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    // 5. cat-file -p <hash>
    let output = Command::new(toy_git())
        .current_dir(repo)
        .args(["cat-file", "-p", &hash])
        .output()
        .expect("failed to run toy-git cat-file");

    assert!(
        output.status.success(),
        "cat-file failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // 6. stdout 应该等于原始内容
    assert_eq!(output.stdout, content);
}

#[test]
fn cat_file_invalid_hash_should_fail() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();

    let output = Command::new(toy_git())
        .current_dir(repo)
        .args([
            "cat-file",
            "-p",
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "cat-file should fail on invalid hash"
    );
}
