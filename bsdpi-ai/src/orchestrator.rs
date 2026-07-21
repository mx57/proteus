//! AiOrchestratorService — конечный автомат AI-оркестрации DPI.
//!
//! Порт C# `BSDPI.AI/Services/AiOrchestratorService.cs`:
//! Цикл: Fingerprint → Select → Execute → Verify → Evolve
//!
//! Состояния:
//! - Idle — ожидание
//! - Fingerprinting — анализ сети
//! - Selecting — выбор стратегии
//! - Executing — запуск DPI
//! - Verifying — проверка работоспособности
//! - Evolving — генетическая эволюция
//! - Error — ошибка (с восстановлением)

use std::sync::{Arc, Mutex};

/// Состояние оркестратора.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorState {
    Idle,
    Fingerprinting,
    Selecting,
    Executing,
    Verifying,
    Evolving,
    Error,
}

impl OrchestratorState {
    pub fn is_active(&self) -> bool {
        !matches!(self, OrchestratorState::Idle | OrchestratorState::Error)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            OrchestratorState::Idle => "idle",
            OrchestratorState::Fingerprinting => "fingerprinting",
            OrchestratorState::Selecting => "selecting",
            OrchestratorState::Executing => "executing",
            OrchestratorState::Verifying => "verifying",
            OrchestratorState::Evolving => "evolving",
            OrchestratorState::Error => "error",
        }
    }
}

/// Конфигурация оркестратора.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Интервал проверки сети (сек)
    pub check_interval_secs: u64,
    /// Интервал эволюции (мин)
    pub evolution_interval_mins: u64,
    /// Количество стратегий для Fast Start
    pub fast_start_count: u32,
    /// Максимальное количество ошибок до остановки
    pub max_errors: u32,
    /// Время ожидания проверки (сек)
    pub verify_timeout_secs: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            evolution_interval_mins: 60,
            fast_start_count: 3,
            max_errors: 5,
            verify_timeout_secs: 10,
        }
    }
}

/// События оркестратора.
#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    StateChanged(OrchestratorState),
    StrategyApplied(String),
    EvolutionTriggered,
    Error(String),
    FingerprintChanged(String),
    VerificationResult { strategy_id: String, success: bool, score: u32 },
}

/// Статистика работы оркестратора.
#[derive(Debug, Clone)]
pub struct OrchestratorStats {
    pub total_checks: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub total_evolutions: u64,
    pub current_streak: u32,
    pub best_streak: u32,
}

impl Default for OrchestratorStats {
    fn default() -> Self {
        Self {
            total_checks: 0,
            total_successes: 0,
            total_failures: 0,
            total_evolutions: 0,
            current_streak: 0,
            best_streak: 0,
        }
    }
}

/// AI-оркестратор — конечный автомат управления DPI.
pub struct AiOrchestratorService {
    /// Текущее состояние
    pub state: OrchestratorState,
    /// Конфигурация
    pub config: OrchestratorConfig,
    /// Текущий хеш слепка сети
    pub current_fingerprint_hash: String,
    /// Предыдущий хеш слепка сети (для обнаружения смены)
    pub previous_fingerprint_hash: String,
    /// ID текущей активной стратегии
    pub current_strategy_id: Option<String>,
    /// Счётчик ошибок
    pub error_count: u32,
    /// Статистика
    pub stats: OrchestratorStats,
    /// Счётчик эволюций
    pub evolution_count: u64,
    /// Список подписчиков событий
    event_handlers: Vec<Box<dyn Fn(&OrchestratorEvent) + Send + Sync>>,
}

