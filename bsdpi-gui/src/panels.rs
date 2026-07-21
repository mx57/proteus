//! Панели GUI для каждой вкладки.

use crate::state::AppState;
use eframe::egui;

/// Главная вкладка — дашборд состояния.
pub fn main_tab(ui: &mut egui::Ui, state: &mut AppState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("📊 Dashboard");
        ui.separator();

        egui::Grid::new("main_grid").striped(true).show(ui, |ui| {
            ui.label("Engine Status:");
            ui.label(&state.engine_status);
            ui.end_row();

            ui.label("Active Engine:");
            ui.label(&state.active_engine);
            ui.end_row();

            ui.label("Uptime:");
            ui.label(&state.uptime_str());
            ui.end_row();

            ui.label("Network Fingerprint:");
            ui.label(&state.fingerprint_hash);
            ui.end_row();

            ui.label("Evolution Generation:");
            ui.label(&state.evolver_generation.to_string());
            ui.end_row();
        });

        ui.separator();

        // Действия
        ui.horizontal(|ui| {
            if ui.button("▶ Start").clicked() {
                state.engine_status = "Running".into();
                state.active_engine = "Zapret".into();
                state.add_log("info", "Engine started".into());
            }
            if ui.button("■ Stop").clicked() {
                state.engine_status = "Stopped".into();
                state.active_engine = "none".into();
                state.add_log("info", "Engine stopped".into());
            }
            if ui.button("🔄 Probe").clicked() {
                state.add_log("info", "Probe initiated".into());
            }
        });
    });
}

/// AI вкладка — Bandit / Evolver / Fingerprint.
pub fn ai_tab(ui: &mut egui::Ui, state: &mut AppState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("🧠 AI Orchestrator");
        ui.separator();

        // Секция Bandit
        ui.heading("Thompson Sampling — Bandit Arms");
        ui.label("Wilson Score ranking of active strategies:");

        let columns = [
            ("Strategy", 200.0),
            ("Mean Reward", 120.0),
            ("Pulls", 80.0),
            ("Bar", 200.0),
        ];

        egui::Grid::new("bandit_grid")
            .striped(true)
            .min_col_width(80.0)
            .show(ui, |ui| {
                // Header
                for (name, _) in &columns {
                    ui.strong(*name);
                }
                ui.end_row();

                // Data rows
                for arm in &state.bandit_arms {
                    ui.label(&arm.name);
                    ui.label(format!("{:.2}", arm.mean_reward));
                    ui.label(arm.pulls.to_string());

                    // Progress bar for mean reward
                    ui.add(egui::ProgressBar::new(arm.mean_reward as f32).desired_width(200.0));
                    ui.end_row();
                }
            });

        ui.separator();

        // Fingerprint секция
        ui.heading("🌐 Network Fingerprint");
        ui.horizontal(|ui| {
            ui.label("Hash:");
            ui.code(&state.fingerprint_hash);
        });
        if ui.button("🔄 Refresh Fingerprint").clicked() {
            state.add_log("info", "Fingerprint refreshed".into());
        }

        ui.separator();

        // Evolution секция
        ui.heading("🧬 Genetic Evolution");
        ui.horizontal(|ui| {
            ui.label("Generation:");
            ui.label(state.evolver_generation.to_string());
            if ui.button("▶ Evolve").clicked() {
                state.evolver_generation += 1;
                state.add_log("info", format!("Evolution triggered: gen {}", state.evolver_generation));
            }
        });
    });
}

/// Engine вкладка — управление DPI движками.
pub fn engine_tab(ui: &mut egui::Ui, state: &mut AppState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("⚙️ DPI Engine Manager");
        ui.separator();

        let engines = ["Zapret", "ByeDPI", "Warp", "Hybrid", "Chained"];

        egui::Grid::new("engine_grid")
            .striped(true)
            .min_col_width(120.0)
            .show(ui, |ui| {
                ui.strong("Engine");
                ui.strong("Status");
                ui.strong("Actions");
                ui.end_row();

                for engine in &engines {
                    let is_active = state.active_engine == *engine;
                    ui.label(format!("{}", engine));
                    ui.label(if is_active { "🟢 Running" } else { "⏹️ Stopped" });

                    ui.horizontal(|ui| {
                        if ui.button("▶").on_hover_text("Start").clicked() {
                            state.engine_status = "Running".into();
                            state.active_engine = engine.to_string();
                            state.add_log("info", format!("Engine started: {}", engine));
                        }
                        if ui.button("■").on_hover_text("Stop").clicked() {
                            state.engine_status = "Stopped".into();
                            state.active_engine = "none".into();
                            state.add_log("info", format!("Engine stopped: {}", engine));
                        }
                        if ui.button("🔍").on_hover_text("Probe").clicked() {
                            state.add_log("info", format!("Probing: {}", engine));
                        }
                    });
                    ui.end_row();
                }
            });

        ui.separator();
        ui.heading("Engine Log");
        egui::ScrollArea::vertical()
            .id_source("engine_log_scroll")
            .max_height(200.0)
            .show(ui, |ui| {
                for entry in state.logs.iter().rev().take(20) {
                    let time = entry.timestamp.format("%H:%M:%S");
                    let color = match entry.level.as_str() {
                        "error" | "critical" => egui::Color32::RED,
                        "warn" => egui::Color32::YELLOW,
                        _ => egui::Color32::WHITE,
                    };
                    ui.colored_label(color, format!("[{}] {}", time, entry.message));
                }
            });
    });
}

