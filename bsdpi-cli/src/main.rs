//! # Proteus CLI
//!
//! Командная строка для управления Proteus DPI bypass.
//!
//! ## Подкоманды
//! - `status` — статус системы
//! - `fingerprint` — показать fingerprint сети
//! - `bandit` — показать состояние bandit
//! - `evolve` — запустить эволюцию
//! - `probe` — проверить соединение

use std::path::PathBuf;
use clap::{Parser, Subcommand};
use bsdpi_ai::FingerprintProvider;

#[derive(Parser)]
#[command(name = "proteus", version, about = "Proteus — самообучающаяся система обхода DPI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Путь к конфигурации
    #[arg(short, long, default_value = "~/.config/proteus/config.toml")]
    config: PathBuf,

    /// Путь к данным (база стратегий и история)
    #[arg(short, long, default_value = "~/.local/share/proteus")]
    data_dir: PathBuf,

    /// Включить подробный вывод
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Показать статус системы
    Status,
    /// Показать fingerprint текущей сети
    Fingerprint,
    /// Показать состояние bandit (стратегии и их оценки)
    Bandit,
    /// Запустить эволюцию стратегий
    Evolve {
        /// Количество поколений
        #[arg(short, long, default_value = "1")]
        generations: u32,
    },
    /// Проверить соединение
    Probe {
        /// URL для проверки (по умолчанию rutracker.org)
        #[arg(default_value = "https://rutracker.org")]
        url: String,
    },
}

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    }

    match &cli.command {
        Commands::Status => cmd_status(),
        Commands::Fingerprint => cmd_fingerprint(),
        Commands::Bandit => cmd_bandit(),
        Commands::Evolve { generations } => cmd_evolve(*generations),
        Commands::Probe { url } => cmd_probe(url),
    }
}

fn cmd_status() {
    println!("🔍 Proteus DPI Bypass System");
    println!();
    println!("AI Core: ✅ ready");
    println!("Bandit:  no data (run `proteus probe` to collect)");
    println!("Network: not fingerprinted");
}

fn cmd_fingerprint() {
    println!("🌐 Network Fingerprint");
    println!();
    match bsdpi_ai::FingerprintProviderImpl::new().current_fingerprint() {
        Ok(fp) => {
            println!("  Label:      {}", fp.label);
            println!("  Transport:  {}", fp.transport);
            println!("  Gateway:    {}", fp.gateway_ip);
            println!("  DNS:        {}", fp.dns_servers.join(", "));
            println!("  Subnet:     {}", fp.local_subnet);
            println!("  Hash:       {}", fp.hash);
        }
        Err(e) => println!("  ❌ Error: {}", e),
    }
}

fn cmd_bandit() {
    println!("🎯 Bandit Status");
    println!();
    println!("  No data loaded. Use `proteus probe` to collect data first.");
}

fn cmd_evolve(_generations: u32) {
    println!("🧬 Evolution");
    println!();
    println!("  Status: no strategies in pool");
}

fn cmd_probe(url: &str) {
    println!("📡 Probing: {}", url);
    println!();

    match reqwest::blocking::get(url) {
        Ok(resp) => {
            let status = resp.status();
            let size = resp.content_length().unwrap_or(0);
            println!("  ✅ URL доступен");
            println!("  Status: {}", status.as_u16());
            println!("  Size:   {} bytes", size);
        }
        Err(e) => {
            println!("  ❌ URL недоступен — DPI блокировка?");
            println!("  Error: {}", e);
            println!();
            println!("  Попробуйте запустить Proteus и повторить.");
        }
    }
}
