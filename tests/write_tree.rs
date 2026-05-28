use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn toy_git() -> &'static str {
    env!("CARGO_BIN_EXE_toy-git")
}

#[test]
fn write_tree_prints_hash_and_stores_object() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    // init
    Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();

    // create and add a file
    let file = repo.join("hello.txt");
    fs::write(&file, b"hello\n").unwrap();
    let status = Command::new(toy_git())
        .current_dir(repo)
        .args(["update-index", file.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());

    // write-tree
    let output = Command::new(toy_git())
        .current_dir(repo)
        .args(["write-tree"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "write-tree failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let tree_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(tree_hash.len(), 40, "hash should be 40 hex chars");

    // tree object should exist in .git/objects
    let obj_path = repo
        .join(".git")
        .join("objects")
        .join(&tree_hash[0..2])
        .join(&tree_hash[2..]);
    assert!(obj_path.exists(), "tree object file should exist");
}

#[test]
fn write_tree_with_empty_index_fails() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();

    // write-tree with no files added should fail
    let output = Command::new(toy_git())
        .current_dir(repo)
        .args(["write-tree"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "write-tree should fail on empty index"
    );
}

#[test]
fn write_tree_multiple_files() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap();

    // add two files
    for name in &["a.txt", "b.txt"] {
        let file = repo.join(name);
        fs::write(&file, format!("content of {}\n", name)).unwrap();
        let status = Command::new(toy_git())
            .current_dir(repo)
            .args(["update-index", file.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());
    }

    // write-tree
    let output = Command::new(toy_git())
        .current_dir(repo)
        .args(["write-tree"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let tree_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(tree_hash.len(), 40);

    // verify tree object exists
    let obj_path = repo
        .join(".git")
        .join("objects")
        .join(&tree_hash[0..2])
        .join(&tree_hash[2..]);
    assert!(obj_path.exists());
}
