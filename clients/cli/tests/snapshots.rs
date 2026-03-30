/// Snapshot tests for CLI output using `insta`.
///
/// Run `cargo test` to execute. On first run (or after intentional changes)
/// use `cargo insta review` to accept new/updated snapshots.
///
/// In CI, set `INSTA_UPDATE=no` to fail on any snapshot mismatch.
use std::process::Command;

fn run_cli(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_star-escrow"))
        .args(args)
        .output()
        .expect("failed to run star-escrow binary");

    // clap writes --help to stdout; combine both streams to be safe.
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    combined
}

#[test]
fn snapshot_help_output() {
    let output = run_cli(&["--help"]);
    insta::assert_snapshot!("help", output);
}

#[test]
fn snapshot_status_help_output() {
    let output = run_cli(&["status", "--help"]);
    insta::assert_snapshot!("status_help", output);
}

#[test]
fn snapshot_create_help_output() {
    let output = run_cli(&["create", "--help"]);
    insta::assert_snapshot!("create_help", output);
}
