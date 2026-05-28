use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn toy_git() -> &'static str {
    env!("CARGO_BIN_EXE_toy-git")
}

#[test]
fn commit_creates_commit_object() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    // init
    Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();

    // add a file
    let file = repo.join("hello.txt");
    fs::write(&file, b"hello\n").unwrap();
    let status = Command::new(toy_git())
        .current_dir(repo)
        .args(["update-index", file.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());

    // commit
    let output = Command::new(toy_git())
        .current_dir(repo)
        .args(["commit", "-m", "first commit"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "commit failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("(root-commit)"), "first commit should be root-commit");

    // extract commit hash from output like "[main (root-commit)] abc123..."
    let commit_hash: String = stdout
        .split_whitespace()
        .last()
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(commit_hash.len(), 40, "commit hash should be 40 hex chars");

    // commit object should exist
    let obj_path = repo
        .join(".git")
        .join("objects")
        .join(&commit_hash[0..2])
        .join(&commit_hash[2..]);
    assert!(obj_path.exists(), "commit object file should exist");

    // branch ref should point to this commit
    let ref_path = repo.join(".git").join("refs").join("heads").join("main");
    let ref_content = fs::read_to_string(&ref_path).unwrap();
    assert_eq!(ref_content.trim(), commit_hash);
}

#[test]
fn second_commit_has_parent() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();

    // first commit
    let file = repo.join("a.txt");
    fs::write(&file, b"v1\n").unwrap();
    Command::new(toy_git())
        .current_dir(repo)
        .args(["update-index", file.to_str().unwrap()])
        .status()
        .unwrap();
    let out1 = Command::new(toy_git())
        .current_dir(repo)
        .args(["commit", "-m", "first"])
        .output()
        .unwrap();
    assert!(out1.status.success());

    // second commit (different file)
    let file2 = repo.join("b.txt");
    fs::write(&file2, b"v2\n").unwrap();
    Command::new(toy_git())
        .current_dir(repo)
        .args(["update-index", file2.to_str().unwrap()])
        .status()
        .unwrap();
    let out2 = Command::new(toy_git())
        .current_dir(repo)
        .args(["commit", "-m", "second"])
        .output()
        .unwrap();
    assert!(out2.status.success());

    // second commit should NOT say root-commit
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert!(
        !stdout2.contains("root-commit"),
        "second commit should not be root-commit"
    );

    // branch ref should point to the second commit
    let ref_path = repo.join(".git").join("refs").join("heads").join("main");
    let ref_content = fs::read_to_string(&ref_path).unwrap();
    let second_hash: String = stdout2
        .split_whitespace()
        .last()
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(ref_content.trim(), second_hash);
}

#[test]
fn commit_empty_index_fails() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();

    let output = Command::new(toy_git())
        .current_dir(repo)
        .args(["commit", "-m", "nothing"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "commit should fail on empty index"
    );
}
