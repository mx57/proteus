# BSDPI Rust Rewrite — Полный план работ

> **For Hermes Agent:** Implement sequentially using TDD. Record all progress in SESSION_STATUS.md.
> **Goal:** Полный рерайт BSDPI_AI на Rust — AI Core, DPI Engine Layer, Core Services, GUI
> **Repository:** `mx57/proteus`
> **Brand:** Proteus — в честь греческого бога, меняющего форму; AI постоянно эволюционирует стратегии обхода DPI
> **Architecture:** Modular Cargo workspace — каждый крейт независимый, подключается по необходимости
> **Tech Stack:** Rust stable, tokio (async), serde (serialization), rand (RNG), sha2 (hashing), sled (local DB), egui/eframe (GUI), clap (CLI)
> **Cross-Platform Target:**
>   - **Windows 10/11 x64** (первичная) — WinDivert (WinRing0), winws.exe embedded
>   - **Linux x86_64 / aarch64** — iptables/nftables, zapret compiled for Linux
>   - **Android arm64** — JNI bridge, `cargo-ndk`, embedded zapret NDK build
>   - **macOS** (future) — pf/Divert sockets
>   - Каждый DPI engine реализует `DpiEngine` trait, платформа выбирается на этапе сборки через `cfg(target_os)`

---

## Этап 0: Workspace & Infrastructure

### Task 0.1: Корневой Cargo.toml workspace
- **Файл:** `Cargo.toml` (root)
- **Содержание:** workspace с членами: bsdpi-ai, bsdpi-engine, bsdpi-core, bsdpi-android, bsdpi-gui, bsdpi-updater
- **Зависимости:** workspace-level deps

### Task 0.2: bsdpi-ai Cargo.toml
- **Файл:** `bsdpi-ai/Cargo.toml`
- **Зависимости:** serde, rand, sha2, chrono, uuid, sled

### Task 0.3: bsdpi-engine Cargo.toml
- **Файл:** `bsdpi-engine/Cargo.toml`

### Task 0.4: bsdpi-core Cargo.toml
- **Файл:** `bsdpi-core/Cargo.toml`

### Task 0.5: CLI entry (can use all crates)
- **Файл:** `bsdpi-cli/Cargo.toml`

---

## Этап 1: AI Core (bsdpi-ai)

### Task 1.1: lib.rs + WilsonScore
- **Модуль:** `bsdpi-ai/src/lib.rs` — публичный API
- **Модуль:** `bsdpi-ai/src/wilson.rs` — Wilson Score Lower Bound
- **Тест:** `bsdpi-ai/tests/wilson_test.rs`
- **Алгоритм:** Wilson Score Lower Bound (95% CI) из C# `Math/WilsonScore.cs`

### Task 1.2: NetworkFingerprint
- **Модуль:** `bsdpi-ai/src/fingerprint.rs`
- **Структуры:** `NetworkFingerprint`, `FingerprintProvider`
- **Тест:** `bsdpi-ai/tests/fingerprint_test.rs`
- **C# аналог:** `Models/NetworkFingerprint.cs`, `Services/NetworkFingerprintProvider.cs`

### Task 1.3: StrategyGenome
- **Модуль:** `bsdpi-ai/src/genome.rs`
- **Структуры:** `StrategyGenome` (50+ полей), `DpiEngineType`, `EngineProfile`
- **Тест:** `bsdpi-ai/tests/genome_test.rs`
- **C# аналог:** `Models/StrategyGenome.cs`

### Task 1.4: GenomeSignature
- **Модуль:** `bsdpi-ai/src/signature.rs`
- **Функция:** `GenomeSignature::compute(genome) -> String` (SHA256 хеш генома)
- **Тест:** `bsdpi-ai/tests/signature_test.rs`
- **C# аналог:** `Services/GenomeSignature.cs`

### Task 1.5: BanditSelector
- **Модуль:** `bsdpi-ai/src/bandit.rs`
- **Структуры:** `BanditSelector`, `Arm` / `BanditArm`
- **Алгоритмы:** Thompson Sampling (Beta distribution), UCB1, выбор лучшего
- **Тест:** `bsdpi-ai/tests/bandit_test.rs`
- **C# аналог:** `Services/BanditSelector.cs`

### Task 1.6: StrategyEvolver
- **Модуль:** `bsdpi-ai/src/evolver.rs`
- **Структуры:** `StrategyEvolver`, `EvolutionConfig`
- **Операторы:** Crossover (скрещивание 2 геномов), Mutation (15 типов), Fitness, Population management
- **Тест:** `bsdpi-ai/tests/evolver_test.rs`
- **C# аналог:** `Services/StrategyEvolver.cs`

### Task 1.7: AiStrategyRegistry
- **Модуль:** `bsdpi-ai/src/registry.rs`
- **Структуры:** `StrategyRecord`, `AiStrategyRegistry`
- **Хранилище:** sled-based local DB для персистентности
- **Тест:** `bsdpi-ai/tests/registry_test.rs`
- **C# аналог:** `Services/AiStrategyRegistry.cs`, `Services/StrategyRecord.cs`

### Task 1.8: AiHistoryStore
- **Модуль:** `bsdpi-ai/src/history.rs`
- **Структуры:** `HistoryRecord`, `AiHistoryStore`
- **Тест:** `bsdpi-ai/tests/history_test.rs`
- **C# аналог:** `Services/AiHistoryStore.cs`, `Models/WorkHistory.cs`

