use std::process::Command;

use tempfile::TempDir;

/// 获取 toy-git 可执行文件路径
fn toy_git() -> &'static str {
    env!("CARGO_BIN_EXE_toy-git")
}

#[test]
fn init_creates_git_dir() {
    // 1. 创建临时目录
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    // 2. 执行 init
    let output = Command::new(toy_git())
        .current_dir(repo)
        .arg("init")
        .output()
        .expect("failed to run toy-git init");

    // 3. 确认命令成功退出
    assert!(
        output.status.success(),
        "init failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // 4. 检查 .git 目录是否存在
    let git_dir = repo.join(".git");
    assert!(
        git_dir.exists() && git_dir.is_dir(),
        ".git directory was not created"
    );

    // 5. 可选：检查一些默认子目录
    for subdir in &["objects", "refs"] {
        let path = git_dir.join(subdir);
        assert!(
            path.exists() && path.is_dir(),
            "expected {} to exist inside .git", subdir
        );
    }
}
