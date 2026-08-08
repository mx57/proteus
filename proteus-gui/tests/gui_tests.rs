use proteus_gui::state::AppState;

#[test]
fn test_app_state_creation() {
    let state = AppState::new();
    assert_eq!(state.version, "0.1.0");
    assert_eq!(state.active_tab, "main");
    assert_eq!(state.engine_status, "Stopped");
    assert_eq!(state.active_engine, "none");
    assert_eq!(state.selected_chain, "Zapret");
    assert_eq!(state.socks_port, 1080);
    assert_eq!(state.auto_start, true);
    assert_eq!(state.check_interval_secs, 30);
    assert_eq!(state.evolution_interval_mins, 60);
    assert_eq!(state.engine_dir, "engine");
    assert_eq!(state.log_level, "info");

    assert!(state.logs.is_empty());
    assert!(!state.bandit_arms.is_empty());
}

#[test]
fn test_app_state_uptime_str() {
    let state = AppState::new();
    // Simulate some time passed
    let uptime = state.uptime_str();
    assert!(!uptime.is_empty());
}

#[test]
fn test_add_log() {
    let mut state = AppState::new();
    assert!(state.logs.is_empty());

    state.add_log("info", "Test log 1".to_string());
    assert_eq!(state.logs.len(), 1);

    let last_log = state.logs.back().unwrap();
    assert_eq!(last_log.level, "info");
    assert_eq!(last_log.message, "Test log 1");

    // Test capacity limit
    for i in 0..1500 {
        state.add_log("debug", format!("Log {}", i));
    }

    assert_eq!(state.logs.len(), 1000); // the max limit is 1000

    let last = state.logs.back().unwrap();
    assert_eq!(last.message, "Log 1499");
}
