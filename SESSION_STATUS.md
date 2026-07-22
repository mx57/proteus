# Proteus — AI DPI Bypass (Rust)

> **Начало:** 2026-07-21
> **Репозиторий:** `mx57/proteus` (будет создан)
> **Прежнее название:** BSDPI_AI (C#)
> **Цель:** Полный рерайт BSDPI_AI на Rust — AI Core, DPI Engines, Core Services, GUI
> **Лицензия:** GPLv3

---

## 📊 Прогресс

| # | Компонент | Статус | Тесты | Примечание |
|---|-----------|--------|-------|------------|
| 0 | Workspace + Cargo.toml + структура | ✅ DONE | — | |
| 1 | **AI Core** (bsdpi-ai) | ✅ DONE | ✅ 97 passed | Весь AI слой завершён |
| 1.1 | lib.rs — публичный API модулей | ✅ DONE | — | |
| 1.2 | WilsonScore — Wilson Lower Bound | ✅ DONE | ✅ 8 | Все диапазоны, z-тесты |
| 1.3 | NetworkFingerprint — слепок сети | ✅ DONE | ✅ 5 | Хеш, сравнение, Display |
| 1.4 | StrategyGenome — геном (50+ полей) | ✅ DONE | ✅ 4 | EngineProfile, CLI args, Display |
| 1.5 | GenomeSignature — SHA256 сигнатура | ✅ DONE | ✅ 5 | Детерминизм, уникальность, ExtraArgs |
| 1.6 | BanditSelector — Thompson Sampling + UCB1 | ✅ DONE | ✅ 19 | Thompson, UCB1, Pareto, Backoff, Normal/Gamma/Beta samplers |
| 1.7 | StrategyEvolver — генетическая эволюция | ✅ DONE | ✅ 16 | Crossover, Mutation (15 типов), GC, Delta |
| 1.8 | AiStrategyRegistry — хранилище стратегий | ✅ DONE | ✅ 14 | JSON-persistence, bandit CRUD, lookup dicts, импорт/экспорт |
| 1.9 | AiHistoryStore — лог истории | ✅ DONE | ✅ 12 | Append-only JSON-Lines, cache, ротация, фильтрация |
| 1.10 | AiOrchestratorService — конечный автомат | ✅ DONE | ✅ 14 | 6 состояний, lifecycle, error handling, события, stats |
| 2 | **DPI Engine** (bsdpi-engine) | ✅ DONE | ✅ 18 passed | DpiEngine trait, Zapret, ByeDpi, Warp, CLI args |
| 3 | **Core Services** (bsdpi-core) | ✅ DONE | ✅ 30 passed | ProbeService, EngineManager, Settings, Chains, Updater |
| 4 | **Android JNI** (bsdpi-android) | 🟡 Stub | — | Требуется NDK |
| 5 | **GUI** (bsdpi-gui) | ✅ DONE | — | 6 табов: Main, AI, Engine, Chains, Settings, Logs |
| 6 | **CLI** (bsdpi-cli) | ✅ DONE | ✅ 2 | 8 команд: start, stop, probe, evolve, bandit, config, update, status |
| 7 | Updater (bsdpi-core / bsdpi-cli) | ✅ DONE | — | Логика обновлений GitHub реализована в bsdpi-core/SelfUpdater и интегрирована в CLI |

---

## 🧱 Архитектура

```
bsdpi-rs/
├── Cargo.toml               # workspace root
├── SESSION_STATUS.md        # этот файл — прогресс
├── PLAN.md                  # полный план работ
├── bsdpi-ai/                # AI Core (pure Rust, платформонезависим)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs           # публичный re-export API
│       ├── wilson.rs        # Wilson Score Lower Bound
│       ├── fingerprint.rs   # NetworkFingerprint + FingerprintProvider trait
│       ├── genome.rs        # StrategyGenome + EngineProfile + DpiEngineType
│       ├── signature.rs     # GenomeSignature (SHA256)
│       ├── bandit.rs        # BanditSelector (Thompson Sampling + UCB1)
│       ├── evolver.rs       # StrategyEvolver (GA)
│       ├── registry.rs      # AiStrategyRegistry (persistent storage)
│       ├── history.rs       # AiHistoryStore (event log)
│       └── orchestrator.rs  # AiOrchestratorService (state machine)
├── bsdpi-engine/            # DPI Engine (WinDivert, Zapret, ByeDPI, Warp)
├── bsdpi-core/              # Core services (probing, settings, updater)
├── bsdpi-android/           # Android JNI bridge (для Android сборки)
├── bsdpi-gui/               # egui/eframe GUI
├── bsdpi-updater/           # Self-update
└── engine/                  # embedded DPI binaries
```

---

## 📝 Лог сессий

### Session 1 — 2026-07-21

**Сделано:**
- [x] Установлен Rust (1.97.1, aarch64-unknown-linux-gnu)
- [x] Установлен build-essential (gcc, make)
- [x] Создан workspace `bsdpi-rs/` со всеми каталогами крейтов
- [x] Создан `SESSION_STATUS.md` — лог прогресса
- [x] Создан `PLAN.md` — детальный план работ
- [x] Создан корневой `Cargo.toml` workspace (6 крейтов)
- [x] Создан `bsdpi-ai/Cargo.toml` с зависимостями
- [x] Реализован **WilsonScore**: формула Lower Bound, z-score support, тесты (8)
- [x] Реализован **NetworkFingerprint**: структура, SHA256 хеш, Display, FingerprintProvider trait + заглушка, тесты (5)
- [x] Реализован **StrategyGenome**: 50+ полей, DpiEngineType, StrategyOrigin, EngineProfile, to_cli_args(), to_engine_profile(), тесты (4)
- [x] Реализован **GenomeSignature**: детерминированный SHA256 от всех полей, дедупликация, тесты (5)
- [x] Созданы stub'ы: BanditSelector, StrategyEvolver, Registry, History, Orchestrator — структуры + базовые тесты (12)
- [x] `cargo build` — успешная компиляция
- [x] `cargo test` — **36 тестов, все проходят** ✅

**Статистика:**
- Файлов: 11 Rust source + 2 Cargo.toml + 2 .md = 15 файлов
- Строк кода: ~4500 (AI Core)
- Тестов: 36, все зелёные

**В плане на след. сессию:**
- [ ] BanditSelector: Thompson Sampling выбор (Beta distribution), UCB1, выбор лучшей руки
- [ ] StrategyEvolver: Crossover (2 генома → потомок), Mutation (15 типов), Fitness, Population management
- [ ] AiStrategyRegistry: sled-based persistency, CRUD операций
- [ ] AiHistoryStore: append-only лог с запросами по времени
- [ ] AiOrchestratorService: полный конечный автомат (Fingerprint→Select→Execute→Verify→Evolve)

---

## ⚙️ Команды сборки

```bash
# Полная сборка workspace
cargo build --workspace

# Сборка с тестами
cargo test --workspace

# Сборка AI core отдельно
cargo build -p bsdpi-ai

# Тесты AI core
cargo test -p bsdpi-ai -- --nocapture

# Проверка без компиляции
cargo check --workspace
```
