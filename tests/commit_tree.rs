use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn toy_git() -> &'static str {
    env!("CARGO_BIN_EXE_toy-git")
}

/// 辅助函数：setup 一个仓库，add 一个文件，返回 (repo路径, tree_hash)
fn setup_with_tree() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let repo = dir.path().to_path_buf();

    Command::new(toy_git())
        .current_dir(&repo)
        .arg("init")
        .status()
        .unwrap();

    let file = repo.join("hello.txt");
    fs::write(&file, b"hello\n").unwrap();
    Command::new(toy_git())
        .current_dir(&repo)
        .args(["update-index", file.to_str().unwrap()])
        .status()
        .unwrap();

    let output = Command::new(toy_git())
        .current_dir(&repo)
        .args(["write-tree"])
        .output()
        .unwrap();

    let tree_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    (dir, tree_hash)
}

#[test]
fn commit_tree_creates_root_commit() {
    let (dir, tree_hash) = setup_with_tree();

    let output = Command::new(toy_git())
        .current_dir(dir.path())
        .args(["commit-tree", "-t", &tree_hash, "-m", "initial"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "commit-tree failed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("root-commit"));

    // commit hash 应该存在
    let commit_hash: String = stdout
        .split_whitespace()
        .last()
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(commit_hash.len(), 40);

    let obj_path = dir
        .path()
        .join(".git")
        .join("objects")
        .join(&commit_hash[0..2])
        .join(&commit_hash[2..]);
    assert!(obj_path.exists(), "commit object should exist");

    // 注意：commit-tree 不更新分支引用！
    let ref_path = dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("main");
    assert!(
        !ref_path.exists(),
        "commit-tree should NOT update branch ref"
    );
}

#[test]
fn commit_tree_with_parent() {
    let (dir, tree_hash) = setup_with_tree();

    // 先创建一个 root commit
    let out1 = Command::new(toy_git())
        .current_dir(dir.path())
        .args([
            "commit-tree",
            "-t", &tree_hash,
            "-m", "first",
        ])
        .output()
        .unwrap();
    let parent_hash: String = String::from_utf8_lossy(&out1.stdout)
        .split_whitespace()
        .last()
        .unwrap()
        .trim()
        .to_string();

    // 用同一个 tree，但指定了 parent
    let out2 = Command::new(toy_git())
        .current_dir(dir.path())
        .args([
            "commit-tree",
            "-t", &tree_hash,
            "-p", &parent_hash,
            "-m", "second",
        ])
        .output()
        .unwrap();
    assert!(out2.status.success());

    let stdout = String::from_utf8_lossy(&out2.stdout);
    // 有 parent 的提交不应显示 root-commit
    assert!(!stdout.contains("root-commit"));
    assert!(stdout.contains("[commit]"));
}
