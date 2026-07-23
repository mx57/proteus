# BSDPI Rust Rewrite — SESSION STATUS

> **Начало:** 2026-07-21
> **Репозиторий:** /root/workspace/bsdpi-rs/ (локально)
> **Цель:** Полный рерайт BSDPI_AI на Rust — AI Core, DPI Engines, Core Services
> **Лицензия:** GPLv3

---

## 📊 Прогресс (реальный)

| # | Компонент | Статус | Тесты |
|---|-----------|--------|-------|
| **0** | **Workspace** | ✅ DONE | — |
| **1** | **AI Core (bsdpi-ai)** | 🟢 **9/10 модулей DONE** | **74/74** |
| 1.1 | wilson.rs — Wilson Score | ✅ DONE | 11 |
| 1.2 | fingerprint.rs — NetworkFingerprint | ✅ DONE | 5 |
| 1.3 | genome.rs — StrategyGenome (50+ params) | ✅ DONE | 7 |
| 1.4 | signature.rs — GenomeSignature (SHA256) | ✅ DONE | 6 |
| 1.5 | bandit.rs — BanditSelector (Thompson + UCB1 + Pareto) | ✅ DONE | 11 |
| 1.6 | evolver.rs — StrategyEvolver (GA: crossover, 15 mutations, GC) | ✅ DONE | 10 |
| 1.7 | registry.rs — AiStrategyRegistry (JSON persistence) | ✅ DONE | 10 |
| 1.8 | history.rs — AiHistoryStore (JSONL append-only log) | ✅ DONE | 8 |
| 1.9 | orchestrator.rs — AiOrchestratorService | ✅ DONE | 6 |
| **2** | **DPI Engine (bsdpi-engine)** | ⬜ | — |
| **3** | **Core Services (bsdpi-core)** | ⬜ | — |
| **4** | **GUI (bsdpi-gui)** | ⬜ | — |

---

## 🧱 Структура

```
/root/workspace/bsdpi-rs/
├── Cargo.toml                    # Workspace root (bsdpi-ai, bsdpi-engine, bsdpi-core, bsdpi-gui)
├── SESSION_STATUS.md             # Этот файл
├── PLAN.md                       # Детальный план
├── bsdpi-ai/src/
│   ├── lib.rs                    # Публичный API
│   ├── error.rs                  # AiError
│   ├── wilson.rs                 # Wilson Score Lower Bound
│   ├── fingerprint.rs            # NetworkFingerprint + FingerprintProvider trait
│   ├── genome.rs                 # StrategyGenome (50+ полей)
│   ├── signature.rs              # GenomeSignature (SHA256)
│   ├── bandit.rs                 # BanditSelector (Thompson Sampling + UCB1 + Pareto)
│   ├── evolver.rs                # StrategyEvolver (GA: crossover, mutation, GC)
│   ├── registry.rs               # TODO: AiStrategyRegistry
│   ├── history.rs                # TODO: AiHistoryStore
│   └── orchestrator.rs           # AiOrchestratorService
├── bsdpi-engine/                 # TODO
├── bsdpi-core/                   # TODO
└── bsdpi-gui/                    # TODO
```

---

## ✅ Что готово (AI Core)

Все 6 модулей AI Core скомпилированы и протестированы:

### wilson.rs
- `lower_bound(successes, trials, z) -> f64` — Wilson Score Lower Bound
- `mean_score(scores) -> f64` — среднее арифметическое
- Константы: `Z_95`, `Z_90`, `Z_99`
- Порт `BSDPI.AI/Math/WilsonScore.cs`

### fingerprint.rs
- `NetworkFingerprint` — хеш сети (SHA256 по транспорту, шлюзу, DNS, подсети)
- `FingerprintProvider` trait — для платформо-зависимой реализации
- `BasicFingerprintProvider` — для тестов
- Порт `Models/NetworkFingerprint.cs`

### genome.rs
- `StrategyGenome` — 50+ полей DPI bypass параметров
- `DpiEngineType` — Zapret, ByeDpi, Warp
- `StrategyOrigin` — Builtin, Evolved, Imported, Manual
- `default_zapret()`, `default_byedpi()`, `default_warp()`
- Serialization: `serde` + `serde_json`
- Порт `Models/StrategyGenome.cs`

### signature.rs
- `compute(genome) -> String` — SHA256 сигнатура (только DPI параметры, без метаданных)
- `compute_set(genomes) -> Vec<String>` — уникальные сигнатуры
- `exists_in(genome, pool) -> bool` — проверка дубликата
- Порт `Models/GenomeSignature.cs`

### bandit.rs
- `BanditSelector` — multi-armed bandit
- `BanditArm` — Beta(alpha, beta) распределение
- `pick()` — выбор стратегии с adaptive exploration
- Thompson Sampling (через Gamma + Normal семплы, Marsaglia & Tsang)
- UCB1 Upper Confidence Bound
- Pareto front (multi-objective: score ↑, latency ↓)
- Exponential backoff при неудачах (300ms → 700ms → 1500ms → 3000ms)
- Порт `Services/BanditSelector.cs`

### evolver.rs
- `StrategyEvolver` — генетический алгоритм
- `evolve(pool, outcomes) -> child` — создание новой стратегии
- Crossover: каждый параметр от случайного родителя (50+ полей)
- Mutation: 15 типов для Zapret, 10 для ByeDpi, 7 для Warp
- `gc_evolved()` — garbage collection слабых evolved (elitism)
- Валидация + дедупликация через GenomeSignature
- Порт `Services/StrategyEvolver.cs`

---

## ⚙️ Команды

```bash
cd /root/workspace/bsdpi-rs && . "$HOME/.cargo/env"

# Сборка
cargo build -p bsdpi-ai

# Тесты (50/50)
cargo test -p bsdpi-ai -v

# Запуск конкретного теста
cargo test -p bsdpi-ai evolver::tests::test_evolve_returns_child -- --nocapture
```

---

## 📝 TODO (следующие шаги)

1. **registry.rs** — AiStrategyRegistry (persistent storage через sled/JSON)
2. **history.rs** — AiHistoryStore (append-only log)
3. **orchestrator.rs** — AiOrchestratorService (state machine)
4. **bsdpi-engine** — DPI Engine traits + Zapret/ByeDpi/Warp impl
5. **bsdpi-core** — probling, settings, updater
6. **bsdpi-gui** — egui frontend