/// Chains вкладка — конструктор цепочек.
pub fn chains_tab(ui: &mut egui::Ui, state: &mut AppState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("⛓️ Chain Builder");
        ui.separator();

        let modes = [
            ("Zapret", "low", "Only Zapret"),
            ("ByeDPI", "low", "Only ByeDPI"),
            ("Warp", "low", "Only Warp"),
            ("Hybrid", "medium", "Zapret + ByeDPI parallel"),
            ("Warp+Zapret", "medium", "Warp + Zapret parallel"),
            ("Warp+ByeDPI", "medium", "Warp + ByeDPI parallel"),
            ("Warp→Zapret Chained", "extreme", "Warp SOCKS5 → Zapret"),
            ("Warp→ByeDPI Chained", "extreme", "Warp SOCKS5 → ByeDPI"),
            ("Bypass", "none", "No protection"),
        ];

        egui::Grid::new("chains_grid")
            .striped(true)
            .min_col_width(150.0)
            .show(ui, |ui| {
                ui.strong("Mode");
                ui.strong("Difficulty");
                ui.strong("Description");
                ui.strong("");
                ui.end_row();

                for (name, difficulty, desc) in &modes {
                    let selected = state.selected_chain == *name;
                    ui.label(*name);
                    ui.label(match *difficulty {
                        "low"     => "🟢 Low",
                        "medium"  => "🟡 Medium",
                        "extreme" => "🔴 Extreme",
                        _         => "⚪ None",
                    });
                    ui.label(*desc);
                    if ui.selectable_label(selected, "Select").clicked() {
                        state.selected_chain = name.to_string();
                        state.add_log("info", format!("Chain selected: {}", name));
                    }
                    ui.end_row();
                }
            });
    });
}

/// Settings вкладка — редактирование конфига.
pub fn settings_tab(ui: &mut egui::Ui, state: &mut AppState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("🔧 Settings");
        ui.separator();

        ui.heading("Network");
        ui.add(egui::Slider::new(&mut state.socks_port, 1024..=65535).text("SOCKS Port"));
        ui.add(egui::Slider::new(&mut state.check_interval_secs, 5..=300).text("Check Interval (s)"));
        ui.add(egui::Slider::new(&mut state.evolution_interval_mins, 1..=1440).text("Evolution Interval (min)"));

        ui.separator();
        ui.heading("General");
        ui.horizontal(|ui| {
            ui.label("Engine Dir:");
            ui.text_edit_singleline(&mut state.engine_dir);
        });

        ui.checkbox(&mut state.auto_start, "Auto start on launch");

        ui.horizontal(|ui| {
            ui.label("Log Level:");
            egui::ComboBox::from_id_salt("log_level")
                .selected_text(&state.log_level)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.log_level, "error".into(), "Error");
                    ui.selectable_value(&mut state.log_level, "warn".into(), "Warn");
                    ui.selectable_value(&mut state.log_level, "info".into(), "Info");
                    ui.selectable_value(&mut state.log_level, "debug".into(), "Debug");
                });
        });

        ui.separator();
        if ui.button("💾 Save Config").clicked() {
            state.add_log("info", "Settings saved".into());
        }
        if ui.button("↩️ Reset to Defaults").clicked() {
            state.socks_port = 1080;
            state.auto_start = true;
            state.check_interval_secs = 30;
            state.evolution_interval_mins = 60;
            state.engine_dir = "engine".into();
            state.add_log("info", "Settings reset to defaults".into());
        }
    });
}

/// Logs вкладка — просмотр логов.
pub fn logs_tab(ui: &mut egui::Ui, state: &mut AppState) {
    egui::ScrollArea::vertical()
        .id_source("logs_scroll")
        .show(ui, |ui| {
            ui.heading("📝 Log Viewer");
            ui.separator();

            if state.logs.is_empty() {
                ui.label("No log entries yet.");
                return;
            }

            for entry in state.logs.iter().rev() {
                let time = entry.timestamp.format("%Y-%m-%d %H:%M:%S");
                let color = match entry.level.as_str() {
                    "error" | "critical" => egui::Color32::RED,
                    "warn" => egui::Color32::YELLOW,
                    "info" => egui::Color32::LIGHT_BLUE,
                    _ => egui::Color32::WHITE,
                };
                ui.colored_label(color, format!("[{}] [{}] {}", time, entry.level, entry.message));
            }
        });
}