### Task 1.9: AiOrchestratorService
- **Модуль:** `bsdpi-ai/src/orchestrator.rs`
- **Структуры:** `OrchestratorConfig`, `AiOrchestratorService`
- **Конечный автомат:** Fingerprint → Select → Execute → Verify → Evolve
- **Тест:** `bsdpi-ai/tests/orchestrator_test.rs`
- **C# аналог:** `Services/AiOrchestratorService.cs`

---

## Этап 2: DPI Engine Layer (bsdpi-engine)

### Task 2.1: Engine Traits
- **Модуль:** `bsdpi-engine/src/traits.rs`
- **Трейты:** `DpiEngine`, `DpiEngineType`, `EngineStatus`, `EngineEvent`
- **Тест:** `bsdpi-engine/tests/traits_test.rs`

### Task 2.2: ZapretEngine
- **Модуль:** `bsdpi-engine/src/zapret.rs`
- **Реализация:** Запуск winws.exe с аргументами из EngineProfile
- **Тест:** `bsdpi-engine/tests/zapret_test.rs`

### Task 2.3: ByeDpiEngine
- **Модуль:** `bsdpi-engine/src/byedpi.rs`
- **Реализация:** Запуск ciadpi.exe с аргументами
- **Тест:** `bsdpi-engine/tests/byedpi_test.rs`

### Task 2.4: WarpEngine
- **Модуль:** `bsdpi-engine/src/warp.rs`
- **Реализация:** Запуск warp-go
- **Тест:** `bsdpi-engine/tests/warp_test.rs`

---

## Этап 3: Core Services (bsdpi-core)

### Task 3.1: ProfileProbeService — сетевое зондирование (HTTP/DNS/ICMP)
### Task 3.2: DpiEngineManager — управление жизненным циклом DPI движков
### Task 3.3: SettingsService — конфиг в TOML/YAML
### Task 3.4: ChainBuilder — конструктор цепочек режимов
### Task 3.5: SelfUpdater — автообновление через GitHub Releases

---

## Этап 4: Android JNI (bsdpi-android)

### Task 4.1: cargo-ndk настройка
### Task 4.2: JNI bridge через `jni-rs`
### Task 4.3: Android native fingerprint provider
### Task 4.4: Kotlin wrapper для KOD (`com.appomart.kod`)

---

## Этап 5: GUI (bsdpi-gui)

### Task 5.1: egui/eframe app scaffold
### Task 5.2: AI панель (Bandit/Evolver/Fingerprint)
### Task 5.3: DPI Engine панель (start/stop/status)
### Task 5.4: Chain Builder (визуальный конструктор)
### Task 5.5: Logs viewer

---

## Этап 6: Release

### Task 6.1: xtask release build
### Task 6.2: Windows single binary + embedded engines
### Task 6.3: Linux binary
### Task 6.4: Android .so / .aar

## 🎯 **Этот этап завершён. Начинайте следующий при первой возможности.**
---

## 🔄 Cross-Platform DPI Engine Architecture

### Зачем

Оригинальный BSDPI_AI — Windows-only. Rust даёт возможность **одна и та же кодовая база** работать на:
- **Windows** — winws.exe (WinDivert) + WinRing0
- **Linux** — нативный `zapret` или `nftables`/`iptables` правила
- **Android** — Встроенный DPI-движок через JNI, `/system/bin/iptables` или tun2proxy
- **macOS** — `pfctl` / Divert sockets

### Архитектура

```rust
// bsdpi-engine/src/lib.rs
mod platform;
#[cfg(target_os = "windows")]
pub use platform::windows::WinDivertEngine as DefaultEngine;
#[cfg(target_os = "linux")]
pub use platform::linux::NfTablesEngine as DefaultEngine;
#[cfg(target_os = "android")]
pub use platform::android::AndroidDpiEngine as DefaultEngine;
```

### Platform-specific build

```toml
# bsdpi-engine/Cargo.toml
[target.'cfg(windows)'.dependencies]
winapi = "0.3"
winsafe = "0.0.19"

[target.'cfg(unix)'.dependencies]
nix = "0.29"
```

### DPI движки по платформам

| Платформа | Движок | Механизм |
|-----------|--------|----------|
| Windows | Zapret (winws.exe) | WinDivert driver, userland |
| Windows | ByeDPI (ciadpi.exe) | Userland, raw sockets |
| Windows | Warp (warp-go) | WireGuard VPN |
| Linux | zapret (нативный) | Raw sockets, NFQUEUE |
| Linux | byedpi (нативный) | Raw sockets |
| Android | встроенный | JNI + tun2proxy / iptables |
| Все | Warp (warp-go) | WireGuard (везде, где есть Go) |

### Абстракция для AI слоя

AI слой (bsdpi-ai) **не знает о платформе** — он работает с `DpiEngineType` enum и `EngineProfile` структурами. Выбор конкретной платформенной реализации происходит в `bsdpi-engine` при запуске.

```rust
// AI слой pure Rust — без платформенного кода
enum DpiEngineType { Zapret, ByeDpi, Warp, Hybrid, Chained, None }
struct EngineProfile { /* platform-independent args */ }

// Платформенная реализация в bsdpi-engine
#[cfg(target_os = "windows")]
impl DpiEngine for ZapretEngine { /* winws.exe */ }
#[cfg(target_os = "linux")]
impl DpiEngine for ZapretEngine { /* nftables/iptables */ }
```

---

## 🔄 Как продолжать

1. Открыть `SESSION_STATUS.md` — видно что сделано
2. Открыть `PLAN.md` — видно что делать дальше
3. `cargo test --workspace` — проверить что всё зелёное
4. Выбрать следующий незавершённый Task
5. Написать тест → реализовать → проверить → записать в SESSION_STATUS
