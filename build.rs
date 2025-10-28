use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=.git/refs/heads/main");

    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();

    let mut git_hash = String::from_utf8(output.stdout).unwrap();

    let status_output = Command::new("git")
        .args(&["status", "--porcelain"])
        .output()
        .unwrap();

    if !status_output.stdout.is_empty() {
        git_hash = format!("{} (dirty)", git_hash.trim());
    }
    println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
}
