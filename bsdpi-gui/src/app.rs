//! Основное состояние и логика приложения BSDPI GUI.

use eframe::egui;

use crate::panels;
use crate::state::AppState;

/// Основное приложение.
pub struct BsdpiApp {
    state: AppState,
}

impl BsdpiApp {
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
        }
    }
}

impl eframe::App for BsdpiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Запрос на постоянное обновление
        ctx.request_repaint_after(std::time::Duration::from_millis(250));

        // Верхняя панель с табами
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("⚡ BSDPI");
                ui.separator();

                // Табы
                let tabs = [
                    ("📊 Main", "main"),
                    ("🧠 AI", "ai"),
                    ("⚙️ Engine", "engine"),
                    ("⛓️ Chains", "chains"),
                    ("🔧 Settings", "settings"),
                    ("📝 Logs", "logs"),
                ];

                for (label, id) in &tabs {
                    let selected = self.state.active_tab == *id;
                    if ui.selectable_label(selected, *label).clicked() {
                        self.state.active_tab = id.to_string();
                    }
                }
            });
        });

        // Нижняя панель со статус-баром
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Engine: {}", self.state.engine_status));
                ui.separator();
                ui.label(format!("Active: {}", self.state.active_engine));
                ui.separator();
                ui.label(format!("Uptime: {}", self.state.uptime_str()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("v{}", self.state.version));
                });
            });
        });

        // Центральная область — содержимое таба
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state.active_tab.as_str() {
                "main" => panels::main_tab(ui, &mut self.state),
                "ai" => panels::ai_tab(ui, &mut self.state),
                "engine" => panels::engine_tab(ui, &mut self.state),
                "chains" => panels::chains_tab(ui, &mut self.state),
                "settings" => panels::settings_tab(ui, &mut self.state),
                "logs" => panels::logs_tab(ui, &mut self.state),
                _ => panels::main_tab(ui, &mut self.state),
            }
        });
    }
}
