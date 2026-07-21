//! # BSDPI DPI Engine Layer
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

pub mod traits;
pub mod zapret;
pub mod byedpi;
pub mod warp;

pub use traits::{DpiEngine, EngineStatus, EngineProcessInfo, EngineEvent, EngineError};
pub use zapret::ZapretEngine;
pub use byedpi::ByeDpiEngine;
pub use warp::WarpEngine;
