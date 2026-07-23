use std::process::Command;

/// The CLI must exit with a non-zero status when a command fails, so that
/// callers (shell scripts, cron, docker-compose) can detect failures.
///
/// We trigger a deterministic, offline error by importing a file that does not
/// exist. `APP_DATA_PATH`/`APP_BUILD_PATH` are pointed at a temp dir so the run
/// leaves nothing behind in the repo.
#[test]
fn exits_nonzero_when_command_fails() {
    let tmp = std::env::temp_dir().join("fossilizer-exit-code-test");

    let status = Command::new(env!("CARGO_BIN_EXE_fossilizer"))
        .env("APP_DATA_PATH", tmp.join("data"))
        .env("APP_BUILD_PATH", tmp.join("build"))
        .arg("import")
        .arg("/nonexistent/definitely-not-a-real-export.tar.gz")
        .status()
        .expect("failed to run the fossilizer binary");

    assert!(
        !status.success(),
        "expected a non-zero exit status when the command fails, got {status:?}"
    );
}
