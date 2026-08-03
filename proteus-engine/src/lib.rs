//! # Proteus DPI Engine Layer
//!
//! Абстракция над DPI-движками: Zapret (winws.exe), ByeDPI (ciadpi.exe), Warp (warp-go).
//! Платформонезависимая — `cfg(target_os)` выбирает бинарник и опции.
//!
//! ## Архитектура
//!
//! DpiEngine trait (общий для всех)
//! - ZapretEngine  — запуск winws.exe / нативный zapret
//! - ByeDpiEngine  — запуск ciadpi.exe / нативный byedpi
//! - WarpEngine    — запуск warp-go / warp-plus
//!
//! Каждый Engine реализует:
//! - `async start(profile)` — запуск процесса
//! - `async stop()` — остановка (kill tree)
//! - `async probe()` — проверка статуса
//! - events через broadcast канал

pub mod byedpi;
pub mod traits;
pub mod warp;
pub mod zapret;

pub use byedpi::ByeDpiEngine;
pub use traits::{DpiEngine, DpiEngineType, EngineError, EngineEvent, EngineProcessInfo, EngineProfile, EngineStatus};
pub use warp::WarpEngine;
pub use zapret::ZapretEngine;
