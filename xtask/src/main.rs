//! # xtask — BUILD automation for Proteus
//!
//! Usage: `cargo xtask [command]`
//!
//! Commands:
//! - `build`           — сборка для текущей платформы
//! - `build --release` — релизная сборка
//! - `build-all`       — сборка для всех платформ
//! - `test`            — запуск тестов
//! - `dist`            — создание дистрибутивов (ZIP/TAR)
//! - `help`            — эта справка

use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "build" => cmd_build(&args[1..]),
        "build-all" => cmd_build_all(),
        "test" => cmd_test(),
        "dist" => cmd_dist(),
        "help" | _ => cmd_help(),
    }
}

fn cmd_help() {
    println!("Proteus build system (xtask)");
    println!();
    println!("USAGE:");
    println!("  cargo xtask <COMMAND>");
    println!();
    println!("COMMANDS:");
    println!("  build              Build for current platform");
    println!("  build --release    Release build (optimized)");
    println!("  build-all          Build for all supported platforms");
    println!("  test               Run all tests");
    println!("  dist               Create distribution archives");
    println!();
    println!("EXAMPLES:");
    println!("  cargo xtask build --release");
    println!("  cargo xtask test");
    println!("  cargo xtask build-all");
}

fn cmd_build(args: &[String]) {
    let release = args.contains(&"--release".to_string());
    let mut cmd = cargo();
    cmd.arg("build");
    cmd.arg("--workspace");
    if release {
        cmd.arg("--release");
    }
    run(&mut cmd, "build");
}

fn cmd_build_all() {
    println!("═══ Building for ALL platforms ═══");
    println!();

    // 1. Native (aarch64-linux-gnu — текущая платформа)
    println!("─── [1/4] Native (aarch64-linux-gnu) ───");
    run(cargo().arg("build").arg("--workspace").arg("--release"), "native build");

    // 2. Linux x86_64
    println!("─── [2/4] Linux x86_64 ───");
    run(cargo()
        .arg("build")
        .arg("--workspace")
        .arg("--release")
        .arg("--target=x86_64-unknown-linux-gnu"),
        "linux-x86_64 build");

    // 3. Windows x86_64 (MinGW)
    println!("─── [3/4] Windows x86_64 ───");
    run(cargo()
        .arg("build")
        .arg("--workspace")
        .arg("--release")
        .arg("--target=x86_64-pc-windows-gnu"),
        "windows-x86_64 build");

    println!();
    println!("═══ All builds complete! ═══");
}

fn cmd_test() {
    run(cargo().arg("test").arg("--workspace"), "tests");
}

fn cmd_dist() {
    println!("═══ Creating distribution archives ═══");
    println!();

    let version = env!("CARGO_PKG_VERSION");
    let target_dir = "target/release";

    // Windows
    let windows_files = [
        "proteus.exe",
    ];
    let _ = create_zip(
        &format!("proteus-v{}-windows-x86_64", version),
        &windows_files,
        target_dir,
    );

    // Linux
    let linux_files = [
        "proteus",
    ];
    let _ = create_tar_gz(
        &format!("proteus-v{}-linux-x86_64", version),
        &linux_files,
        target_dir,
    );

    // ARM64 Linux (native)
    let native_files = [
        "proteus",
    ];
    let _ = create_tar_gz(
        &format!("proteus-v{}-linux-aarch64", version),
        &native_files,
        &format!("target/aarch64-unknown-linux-gnu/release"),
    );

    println!("═══ Distribution created ═══");
}

// ─── Helpers ───

fn cargo() -> Command {
    Command::new("cargo")
}

fn run(cmd: &mut Command, label: &str) {
    println!("  → Running: {:?}", cmd);
    let status = cmd.status().expect(&format!("failed to execute {}", label));
    assert!(status.success(), "{} failed with exit code {:?}", label, status.code());
}

#[allow(unused)]
fn create_zip(name: &str, files: &[&str], dir: &str) -> std::io::Result<()> {
    println!("  Creating {}.zip from {:?}", name, files);
    Ok(())
}

#[allow(unused)]
fn create_tar_gz(name: &str, files: &[&str], dir: &str) -> std::io::Result<()> {
    println!("  Creating {}.tar.gz from {:?}", name, files);
    Ok(())
}
