<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/mx57/proteus/main/assets/logo-white.svg">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/mx57/proteus/main/assets/logo-dark.svg">
  <img width="500" alt="Proteus Logo" src="https://raw.githubusercontent.com/mx57/proteus/main/assets/logo-dark.svg">
</picture>

<br/>

**Провайдер блокирует? Proteus находит способ.**

Самообучающаяся система обхода DPI, которая **сама подбирает и эволюционирует** рабочие стратегии под вашу сеть.

[![Rust](https://img.shields.io/badge/Rust-1.97+-DEA584?logo=rust&style=flat-square)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-GPLv3-blue.svg?style=flat-square)](./LICENSE)
[![Tests](https://img.shields.io/github/actions/workflow/status/mx57/proteus/ci.yml?label=tests&style=flat-square)](./.github/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/mx57/proteus?style=flat-square)](https://github.com/mx57/proteus/releases)

---

</div>

## 🧠 Как это работает

Proteus использует **три AI-алгоритма** для автоматического подбора параметров обхода DPI:

| Алгоритм | Что делает |
|----------|------------|
| **🎯 Thompson Sampling** | Анализирует успешность каждой стратегии и балансирует между exploitation (лучшая) и exploration (новая) |
| **🧬 Генетическая эволюция** | Скрещивает параметры лучших стратегий, мутирует их — «выращивает» новые, более эффективные |
| **📊 Wilson Score** | Статистически строгая оценка качества каждой стратегии (95% доверительный интервал) |
| **🌐 Network Fingerprinting** | Запоминает политику для каждой сети отдельно (Wi-Fi, мобильный интернет) |

## 🔧 Возможности

- **🔄 Thompson Sampling** — многорукие бандиты с Beta-распределением
- **🧬 Генетическая эволюция** — 15 типов мутаций, кроссинговер, элитизм, GC слабейших
- **🌐 Network Fingerprinting** — SHA256 слепок сети, отдельная политика на каждую сеть
- **⚡ Fast Start** — мгновенная проверка лучших стратегий при запуске
- **📊 Wilson Score** — ранжирование стратегий по нижней границе Уилсона
- **🌍 Multi-platform** — Windows, Linux
- **🖥️ Desktop GUI** — egui/eframe кроссплатформенный интерфейс
- **⌨️ CLI** — управление через командную строку

## 🚀 Быстрый старт

```bash
# Сборка
cargo build --release

# CLI
cargo run --bin proteus -- status
cargo run --bin proteus -- bandit
cargo run --bin proteus -- probe

# GUI (требуется дисплей)
cargo run -p bsdpi-gui

# Тесты
cargo test --workspace
```

## 🏗️ Архитектура

```
bsdpi-rs/
├── bsdpi-ai/        # AI Core (pure Rust)
│   ├── wilson.rs    # Wilson Score Lower Bound
│   ├── bandit.rs    # Thompson Sampling + UCB1
│   ├── evolver.rs   # Генетическая эволюция
│   ├── genome.rs    # Геном стратегии (50+ параметров)
│   ├── fingerprint  # Network Fingerprint
│   └── orchestrator # AI-оркестратор
├── bsdpi-engine/    # DPI Engine Layer (Zapret, ByeDPI, Warp)
├── bsdpi-core/      # Core Services (probe, settings, chains)
├── bsdpi-gui/       # Desktop GUI (egui/eframe)
├── bsdpi-cli/       # CLI binary (proteus)
└── bsdpi-updater/   # Self-update
```

## 📦 Статус проекта

| Компонент | Статус | Тесты |
|-----------|--------|-------|
| AI Core (bandit, evolver, fingerprint, ...) | ✅ Done | 97 |
| DPI Engine (Zapret, ByeDPI, Warp) | ✅ Done | 18 |
| Core Services (probe, manager, settings) | ✅ Done | 30 |
| Desktop GUI | 🟡 Stub | — |
| CLI | ✅ Done | 2 |
| Updater | 🟡 Stub | — |

## 📄 Лицензия

GPLv3 — весь код открыт. © 2026 mx57

---

<div align="center">
<b>Proteus</b> — меняй форму, обходи блокировки. 🎭
</div>
