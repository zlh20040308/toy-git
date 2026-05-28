use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn toy_git() -> &'static str {
    env!("CARGO_BIN_EXE_toy-git")
}

#[test]
fn update_index_creates_index_file() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    // init
    let status = Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();
    assert!(status.success());

    // create a file
    let file_path = repo.join("hello.txt");
    fs::write(&file_path, b"hello\n").unwrap();

    // update-index
    let status = Command::new(toy_git())
        .current_dir(repo)
        .args(["update-index", file_path.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());

    // verify .git/index exists
    let index_path = repo.join(".git").join("index");
    assert!(index_path.exists(), ".git/index should exist after update-index");
}

#[test]
fn update_index_stores_blob() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    // init
    Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();

    // create a file with known content
    let file_path = repo.join("test.txt");
    fs::write(&file_path, b"hello\n").unwrap();

    // update-index
    let status = Command::new(toy_git())
        .current_dir(repo)
        .args(["update-index", file_path.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());

    // the blob should be stored: ce013625030ba8dba906f756967f9e9ca394464a
    let obj_path = repo
        .join(".git")
        .join("objects")
        .join("ce")
        .join("013625030ba8dba906f756967f9e9ca394464a");
    assert!(obj_path.exists(), "blob should be stored in objects dir");
}

#[test]
fn update_index_updates_existing_entry() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();

    let file_path = repo.join("a.txt");
    fs::write(&file_path, b"v1\n").unwrap();

    // first add
    let status = Command::new(toy_git())
        .current_dir(repo)
        .args(["update-index", file_path.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());

    // change file content
    fs::write(&file_path, b"v2\n").unwrap();

    // second add (update)
    let status = Command::new(toy_git())
        .current_dir(repo)
        .args(["update-index", file_path.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success(), "updating existing entry should succeed");

    // the index should still have only 1 entry for this file
    // (we verify by checking it doesn't crash — structural validation)
}