impl AiOrchestratorService {
    /// Создать новый оркестратор.
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            state: OrchestratorState::Idle,
            config,
            current_fingerprint_hash: String::new(),
            previous_fingerprint_hash: String::new(),
            current_strategy_id: None,
            error_count: 0,
            stats: OrchestratorStats::default(),
            evolution_count: 0,
            event_handlers: Vec::new(),
        }
    }

    /// Добавить обработчик событий.
    pub fn on_event<F>(&mut self, handler: F)
    where
        F: Fn(&OrchestratorEvent) + Send + Sync + 'static,
    {
        self.event_handlers.push(Box::new(handler));
    }

    /// Оповестить подписчиков о событии.
    fn emit(&self, event: &OrchestratorEvent) {
        for handler in &self.event_handlers {
            handler(event);
        }
    }

    // ─── Переходы состояний ───

    /// Перейти в новое состояние.
    pub fn transition_to(&mut self, new_state: OrchestratorState) {
        let old_state = self.state;
        self.state = new_state;
        self.emit(&OrchestratorEvent::StateChanged(new_state));
        log::info!("Orchestrator: {:?} → {:?}", old_state, new_state);
    }

    /// Запустить оркестратор (Idle → Fingerprinting).
    pub fn start(&mut self) {
        self.error_count = 0;
        self.transition_to(OrchestratorState::Fingerprinting);
    }

    /// Остановить оркестратор (любое состояние → Idle).
    pub fn stop(&mut self) {
        self.current_strategy_id = None;
        self.transition_to(OrchestratorState::Idle);
    }

    /// Завершить fingerprinting (Fingerprinting → Selecting).
    pub fn on_fingerprint_complete(&mut self, fingerprint_hash: String) {
        if self.state != OrchestratorState::Fingerprinting {
            return;
        }

        self.previous_fingerprint_hash = self.current_fingerprint_hash.clone();
        self.current_fingerprint_hash = fingerprint_hash.clone();

        if !self.previous_fingerprint_hash.is_empty()
            && self.previous_fingerprint_hash != fingerprint_hash
        {
            self.emit(&OrchestratorEvent::FingerprintChanged(fingerprint_hash));
        }

        self.transition_to(OrchestratorState::Selecting);
    }

    /// Выбрать стратегию (Selecting → Executing).
    pub fn on_strategy_selected(&mut self, strategy_id: String) {
        if self.state != OrchestratorState::Selecting {
            return;
        }
        self.current_strategy_id = Some(strategy_id.clone());
        self.emit(&OrchestratorEvent::StrategyApplied(strategy_id));
        self.transition_to(OrchestratorState::Executing);
    }

    /// DPI запущен (Executing → Verifying).
    pub fn on_engine_started(&mut self) {
        if self.state != OrchestratorState::Executing {
            return;
        }
        self.transition_to(OrchestratorState::Verifying);
    }

    /// DPI не удалось запустить (Executing → Selecting или Error).
    pub fn on_engine_failed(&mut self, error: String) {
        if self.state != OrchestratorState::Executing {
            return;
        }
        self.stats.total_failures += 1;
        self.stats.current_streak = 0;
        self.error_count += 1;

        if self.error_count >= self.config.max_errors {
            self.emit(&OrchestratorEvent::Error(format!("Max errors: {}", error)));
            self.transition_to(OrchestratorState::Error);
        } else {
            self.emit(&OrchestratorEvent::Error(error));
            self.transition_to(OrchestratorState::Selecting);
        }
    }

    /// Проверка прошла успешно (Verifying → Fingerprinting).
    pub fn on_verification_passed(&mut self, score: u32) {
        if self.state != OrchestratorState::Verifying {
            return;
        }
        self.stats.total_checks += 1;
        self.stats.total_successes += 1;
        self.stats.current_streak += 1;
        if self.stats.current_streak > self.stats.best_streak {
            self.stats.best_streak = self.stats.current_streak;
        }
        self.error_count = 0;

        if let Some(ref id) = self.current_strategy_id {
            self.emit(&OrchestratorEvent::VerificationResult {
                strategy_id: id.clone(),
                success: true,
                score,
            });
        }

        self.transition_to(OrchestratorState::Fingerprinting);
    }

    /// Проверка провалилась (Verifying → Selecting или Error).
    pub fn on_verification_failed(&mut self, error: String) {
        if self.state != OrchestratorState::Verifying {
            return;
        }
        self.stats.total_checks += 1;
        self.stats.total_failures += 1;
        self.stats.current_streak = 0;
        self.error_count += 1;

        if let Some(ref id) = self.current_strategy_id {
            self.emit(&OrchestratorEvent::VerificationResult {
                strategy_id: id.clone(),
                success: false,
                score: 0,
            });
        }

        if self.error_count >= self.config.max_errors {
            self.emit(&OrchestratorEvent::Error(format!("Max verification failures: {}", error)));
            self.transition_to(OrchestratorState::Error);
        } else {
            self.emit(&OrchestratorEvent::Error(error));
            self.transition_to(OrchestratorState::Selecting);
        }
    }

    /// Запустить эволюцию (любое активное состояние → Evolving → обратно).
    pub fn trigger_evolution(&mut self) {
        self.evolution_count += 1;
        self.stats.total_evolutions += 1;
        let prev_state = self.state;
        self.transition_to(OrchestratorState::Evolving);
        self.emit(&OrchestratorEvent::EvolutionTriggered);
        // Возвращаемся в предыдущее состояние после эволюции
        self.state = prev_state;
    }

    /// Восстановиться после ошибки (Error → Fingerprinting).
    pub fn recover(&mut self) {
        if self.state != OrchestratorState::Error {
            return;
        }
        self.error_count = 0;
        self.transition_to(OrchestratorState::Fingerprinting);
    }

    /// Проверить, нужно ли запускать эволюцию (по интервалу).
    pub fn should_evolve(&self, evolution_count: u64) -> bool {
        let interval = self.config.evolution_interval_mins;
        if interval == 0 {
            return false;
        }
        // Каждые `interval` минут
        evolution_count > 0 && evolution_count % interval == 0
    }

    /// Получить подсказку следующего действия (для внешнего планировщика).
    pub fn next_action(&self) -> OrchestratorState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let orch = AiOrchestratorService::new(OrchestratorConfig::default());
        assert_eq!(orch.state, OrchestratorState::Idle);
        assert!(orch.current_strategy_id.is_none());
        assert_eq!(orch.error_count, 0);
    }

    #[test]
    fn test_state_active() {
        assert!(!OrchestratorState::Idle.is_active());
        assert!(OrchestratorState::Selecting.is_active());
        assert!(OrchestratorState::Executing.is_active());
        assert!(OrchestratorState::Verifying.is_active());
        assert!(OrchestratorState::Fingerprinting.is_active());
        assert!(OrchestratorState::Evolving.is_active());
        assert!(!OrchestratorState::Error.is_active());
    }

    #[test]
    fn test_default_config() {
        let cfg = OrchestratorConfig::default();
        assert_eq!(cfg.check_interval_secs, 30);
        assert_eq!(cfg.evolution_interval_mins, 60);
        assert_eq!(cfg.fast_start_count, 3);
    }

    #[test]
    fn test_full_lifecycle() {
        let mut orch = AiOrchestratorService::new(OrchestratorConfig::default());

        // Idle → Fingerprinting
        orch.start();
        assert_eq!(orch.state, OrchestratorState::Fingerprinting);

        // Fingerprinting → Selecting
        orch.on_fingerprint_complete("hash-1".into());
        assert_eq!(orch.state, OrchestratorState::Selecting);

        // Selecting → Executing
        orch.on_strategy_selected("s1".into());
        assert_eq!(orch.state, OrchestratorState::Executing);
        assert_eq!(orch.current_strategy_id.as_deref(), Some("s1"));

        // Executing → Verifying
        orch.on_engine_started();
        assert_eq!(orch.state, OrchestratorState::Verifying);

        // Verifying → Fingerprinting (цикл замкнулся)
        orch.on_verification_passed(95);
        assert_eq!(orch.state, OrchestratorState::Fingerprinting);
    }

    #[test]
    fn test_start_stop() {
        let mut orch = AiOrchestratorService::new(OrchestratorConfig::default());
        orch.start();
        assert_eq!(orch.state, OrchestratorState::Fingerprinting);

        orch.stop();
        assert_eq!(orch.state, OrchestratorState::Idle);
        assert!(orch.current_strategy_id.is_none());
    }

    #[test]
    fn test_error_handling() {
        let mut orch = AiOrchestratorService::new(OrchestratorConfig {
            max_errors: 2,
            ..Default::default()
        });

        orch.start();
        orch.on_fingerprint_complete("hash-1".into());
        orch.on_strategy_selected("s1".into());

        // Первая ошибка: должно перейти в Selecting (повтор)
        orch.on_engine_failed("timeout".into());
        assert_eq!(orch.state, OrchestratorState::Selecting);
        assert_eq!(orch.error_count, 1);

        orch.on_strategy_selected("s2".into());
        orch.on_engine_started();

        // Вторая ошибка: max_errors = 2, должно перейти в Error
        orch.on_verification_failed("connection lost".into());
        assert_eq!(orch.state, OrchestratorState::Error);
        assert_eq!(orch.error_count, 2);
    }

    #[test]
    fn test_recovery() {
        let mut orch = AiOrchestratorService::new(OrchestratorConfig {
            max_errors: 1,
            ..Default::default()
        });

        orch.start();
        orch.on_fingerprint_complete("hash-1".into());
        orch.on_strategy_selected("s1".into());
        orch.on_engine_failed("crash".into());
        assert_eq!(orch.state, OrchestratorState::Error);

        orch.recover();
        assert_eq!(orch.state, OrchestratorState::Fingerprinting);
        assert_eq!(orch.error_count, 0);
    }

    #[test]
    fn test_evolution_trigger() {
        let mut orch = AiOrchestratorService::new(OrchestratorConfig::default());

        orch.start();
        orch.on_fingerprint_complete("hash-1".into());
        orch.on_strategy_selected("s1".into());

        orch.trigger_evolution();
        assert_eq!(orch.evolution_count, 1);
        assert_eq!(orch.stats.total_evolutions, 1);
    }

    #[test]
    fn test_fingerprint_change() {
        let mut orch = AiOrchestratorService::new(OrchestratorConfig::default());
        orch.current_fingerprint_hash = "old-hash".into();

        orch.start();
        orch.on_fingerprint_complete("new-hash".into());

        assert_eq!(orch.current_fingerprint_hash, "new-hash");
        assert_eq!(orch.previous_fingerprint_hash, "old-hash");
    }

    #[test]
    fn test_should_evolve() {
        let orch = AiOrchestratorService::new(OrchestratorConfig {
            evolution_interval_mins: 5,
            ..Default::default()
        });

        // Каждые 5 минут
        assert!(!orch.should_evolve(0));
        assert!(!orch.should_evolve(4));
        assert!(orch.should_evolve(5));
        assert!(orch.should_evolve(10));
    }

    #[test]
    fn test_stats_tracking() {
        let mut orch = AiOrchestratorService::new(OrchestratorConfig::default());

        orch.start();
        orch.on_fingerprint_complete("h1".into());
        orch.on_strategy_selected("s1".into());
        orch.on_engine_started();
        orch.on_verification_passed(90);

        assert_eq!(orch.stats.total_checks, 1);
        assert_eq!(orch.stats.total_successes, 1);
        assert_eq!(orch.stats.current_streak, 1);
        assert_eq!(orch.stats.best_streak, 1);
    }

    #[test]
    fn test_state_as_str() {
        assert_eq!(OrchestratorState::Idle.as_str(), "idle");
        assert_eq!(OrchestratorState::Fingerprinting.as_str(), "fingerprinting");
        assert_eq!(OrchestratorState::Executing.as_str(), "executing");
        assert_eq!(OrchestratorState::Verifying.as_str(), "verifying");
        assert_eq!(OrchestratorState::Evolving.as_str(), "evolving");
        assert_eq!(OrchestratorState::Error.as_str(), "error");
    }

    #[test]
    fn test_verification_failure_triggers_error() {
        let mut orch = AiOrchestratorService::new(OrchestratorConfig {
            max_errors: 2,
            ..Default::default()
        });

        // Цикл 1: успех
        orch.start();
        orch.on_fingerprint_complete("h1".into());
        orch.on_strategy_selected("s1".into());
        orch.on_engine_started();
        orch.on_verification_passed(90);
        assert_eq!(orch.error_count, 0);

        // Цикл 2: ошибка
        orch.on_fingerprint_complete("h1".into());
        orch.on_strategy_selected("s2".into());
        orch.on_engine_started();
        orch.on_verification_failed("timeout".into());
        assert_eq!(orch.state, OrchestratorState::Selecting);
        assert_eq!(orch.error_count, 1);

        // Цикл 3: ещё одна ошибка → Error
        orch.on_strategy_selected("s3".into());
        orch.on_engine_started();
        orch.on_verification_failed("crash".into());
        assert_eq!(orch.state, OrchestratorState::Error);
    }

    #[test]
    fn test_state_transition_guards() {
        let mut orch = AiOrchestratorService::new(OrchestratorConfig::default());

        // Попытка завершить fingerprinting когда мы в Idle — игнорируется
        orch.on_fingerprint_complete("h1".into());
        assert_eq!(orch.state, OrchestratorState::Idle);

        // Попытка выбрать стратегию когда мы в Idle — игнорируется
        orch.on_strategy_selected("s1".into());
        assert_eq!(orch.state, OrchestratorState::Idle);
    }

    #[test]
    fn test_event_handler() {
        let mut orch = AiOrchestratorService::new(OrchestratorConfig::default());
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        orch.on_event(move |event| {
            events_clone.lock().unwrap().push(format!("{:?}", event));
        });

        orch.start();
        drop(orch);

        let captured = events.lock().unwrap();
        assert!(!captured.is_empty());
    }
}
