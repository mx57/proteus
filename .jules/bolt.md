## 2026-07-22 - GitHub Updates
**Инсайт:** Логика проверки обновлений частично дублировалась в CLI и core. `reqwest` зависимость отсутствовала feature "json" в bsdpi-core.
**Действие:** Завершать функции-заглушки (SelfUpdater в bsdpi-core) и переносить в них дублирующийся код для сохранения чистоты архитектуры.
## 2026-07-23 - State Machine Architecture for AI Core
**Инсайт:** AiOrchestratorService implemented using a strict state machine pattern (`OrchestratorState`). This ensures valid transitions (e.g., cannot Verify if not Executing) and correctly models the AI lifecycle: `Idle -> Fingerprinting -> Selecting -> Executing -> Verifying -> Evolving (or Selecting) -> Idle`.
**Действие:** Always use explicit state tracking for services coordinating multiple asynchronous or sequential tasks to prevent inconsistent states.
