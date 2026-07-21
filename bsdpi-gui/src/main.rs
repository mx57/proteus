//! # BSDPI GUI — egui/eframe desktop application
//!
//! Вкладки:
//! - **Main** — состояние оркестратора
//! - **AI** — Bandit/Evolver/Fingerprint мониторинг
//! - **Engine** — DPI движки (start/stop/probe/logs)
//! - **Settings** — TOML конфиг
//! - **Chains** — конструктор цепочек
//! - **Logs** — просмотр логов

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

mod app;
mod panels;
mod state;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "BSDPI",
        options,
        Box::new(|_cc| Ok(Box::new(app::BsdpiApp::new()))),
    )
}
