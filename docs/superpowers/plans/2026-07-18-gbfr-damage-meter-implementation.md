# GBFR Korean Damage Meter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Granblue Fantasy: Relink Endless Ragnarok 2.0.2에서 파티원별 캐릭터 이름, 누적 피해 바, 누적 피해량과 DPS를 한국어 소형 오버레이로 표시한다.

**Architecture:** Awa Edition 1.8.6의 `src-hook` DLL이 게임 데미지 이벤트를 named pipe로 보내고, Rust/Tauri 파서가 전투 상태를 집계한다. React 프런트엔드는 250ms마다 최신 상태를 반영하며 `InProgress` 동안만 330x145 투명 창을 그린다. 리워드 UI 진입 직전에 `OnBattleEnd`를 보내 저장한 뒤 집계를 비우므로 결과가 리워드 화면에 겹치지 않는다.

**Tech Stack:** Windows x64, Rust nightly-2024-05-04, Cargo, Tauri 1.5, Node.js 20, React 18, TypeScript 5, Zustand, i18next, Vitest, Testing Library, MSI/WiX

## Global Constraints

- 게임 대상은 Windows/Steam의 Granblue Fantasy: Relink Endless Ragnarok 2.0.2다.
- 기준 소스는 Awa Edition annotated tag `1.8.6`, commit `8d724ef436d12a5a409dafb41d81e200c025a733`이다.
- 기본 언어는 한국어(`ko`)이며 한국어 리소스가 없을 때만 영어로 대체한다.
- 1920x1080 기준 창 크기는 330x145px, 위치는 x=45/y=470이다.
- 표시 갱신은 250ms, 바 너비 보간은 150ms linear다.
- 최대 4명을 누적 피해 내림차순으로 표시하며 최고 누적 피해자의 바가 100%다.
- 표시 필드는 캐릭터 이름, 누적 피해 바, 누적 피해량, DPS뿐이다.
- 첫 유효 피해가 전투 시작이며 마지막 피해 뒤 DPS는 동결된다.
- 최종 결과는 `ResultRewardSetup::execute` 진입 직전까지 유지한 뒤 숨기고 초기화한다.
- Reloaded-II, 입력 자동화, 게임 데이터 변경, 외부 랭킹 전송 및 새로운 네트워크 기능은 추가하지 않는다.
- False Spring과 onelittlechildawa의 MIT 저작권 및 허가문을 보존한다.
- 각 구현 작업은 실패 테스트 확인 후 최소 구현, 전체 관련 테스트 확인, 커밋 순서로 진행한다.

---

## File Structure

- `protocol/src/lib.rs`: define the DLL-to-app core-hook readiness wire status.
- `src-hook/src/lib.rs`: send the `Ready`/`Unsupported` handshake first to every new pipe client.
- `src-hook/src/hooks/quest.rs`: 리워드 경계 이벤트를 원 게임 함수 호출 전에 전송한다.
- `src-hook/src/hooks/mod.rs`: 보조 identity 훅 실패를 core damage 훅과 격리한다.
- `src-tauri/src/parser/v1/mod.rs`: 결정적 시간 기반 집계, reward reset, 연결 단절 reset, unknown player 정책을 담당한다.
- `src-tauri/src/main.rs`: pipe 연결 상태 이벤트, 창 기본 geometry와 click-through 상태를 관리한다.
- `src/components/compact-meter/compactMeterModel.ts`: 표시 행 정렬, 상대 바 비율, 숫자 모델을 계산한다.
- `src/components/compact-meter/CompactDamageMeter.tsx`: 네 행짜리 한국어 미터를 렌더링한다.
- `src/components/compact-meter/CompactDamageMeter.css`: compact meter 전용 스타일만 보유한다.
- `src/pages/useCompactMeter.ts`: Tauri 이벤트를 구독하고 250ms 간격으로 UI 상태를 반영한다.
- `src/pages/Meter.tsx`: 연결/전투 상태에 따라 compact meter를 표시하거나 아무것도 그리지 않는다.
- `src/stores/useMeterSettingsStore.ts`: 투명도, 색상, streamer mode와 최초 geometry 적용 여부를 보존한다.
- `src/i18n.ts`, `src-tauri/lang/{ko,en}/ui.json`: 한국어 기본값과 compact/status 문구를 제공한다.
- `src-tauri/tauri.conf.json`: 330x145 투명 MSI 창과 새 제품 정보를 정의한다.

### Task 1: Import the Awa Edition 1.8.6 Baseline

**Files:**
- Import: Awa Edition tag `1.8.6` repository tree
- Preserve: `.gitignore`
- Preserve: `docs/superpowers/specs/2026-07-18-gbfr-damage-meter-design.md`
- Preserve: `docs/superpowers/plans/2026-07-18-gbfr-damage-meter-implementation.md`
- Verify: `LICENSE`, `README.md`, `package-lock.json`, `Cargo.lock`

**Interfaces:**
- Consumes: local `master` containing the approved design and this plan.
- Produces: branch `codex/gbfr-damage-meter` whose first parent is Awa `1.8.6` and whose history also contains local documentation commits.

- [ ] **Step 1: Verify the local documentation branch is clean and protected**

Run:

```powershell
git status --short --branch
git branch backup/pre-awa-import master
```

Expected: `master` is clean; `backup/pre-awa-import` points to the same commit as `master`.

- [ ] **Step 2: Fetch the exact release source reference**

Run:

```powershell
git remote add awa https://github.com/onelittlechildawa/gbfr-logs.git
git fetch awa refs/tags/1.8.6:refs/tags/awa/1.8.6
git rev-parse 'awa/1.8.6^{commit}'
```

Expected: `8d724ef436d12a5a409dafb41d81e200c025a733`. Stop if it differs.

- [ ] **Step 3: Create the implementation branch from Awa and merge the local docs history**

Run:

```powershell
git switch -c codex/gbfr-damage-meter 'awa/1.8.6^{commit}'
git merge --allow-unrelated-histories --no-ff master -m "merge: preserve GBFR damage meter design and plan"
```

Expected: only `.gitignore` may report an add/add conflict. If it conflicts, keep the Awa file and append exactly:

```gitignore
.superpowers/
```

Then run:

```powershell
git add .gitignore docs/superpowers
git commit --no-edit
```

- [ ] **Step 4: Verify both histories and the MIT baseline**

Run:

```powershell
git merge-base --is-ancestor 8d724ef436d12a5a409dafb41d81e200c025a733 HEAD
git merge-base --is-ancestor 011c26c HEAD
git status --short
Get-Content -Encoding utf8 LICENSE | Select-Object -First 6
```

Expected: both ancestry commands exit 0, status is clean, and LICENSE names False Spring and onelittlechildawa.

- [ ] **Step 5: Install dependencies and prove the imported baseline passes**

Run:

```powershell
npm ci
npm test -- --run
npm run tsc
npm run lint
npm run format-check
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: all existing TypeScript and Rust tests pass. Record any pre-existing failure before making feature changes.

### Task 2: Make the Reward Boundary and Encounter Lifecycle Deterministic

**Files:**
- Modify: `src-hook/src/hooks/quest.rs`
- Modify: `src-tauri/src/parser/v1/mod.rs`
- Test: inline `#[cfg(test)]` modules in both files

**Interfaces:**
- Consumes: existing `Message::OnBattleEnd`, `ParserStatus`, `DamageEvent` and `Parser::save_encounter_to_db`.
- Produces: `Parser::on_damage_event_at(event: DamageEvent, now: i64)` and a reward transition that saves once, resets to `Waiting`, and emits an empty encounter.

- [ ] **Step 1: Add failing deterministic lifecycle tests**

Add this helper and tests to `src-tauri/src/parser/v1/mod.rs`'s test module, using the module's existing imports:

```rust
fn test_damage(source_index: u32, target_index: u32, damage: i32) -> DamageEvent {
    DamageEvent {
        source: Actor {
            index: source_index,
            actor_type: 0x4C714F77,
            parent_index: source_index,
            parent_actor_type: 0x4C714F77,
        },
        target: Actor {
            index: target_index,
            actor_type: 0xDEAD0000 + target_index,
            parent_index: target_index,
            parent_actor_type: 0xDEAD0000 + target_index,
        },
        damage,
        flags: 0,
        action_id: ActionType::Normal(100),
        attack_rate: None,
        stun_value: None,
        damage_cap: None,
        details: None,
    }
}

#[test]
fn first_hit_starts_and_multiple_targets_share_one_encounter() {
    let mut parser = Parser::default();
    parser.on_damage_event_at(test_damage(7, 1, 1_000), 1_000);
    parser.on_damage_event_at(test_damage(7, 2, 3_000), 5_000);

    assert_eq!(parser.status, ParserStatus::InProgress);
    assert_eq!(parser.derived_state.start_time, 1_000);
    assert_eq!(parser.derived_state.end_time, 5_000);
    assert_eq!(parser.derived_state.total_damage, 4_000);
    assert_eq!(parser.derived_state.party.get(&7).unwrap().total_damage, 4_000);
    assert_eq!(parser.derived_state.targets.len(), 2);
    assert_eq!(parser.derived_state.party.get(&7).unwrap().dps, 1_000.0);
}

#[test]
fn reward_saves_then_clears_damage_and_stale_identity() {
    let mut parser = Parser::default();
    parser.encounter.player_data[0] = Some(PlayerData {
        actor_index: 7,
        display_name: "Player".into(),
        character_name: "Player".into(),
        character_type: CharacterType::Pl2400,
        sigils: Vec::new(),
        is_online: true,
        weapon_info: None,
        overmastery_info: None,
        player_stats: None,
    });
    parser.on_damage_event_at(test_damage(7, 1, 4_000), 1_000);
    let frozen_dps = parser.derived_state.party.get(&7).unwrap().dps;

    assert!(parser.on_battle_end_event());
    assert_eq!(parser.status, ParserStatus::Waiting);
    assert_eq!(parser.derived_state.status, ParserStatus::Waiting);
    assert_eq!(parser.derived_state.total_damage, 0);
    assert!(parser.derived_state.party.is_empty());
    assert!(parser.derived_state.targets.is_empty());
    assert!(parser.encounter.raw_event_log.is_empty());
    assert!(parser.encounter.player_data.iter().all(Option::is_none));
    assert!(!parser.on_battle_end_event());

    parser.on_damage_event_at(test_damage(7, 3, 500), 10_000);
    assert_eq!(parser.derived_state.total_damage, 500);
    assert_ne!(parser.derived_state.party.get(&7).unwrap().dps, frozen_dps);
}
```

- [ ] **Step 2: Run the parser tests and confirm RED**

Run:

```powershell
cargo test -p gbfr-logs parser::v1::tests -- --nocapture
```

Expected: compile failure because `on_damage_event_at` does not exist, then after declaring it without reset changes the reward test fails because status remains `Stopped`.

- [ ] **Step 3: Extract deterministic damage processing and reset after save**

Make the public handler delegate to a private deterministic method:

```rust
pub fn on_damage_event(&mut self, event: DamageEvent) {
    self.on_damage_event_at(event, Utc::now().timestamp_millis());
}

fn on_damage_event_at(&mut self, event: DamageEvent, now: i64) {
    if Self::should_ignore_damage_event(&event) {
        return;
    }

    if self.status == ParserStatus::Stopped || self.status == ParserStatus::Waiting {
        self.reset();
        self.derived_state.start(now);
        self.update_status(ParserStatus::InProgress);
    }

    self.encounter
        .push_event(now, Message::DamageEvent(event.clone()));
    let player_data = self
        .encounter
        .player_data
        .iter()
        .flatten()
        .find(|player| player.actor_index == event.source.parent_index);
    let damage_instance = AdjustedDamageInstance::from_damage_event(&event, player_data);
    self.derived_state
        .process_damage_event(now, &damage_instance);

    if let Some(window) = &self.window_handle {
        let _ = window.emit("encounter-update", &self.derived_state);
    }
}
```

Change `finish_and_save_encounter` so it completes the database write before clearing the UI state:

```rust
fn finish_and_save_encounter(&mut self) -> bool {
    self.update_status(ParserStatus::Stopped);

    let saved = match self.save_encounter_to_db() {
        Ok(id) => {
            if let Some(app) = &self.app {
                let _ = app.emit_all("encounter-saved", id);
            } else if let Some(window) = &self.window_handle {
                let _ = window.emit("encounter-saved", id);
            }
            true
        }
        Err(error) => {
            if let Some(app) = &self.app {
                let _ = app.emit_all("encounter-saved-error", error.to_string());
            } else if let Some(window) = &self.window_handle {
                let _ = window.emit("encounter-saved-error", error.to_string());
            }
            false
        }
    };

    self.reset();
    self.update_status(ParserStatus::Waiting);
    if let Some(window) = &self.window_handle {
        let _ = window.emit("encounter-update", &self.derived_state);
    }
    saved
}
```

- [ ] **Step 4: Add a failing hook-order test**

Add a pure helper test in `src-hook/src/hooks/quest.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::notify_before_original;
    use std::cell::RefCell;

    #[test]
    fn battle_end_notification_precedes_reward_setup() {
        let calls = RefCell::new(Vec::new());
        notify_before_original(
            || calls.borrow_mut().push("notify"),
            || calls.borrow_mut().push("original"),
        );
        assert_eq!(*calls.borrow(), vec!["notify", "original"]);
    }
}
```

Run:

```powershell
cargo test -p hook hooks::quest::tests -- --nocapture
```

Expected: compile failure because `notify_before_original` is undefined.

- [ ] **Step 5: Send `OnBattleEnd` before the original reward function**

Add and use this helper:

```rust
fn notify_before_original(notify: impl FnOnce(), original: impl FnOnce()) {
    notify();
    original();
}

fn run(&self, a1: *const usize) {
    notify_before_original(
        || {
            let _ = self.tx.send(Message::OnBattleEnd);
        },
        || unsafe { OnBattleEnd.call(a1) },
    );
}
```

- [ ] **Step 6: Verify and commit the lifecycle boundary**

Run:

```powershell
cargo test -p gbfr-logs parser::v1::tests -- --nocapture
cargo test -p hook hooks::quest::tests -- --nocapture
cargo test --workspace --all-targets --locked
git add src-hook/src/hooks/quest.rs src-tauri/src/parser/v1/mod.rs
git commit -m "fix: clear meter at the reward boundary"
```

Expected: all Rust tests pass; one focused commit is created.

### Task 3: Isolate Optional Hooks and Clear State on Disconnect

**Files:**
- Modify: `protocol/src/lib.rs`
- Modify: `src-hook/src/lib.rs`
- Modify: `src-hook/src/hooks/mod.rs`
- Modify: `src-tauri/src/parser/v1/mod.rs`
- Modify: `src-tauri/src/main.rs`
- Test: inline Rust unit tests

**Interfaces:**
- Consumes: `Parser::reset`, `ParserStatus::Waiting`, `PlayerIdentityEvent` and existing Tauri events.
- Produces: `HookStatus::{Ready, Unsupported}`, `Parser::on_connection_lost()`, `ConnectionState`, and identity-gated unknown-player aggregation.

- [ ] **Step 1: Write failing parser resilience tests**

Add to `src-tauri/src/parser/v1/mod.rs` tests:

```rust
#[test]
fn disconnect_discards_live_meter_without_saving() {
    let mut parser = Parser::default();
    parser.on_damage_event_at(test_damage(7, 1, 1_000), 1_000);
    parser.on_connection_lost();

    assert_eq!(parser.status, ParserStatus::Waiting);
    assert_eq!(parser.derived_state.total_damage, 0);
    assert!(parser.derived_state.party.is_empty());
    assert!(parser.encounter.raw_event_log.is_empty());
}

#[test]
fn identified_unknown_player_is_separate_but_unknown_enemy_is_ignored() {
    let mut parser = Parser::default();
    let unknown_hash = 0x11112222;
    parser.encounter.player_data[0] = Some(PlayerData {
        actor_index: 41,
        display_name: "Unknown Player".into(),
        character_name: "Unknown Player".into(),
        character_type: CharacterType::Unknown(unknown_hash),
        sigils: Vec::new(),
        is_online: true,
        weapon_info: None,
        overmastery_info: None,
        player_stats: None,
    });

    let mut player_hit = test_damage(41, 1, 700);
    player_hit.source.actor_type = unknown_hash;
    player_hit.source.parent_actor_type = unknown_hash;
    parser.on_damage_event_at(player_hit, 1_000);

    let mut enemy_hit = test_damage(99, 1, 900);
    enemy_hit.source.actor_type = 0x99998888;
    enemy_hit.source.parent_actor_type = 0x99998888;
    parser.on_damage_event_at(enemy_hit, 2_000);

    assert_eq!(parser.derived_state.party.len(), 1);
    assert_eq!(parser.derived_state.party.get(&41).unwrap().total_damage, 700);
}

#[test]
fn invalid_damage_is_ignored_but_a_large_valid_hit_is_preserved() {
    let mut parser = Parser::default();
    parser.on_damage_event_at(test_damage(7, 1, -1), 1_000);
    parser.on_damage_event_at(test_damage(7, 1, 1_000_000_000), 2_000);

    assert_eq!(parser.derived_state.total_damage, 1_000_000_000);
    assert_eq!(parser.encounter.raw_event_log.len(), 1);
}
```

Append this wire-format test to `protocol/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::{HookStatus, Message};

    #[test]
    fn hook_status_round_trips() {
        for status in [HookStatus::Ready, HookStatus::Unsupported] {
            let bytes = bincode::serialize(&Message::HookStatus(status)).unwrap();
            let decoded: Message = bincode::deserialize(&bytes).unwrap();
            assert!(matches!(decoded, Message::HookStatus(value) if value == status));
        }
    }
}
```

- [ ] **Step 2: Run resilience tests and confirm RED**

Run:

```powershell
cargo test -p gbfr-logs parser::v1::tests -- --nocapture
cargo test -p protocol hook_status_round_trips -- --nocapture
```

Expected: `on_connection_lost`, `HookStatus`, and `Message::HookStatus` are missing; identified unknown damage is ignored.

- [ ] **Step 3: Implement connection reset and identity-gated unknown handling**

Add:

```rust
pub fn on_connection_lost(&mut self) {
    self.reset();
    self.update_status(ParserStatus::Waiting);
    if let Some(window) = &self.window_handle {
        let _ = window.emit("encounter-update", &self.derived_state);
    }
}
```

Change `should_ignore_damage_event` to accept identity evidence:

```rust
fn should_ignore_damage_event(event: &DamageEvent, has_player_identity: bool) -> bool {
    if event.damage <= 0 {
        warn!("Ignoring non-positive damage event: {event:?}");
        return true;
    }
    if event.damage >= 1_000_000_000 {
        warn!("Suspiciously large damage event retained for diagnostics: {event:?}");
    }
    if event.target.actor_type == 0x022a350f {
        return true;
    }

    let character_type = CharacterType::from_hash(event.source.parent_actor_type);
    matches!(character_type, CharacterType::Unknown(_)) && !has_player_identity
}
```

Do not deduplicate equal events: legitimate multi-hit attacks can have identical values. Every accepted event remains in `raw_event_log`, while non-positive and unusually large values leave an explicit diagnostic log entry.

At the start of `on_damage_event_at`, compute identity without retaining a borrow:

```rust
let has_player_identity = self
    .encounter
    .player_data
    .iter()
    .flatten()
    .any(|player| player.actor_index == event.source.parent_index);
if Self::should_ignore_damage_event(&event, has_player_identity) {
    return;
}
```

- [ ] **Step 4: Add the hook readiness wire handshake**

Append `HookStatus` and its message variant in `protocol/src/lib.rs`; keep the new `Message` variant last so existing discriminants do not move:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookStatus {
    Ready,
    Unsupported,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    OnAreaEnter(AreaEnterEvent),
    OnQuestComplete(QuestCompleteEvent),
    DamageEvent(DamageEvent),
    OnUpdateSBA(OnUpdateSBAEvent),
    OnAttemptSBA(OnAttemptSBAEvent),
    OnPerformSBA(OnPerformSBAEvent),
    OnContinueSBAChain(OnContinueSBAChainEvent),
    PlayerLoadEvent(PlayerLoadEvent),
    OnDeathEvent(OnDeathEvent),
    OnBattleEnd,
    PlayerIdentityEvent(PlayerIdentityEvent),
    HookStatus(HookStatus),
}
```

In `src-hook/src/lib.rs`, change the existing helper and server method to send the status directly to every newly connected client before forwarding live broadcast messages:

```rust
async fn send_message(
    stream: &mut FramedWrite<SendPipeStream<pipe_mode::Bytes>, LengthDelimitedCodec>,
    message: &Message,
) -> Result<()> {
    let bytes = protocol::bincode::serialize(message)?;
    stream.send(bytes.into()).await?;
    Ok(())
}

async fn handle_client(
    mut stream: FramedWrite<SendPipeStream<pipe_mode::Bytes>, LengthDelimitedCodec>,
    mut rx: event::Rx,
    hook_status: HookStatus,
) -> Result<()> {
    send_message(&mut stream, &Message::HookStatus(hook_status)).await?;
    while let Ok(message) = rx.recv().await {
        send_message(&mut stream, &message).await?;
    }

    Ok(())
}

impl Server {
    async fn run(&self, hook_status: HookStatus) {
        if let Ok(listener) = PipeListenerOptions::new()
            .path(protocol::PIPE_NAME)
            .mode(PipeMode::Bytes)
            .accept_remote(false)
            .create_tokio_send_only()
        {
            loop {
                match listener.accept().await {
                    Ok(stream) => {
                        let rx = self.tx.subscribe();
                        tokio::spawn(async move {
                            let writer = FramedWrite::new(stream, LengthDelimitedCodec::new());
                            let _ = handle_client(writer, rx, hook_status).await;
                        });
                    }
                    Err(error) => warn!("Error accepting client: {error:?}"),
                }
            }
        }
    }
}
```

Import `protocol::{HookStatus, Message}` and replace the hook setup block at the bottom of `setup` with:

```rust
let hook_status = match hooks::setup_hooks(tx) {
    Ok(()) => {
        info!("Hooks initialized");
        HookStatus::Ready
    }
    Err(error) => {
        warn!("Required damage hook unavailable: {error:?}");
        HookStatus::Unsupported
    }
};

server.run(hook_status).await;
```

Do not broadcast the initial status: a Tokio broadcast channel has no backlog, so a client connecting later could miss it.

- [ ] **Step 5: Emit typed connection states and reset after pipe EOF**

Add near the state declarations in `src-tauri/src/main.rs`:

```rust
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ConnectionState {
    Searching,
    Connected,
    Disconnected,
    Unsupported,
}

fn emit_connection_state(app: &AppHandle, state: ConnectionState) {
    let _ = app.emit_all("connection-state", state);
}
```

Use `Searching` before the process loop and `Unsupported` when DLL injection returns an error. Preserve Awa's attempt to connect to an already-loaded hook after injection failure; do not `continue` before `connect_and_run_parser(app)`. Do not emit `Connected` merely because the pipe opened. Add `let mut connection_ready = false;` beside the framed reader and replace the decoded-message dispatch with:

```rust
if !matches!(&msg, protocol::Message::HookStatus(_)) && !connection_ready {
    connection_ready = true;
    emit_connection_state(&app, ConnectionState::Connected);
}

match msg {
    protocol::Message::DamageEvent(event) => state.on_damage_event(event),
    protocol::Message::OnAreaEnter(event) => state.on_area_enter_event(event),
    protocol::Message::PlayerLoadEvent(event) => state.on_player_load_event(event),
    protocol::Message::PlayerIdentityEvent(event) => state.on_player_identity_event(event),
    protocol::Message::OnQuestComplete(event) => state.on_quest_complete_event(event),
    protocol::Message::OnUpdateSBA(event) => state.on_sba_update(event),
    protocol::Message::OnAttemptSBA(event) => state.on_sba_attempt(event),
    protocol::Message::OnPerformSBA(event) => state.on_sba_perform(event),
    protocol::Message::OnContinueSBAChain(event) => state.on_continue_sba_chain(event),
    protocol::Message::OnDeathEvent(event) => state.on_death_event(event),
    protocol::Message::OnBattleEnd => state.on_battle_end_event(),
    protocol::Message::HookStatus(HookStatus::Ready) => {
        connection_ready = true;
        emit_connection_state(&app, ConnectionState::Connected);
    }
    protocol::Message::HookStatus(HookStatus::Unsupported) => {
        connection_ready = false;
        emit_connection_state(&app, ConnectionState::Unsupported);
    }
}
```

The pre-match fallback marks an older already-loaded 1.8.6 DLL connected on its first valid gameplay message even though that DLL cannot send `HookStatus`.

Import `protocol::HookStatus`. After the inner reader loop breaks, call:

```rust
state.on_connection_lost();
emit_connection_state(&app, ConnectionState::Disconnected);
let _ = app.emit_all("error-alert", "Game connection closed");
```

Replace the ignored injection result with:

```rust
match syringe.inject(dll_path) {
    Ok(_) => {
        let _ = app.emit_all("success-alert", "Found game.");
    }
    Err(error) => {
        warn!("Could not inject Hook {dll_path:?}: {error:?}");
        emit_connection_state(&app, ConnectionState::Unsupported);
        let _ = app.emit_all(
            "error-alert",
            "Hook injection failed; trying the existing game connection.",
        );
    }
}
```

Leave the existing `connect_and_run_parser(app); break;` immediately after this `match`.

- [ ] **Step 6: Keep identity optional but require the reward boundary**

Keep the existing player identity block non-fatal:

```rust
match OnLoadPlayerIdentityHook::new(tx.clone()).setup(&process) {
    Ok(()) => info!("Player identity hook enabled"),
    Err(error) => warn!("Player identity hook unavailable; using character types only: {error}"),
}
```

Keep `OnProcessDamageHook::setup(&process)?` as required. Replace the non-fatal battle-end block with a required setup because the compact meter cannot honor the approved reward-screen lifetime without it:

```rust
OnBattleEndHook::new(tx).setup(&process)?;
info!("Game 2.0.2 result reward hook enabled");
```

Now `HookStatus::Ready` means both damage collection and the exact reward boundary are available; identity and detailed-damage hooks remain optional.

- [ ] **Step 7: Verify and commit resilience behavior**

Run:

```powershell
cargo test -p gbfr-logs parser::v1::tests -- --nocapture
cargo test -p protocol hook_status_round_trips -- --nocapture
cargo test -p hook hooks::player::tests -- --nocapture
cargo test --workspace --all-targets --locked
git add protocol/src/lib.rs src-hook/src/lib.rs src-hook/src/hooks/mod.rs src-tauri/src/parser/v1/mod.rs src-tauri/src/main.rs
git commit -m "fix: isolate meter from optional hook failures"
```

Expected: all Rust tests pass, a failed required hook reports `unsupported`, and connection loss leaves no stale meter data.

### Task 4: Build the Compact Meter View Model

**Files:**
- Create: `src/components/compact-meter/compactMeterModel.ts`
- Create: `src/components/compact-meter/compactMeterModel.test.ts`

**Interfaces:**
- Consumes: `EncounterState`, `CharacterType`.
- Produces: `CompactMeterRow` and `buildCompactMeterRows(encounterState): CompactMeterRow[]`.

- [ ] **Step 1: Write failing model tests**

Create `compactMeterModel.test.ts` with a minimal `EncounterState` fixture and these assertions:

```ts
import { describe, expect, it } from "vitest";
import { EncounterState, PlayerState } from "@/types";
import { buildCompactMeterRows } from "./compactMeterModel";

const player = (index: number, totalDamage: number, dps: number): PlayerState => ({
  index,
  characterType: `Pl${index.toString().padStart(4, "0")}`,
  totalDamage,
  dps,
  sba: 0,
  totalStunValue: 0,
  stunPerSecond: 0,
  lastDamageTime: 0,
  skillBreakdown: [],
});

const encounter = (players: PlayerState[]): EncounterState => ({
  totalDamage: players.reduce((sum, item) => sum + item.totalDamage, 0),
  dps: players.reduce((sum, item) => sum + item.dps, 0),
  startTime: 1_000,
  endTime: 5_000,
  party: Object.fromEntries(players.map((item) => [item.index, item])),
  targets: {},
  status: "InProgress",
});

describe("buildCompactMeterRows", () => {
  it("sorts descending, limits four rows, and makes the leader 100 percent", () => {
    const rows = buildCompactMeterRows(
      encounter([
        player(1, 100, 25),
        player(2, 400, 100),
        player(3, 200, 50),
        player(4, 300, 75),
        player(5, 50, 12.5),
      ]),
    );
    expect(rows.map((row) => row.actorIndex)).toEqual([2, 4, 3, 1]);
    expect(rows.map((row) => row.barPercent)).toEqual([100, 75, 50, 25]);
  });

  it("returns finite zero-width bars when every total is zero", () => {
    const rows = buildCompactMeterRows(encounter([player(1, 0, 0), player(2, 0, 0)]));
    expect(rows.every((row) => row.barPercent === 0)).toBe(true);
    expect(rows.every((row) => Number.isFinite(row.barPercent))).toBe(true);
  });
});
```

- [ ] **Step 2: Run the model test and confirm RED**

Run:

```powershell
npm test -- --run src/components/compact-meter/compactMeterModel.test.ts
```

Expected: module-not-found failure.

- [ ] **Step 3: Implement the pure row model**

Create `compactMeterModel.ts`:

```ts
import { CharacterType, EncounterState } from "@/types";

export type CompactMeterRow = {
  actorIndex: number;
  characterType: CharacterType;
  totalDamage: number;
  dps: number;
  barPercent: number;
};

export const buildCompactMeterRows = (encounter: EncounterState): CompactMeterRow[] => {
  const players = Object.values(encounter.party)
    .sort((left, right) => right.totalDamage - left.totalDamage || left.index - right.index)
    .slice(0, 4);
  const highestDamage = Math.max(0, ...players.map((player) => player.totalDamage));

  return players.map((player) => ({
    actorIndex: player.index,
    characterType: player.characterType,
    totalDamage: player.totalDamage,
    dps: Math.round(player.dps),
    barPercent: highestDamage === 0 ? 0 : (player.totalDamage / highestDamage) * 100,
  }));
};
```

- [ ] **Step 4: Verify and commit the view model**

Run:

```powershell
npm test -- --run src/components/compact-meter/compactMeterModel.test.ts
npm run tsc
git add src/components/compact-meter
git commit -m "feat: add compact damage row model"
```

Expected: model tests and TypeScript compile pass.

### Task 5: Render and Throttle the Korean Compact Overlay

**Files:**
- Create: `src/components/compact-meter/CompactDamageMeter.tsx`
- Create: `src/components/compact-meter/CompactDamageMeter.test.tsx`
- Create: `src/components/compact-meter/CompactDamageMeter.css`
- Create: `src/pages/useCompactMeter.ts`
- Create: `src/pages/useCompactMeter.test.tsx`
- Modify: `src/pages/Meter.tsx`
- Modify: `src/types.ts`

**Interfaces:**
- Consumes: `CompactMeterRow[]`, Tauri events `encounter-update`, `on-area-enter`, `connection-state`, and stored transparency.
- Produces: `useCompactMeter(): { encounterState; rows; connectionState; transparency }` and `CompactDamageMeter({ rows, transparency })`.

- [ ] **Step 1: Write failing component tests**

Mock `react-i18next` and render two rows in `CompactDamageMeter.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { CompactDamageMeter } from "./CompactDamageMeter";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "ui.compact-meter.title": "파티 데미지",
        "characters:Pl1400": "나루메아",
        "ui.compact-meter.unknown-character": "알 수 없는 캐릭터",
      })[key] ?? key,
  }),
}));

it("shows Korean character names, full totals, DPS, and relative bars", () => {
  const { container } = render(
    <CompactDamageMeter
      transparency={0.72}
      rows={[
        { actorIndex: 1, characterType: "Pl1400", totalDamage: 1_234_567, dps: 12_345, barPercent: 100 },
        { actorIndex: 2, characterType: { Unknown: 1 }, totalDamage: 617_284, dps: 6_172, barPercent: 50 },
      ]}
    />,
  );

  expect(screen.getByText("파티 데미지")).toBeTruthy();
  expect(screen.getByText("나루메아")).toBeTruthy();
  expect(screen.getByText("알 수 없는 캐릭터")).toBeTruthy();
  expect(screen.getByText("1,234,567")).toBeTruthy();
  expect(screen.getByText("12,345 DPS")).toBeTruthy();
  expect((container.querySelectorAll<HTMLElement>(".compact-meter__bar")[1]).style.width).toBe("50%");
});
```

- [ ] **Step 2: Write a failing 250ms hook test**

Create `useCompactMeter.test.tsx` with controlled Tauri listeners:

```tsx
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { EncounterState } from "@/types";
import useCompactMeter from "./useCompactMeter";

const mocks = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, callback: (event: { payload: unknown }) => void) => {
    mocks.listeners.set(name, callback);
    return vi.fn();
  }),
}));

vi.mock("@/stores/useMeterSettingsStore", () => ({
  useMeterSettingsStore: (selector: (state: { transparency: number }) => unknown) =>
    selector({ transparency: 0.72 }),
}));

const activeEncounter: EncounterState = {
  totalDamage: 1_000,
  dps: 250,
  startTime: 1_000,
  endTime: 5_000,
  status: "InProgress",
  targets: {},
  party: {
    7: {
      index: 7,
      characterType: "Pl1400",
      totalDamage: 1_000,
      dps: 250,
      sba: 0,
      totalStunValue: 0,
      stunPerSecond: 0,
      lastDamageTime: 5_000,
      skillBreakdown: [],
    },
  },
};

beforeEach(() => {
  vi.useFakeTimers();
  mocks.listeners.clear();
});

afterEach(() => vi.useRealTimers());

it("publishes only the newest encounter every 250ms and clears on disconnect", async () => {
  const { result } = renderHook(() => useCompactMeter());
  await act(async () => Promise.resolve());

  act(() => mocks.listeners.get("connection-state")?.({ payload: "connected" }));
  act(() => mocks.listeners.get("encounter-update")?.({ payload: activeEncounter }));
  expect(result.current.encounterState.totalDamage).toBe(0);

  act(() => vi.advanceTimersByTime(250));
  expect(result.current.encounterState.totalDamage).toBe(1_000);
  expect(result.current.rows).toHaveLength(1);

  act(() => mocks.listeners.get("connection-state")?.({ payload: "disconnected" }));
  act(() => vi.advanceTimersByTime(250));
  expect(result.current.rows).toEqual([]);
});
```

- [ ] **Step 3: Run both tests and confirm RED**

Run:

```powershell
npm test -- --run src/components/compact-meter/CompactDamageMeter.test.tsx src/pages/useCompactMeter.test.tsx
```

Expected: both modules are missing.

- [ ] **Step 4: Implement the compact component and isolated CSS**

Create `CompactDamageMeter.tsx`:

```tsx
import { CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { CompactMeterRow } from "./compactMeterModel";
import "./CompactDamageMeter.css";

const format = new Intl.NumberFormat("ko-KR", { maximumFractionDigits: 0 });

export type CompactDamageMeterProps = {
  rows: CompactMeterRow[];
  transparency: number;
};

export const CompactDamageMeter = ({ rows, transparency }: CompactDamageMeterProps) => {
  const { t } = useTranslation();
  const unknown = t("ui.compact-meter.unknown-character");
  const characterName = (row: CompactMeterRow) => {
    if (typeof row.characterType !== "string") return unknown;
    const translated = t(`characters:${row.characterType}`, { defaultValue: "" });
    return translated.trim() ? translated : unknown;
  };

  return (
    <section
      className="compact-meter"
      style={{ "--meter-opacity": transparency } as CSSProperties}
      aria-label={t("ui.compact-meter.title")}
    >
      <header className="compact-meter__header" data-tauri-drag-region>
        {t("ui.compact-meter.title")}
      </header>
      {rows.slice(0, 4).map((row) => (
        <div className="compact-meter__row" key={row.actorIndex}>
          <div className="compact-meter__bar" style={{ width: `${row.barPercent}%` }} aria-hidden="true" />
          <div className="compact-meter__content">
            <span className="compact-meter__name">{characterName(row)}</span>
            <span>{format.format(row.totalDamage)}</span>
            <span>{format.format(row.dps)} DPS</span>
          </div>
        </div>
      ))}
    </section>
  );
};
```

Use these exact layout rules in `CompactDamageMeter.css`:

```css
.compact-meter {
  width: 100%;
  color: #f7fbff;
  background: rgba(10, 20, 34, var(--meter-opacity));
  border: 1px solid rgba(180, 220, 255, 0.35);
  border-radius: 5px;
  overflow: hidden;
  text-shadow: 0 1px 2px #000;
}
.compact-meter__header { height: 25px; padding: 4px 8px; font-size: 12px; font-weight: 700; }
.compact-meter__row { position: relative; height: 29px; background: rgba(255, 255, 255, 0.06); }
.compact-meter__bar {
  position: absolute;
  inset: 0 auto 0 0;
  height: 100%;
  background: linear-gradient(90deg, rgba(47, 118, 183, 0.84), rgba(105, 177, 231, 0.48));
  transition: width 150ms linear;
}
.compact-meter__content {
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-columns: minmax(70px, 1fr) auto auto;
  align-items: center;
  height: 100%;
  gap: 7px;
  padding: 0 7px;
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}
.compact-meter__name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 700; }
```

- [ ] **Step 5: Implement `useCompactMeter` with a pending-state ref**

Add this shared type to `src/types.ts`:

```ts
export type ConnectionState = "searching" | "connected" | "disconnected" | "unsupported";
```

Create `useCompactMeter.ts`:

```ts
import { useMeterSettingsStore } from "@/stores/useMeterSettingsStore";
import { ConnectionState, EncounterState } from "@/types";
import { buildCompactMeterRows } from "@/components/compact-meter/compactMeterModel";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

export const UPDATE_INTERVAL_MS = 250;
export const DEFAULT_ENCOUNTER_STATE: EncounterState = {
  totalDamage: 0,
  dps: 0,
  startTime: 0,
  endTime: 0,
  party: {},
  targets: {},
  status: "Waiting",
};

export default function useCompactMeter() {
  const [encounterState, setEncounterState] = useState(DEFAULT_ENCOUNTER_STATE);
  const [connectionState, setConnectionState] = useState<ConnectionState>("searching");
  const pendingEncounter = useRef(DEFAULT_ENCOUNTER_STATE);
  const transparency = useMeterSettingsStore((state) => state.transparency);

  useEffect(() => {
    const subscriptions = [
      listen<EncounterState>("encounter-update", (event) => { pendingEncounter.current = event.payload; }),
      listen<EncounterState>("on-area-enter", () => { pendingEncounter.current = DEFAULT_ENCOUNTER_STATE; }),
      listen<ConnectionState>("connection-state", (event) => {
        setConnectionState(event.payload);
        if (event.payload !== "connected") pendingEncounter.current = DEFAULT_ENCOUNTER_STATE;
      }),
    ];
    const interval = window.setInterval(() => setEncounterState(pendingEncounter.current), UPDATE_INTERVAL_MS);

    return () => {
      window.clearInterval(interval);
      void Promise.all(subscriptions).then((unlisten) => unlisten.forEach((dispose) => dispose()));
    };
  }, []);

  const visible = connectionState === "connected" && encounterState.status === "InProgress";
  const rows = visible ? buildCompactMeterRows(encounterState) : [];
  return { encounterState, connectionState, rows, transparency };
}
```

- [ ] **Step 6: Replace the legacy live table only at the meter route**

Replace `src/pages/Meter.tsx` with a thin composition:

```tsx
import "@/i18n";
import { CompactDamageMeter } from "@/components/compact-meter/CompactDamageMeter";
import useCompactMeter from "./useCompactMeter";

export const Meter = () => {
  const { rows, transparency } = useCompactMeter();
  if (rows.length === 0) return null;
  return <CompactDamageMeter rows={rows} transparency={transparency} />;
};
```

Do not delete the legacy `Table`, logs pages or historical parser components; they remain available outside the live overlay.

- [ ] **Step 7: Verify and commit the compact overlay**

Run:

```powershell
npm test -- --run src/components/compact-meter/CompactDamageMeter.test.tsx src/pages/useCompactMeter.test.tsx
npm test -- --run
npm run tsc
npm run lint
npm run format-check
git add src/components/compact-meter src/pages/Meter.tsx src/pages/useCompactMeter.ts src/pages/useCompactMeter.test.tsx src/types.ts
git commit -m "feat: render compact Korean damage overlay"
```

Expected: component/hook tests pass and the live route contains no expandable skills or extra columns.

### Task 6: Set Korean Defaults, Window Geometry, and Settings Mode

**Files:**
- Modify: `src/i18n.ts`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`
- Modify: `src/stores/useMeterSettingsStore.ts`
- Modify: `src/pages/useCompactMeter.test.tsx`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/tauri.conf.json`
- Test: `src/i18n.test.ts`, inline Rust geometry tests

**Interfaces:**
- Consumes: `ConnectionState`, persisted Zustand key `meter-settings`, Tauri main window.
- Produces: `reset_meter_geometry`, `MeterGeometry`, Korean-first i18n and click-through enabled by default.

- [ ] **Step 1: Add failing locale and geometry tests**

Create `src/i18n.test.ts`:

```ts
import { beforeEach, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/fs", () => ({ readTextFile: vi.fn(async () => "{}") }));
vi.mock("@tauri-apps/api/path", () => ({ resolveResource: vi.fn(async (path: string) => path) }));

beforeEach(() => {
  localStorage.clear();
  vi.resetModules();
});

it("uses Korean for a fresh profile", async () => {
  const { default: i18n, SUPPORTED_LANGUAGES } = await import("./i18n");
  await i18n.changeLanguage(i18n.language);
  expect(i18n.language).toBe("ko");
  expect(SUPPORTED_LANGUAGES.ko).toBe("한국어");
});
```

In `src-tauri/src/main.rs` tests add:

```rust
#[test]
fn meter_geometry_matches_the_1080p_design() {
    let geometry = meter_geometry(1920.0, 1080.0);
    assert_eq!(geometry, MeterGeometry { x: 45.0, y: 470.0, width: 330.0, height: 145.0 });
}

#[test]
fn meter_geometry_scales_but_does_not_exceed_one_and_a_half() {
    let geometry = meter_geometry(3840.0, 2160.0);
    assert_eq!(geometry.width, 495.0);
    assert_eq!(geometry.height, 217.5);
}
```

- [ ] **Step 2: Run tests and confirm RED**

Run:

```powershell
npm test -- --run src/i18n.test.ts
cargo test -p gbfr-logs meter_geometry -- --nocapture
```

Expected: locale remains detector-controlled and Rust geometry symbols are undefined.

- [ ] **Step 3: Configure Korean-first i18n and exact compact strings**

Change `SUPPORTED_LANGUAGES` from `"ko-KR"` to `ko`, and initialize with:

```ts
lng: localStorage.getItem("i18nextLng") ?? "ko",
fallbackLng: { default: ["ko", "en"], "zh-TW": ["zh-CN", "en"] },
```

Add under the top-level `ui` object in both language files:

```json
"compact-meter": {
  "title": "파티 데미지",
  "unknown-character": "알 수 없는 캐릭터"
},
"connection": {
  "searching": "게임 연결 대기 중",
  "connected": "게임 연결됨",
  "disconnected": "게임 연결 끊김",
  "unsupported": "게임 2.0.2 훅을 찾을 수 없습니다"
}
```

Add these exact values to `en/ui.json`:

```json
"compact-meter": {
  "title": "Party Damage",
  "unknown-character": "Unknown Character"
},
"connection": {
  "searching": "Waiting for the game",
  "connected": "Game connected",
  "disconnected": "Game disconnected",
  "unsupported": "The game 2.0.2 hook was not found"
}
```

- [ ] **Step 4: Add proportional geometry and a reset command**

Add this pure model to `src-tauri/src/main.rs`:

```rust
#[derive(Debug, PartialEq)]
struct MeterGeometry { x: f64, y: f64, width: f64, height: f64 }

fn meter_geometry(screen_width: f64, screen_height: f64) -> MeterGeometry {
    let scale = (screen_width / 1920.0).min(screen_height / 1080.0).clamp(0.75, 1.5);
    MeterGeometry { x: 45.0 * scale, y: 470.0 * scale, width: 330.0 * scale, height: 145.0 * scale }
}

#[tauri::command]
fn reset_meter_geometry(window: tauri::Window) -> Result<(), String> {
    let monitor = window.current_monitor().map_err(|error| error.to_string())?
        .ok_or_else(|| "No monitor found".to_string())?;
    let logical = monitor.size().to_logical::<f64>(monitor.scale_factor());
    let geometry = meter_geometry(logical.width, logical.height);
    window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(geometry.x, geometry.y)))
        .map_err(|error| error.to_string())?;
    window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(geometry.width, geometry.height)))
        .map_err(|error| error.to_string())
}
```

Register the command, call it from the tray `reset_windows` branch, and set `ClickThrough(AtomicBool::new(true))`. In `.setup`, call `main_window.set_ignore_cursor_events(true)`.

- [ ] **Step 5: Apply geometry only once per persisted settings profile**

Add `geometry_initialized: boolean` with default `false` to `MeterSettings` and select `set` plus this field in `useCompactMeter`. Add this effect:

```ts
useEffect(() => {
  if (geometry_initialized) return;
  void invoke("reset_meter_geometry").then(() => setMeterSettings({ geometry_initialized: true }));
}, [geometry_initialized, setMeterSettings]);
```

Import `invoke` from `@tauri-apps/api`. Existing users with no field receive the default and are migrated once. The compact header remains a drag region; users toggle click-through from the tray to enter/leave settings mode.

Update the store mock in `useCompactMeter.test.tsx` so this later interface remains type-consistent:

```ts
vi.mock("@tauri-apps/api", () => ({ invoke: vi.fn(async () => undefined) }));
vi.mock("@/stores/useMeterSettingsStore", () => ({
  useMeterSettingsStore: (selector: (state: object) => unknown) =>
    selector({ transparency: 0.72, geometry_initialized: true, set: vi.fn() }),
}));
```

Set the main window in `src-tauri/tauri.conf.json` to:

```json
"width": 330,
"height": 145,
"minWidth": 280,
"minHeight": 120,
"resizable": true,
"decorations": false,
"transparent": true,
"alwaysOnTop": true
```

- [ ] **Step 6: Show connection status in the existing settings page**

Add state and this subscription in `src/pages/useSettings.ts`:

```ts
const [connectionState, setConnectionState] = useState<ConnectionState>("searching");
useEffect(() => {
  const subscription = listen<ConnectionState>("connection-state", (event) => setConnectionState(event.payload));
  return () => { void subscription.then((dispose) => dispose()); };
}, []);
```

Import `ConnectionState`, `listen`, `useEffect` and `useState`, return `connectionState`, then render this near the top of `Settings.tsx`:

```tsx
<Text size="sm" c="dimmed">{t(`ui.connection.${connectionState}`)}</Text>
```

This message stays out of the combat overlay.

- [ ] **Step 7: Verify and commit locale/window behavior**

Run:

```powershell
npm test -- --run src/i18n.test.ts src/pages/useCompactMeter.test.tsx
cargo test -p gbfr-logs meter_geometry -- --nocapture
npm test -- --run
npm run tsc
npm run lint
cargo test --workspace --all-targets --locked
git add src/i18n.ts src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json src/stores/useMeterSettingsStore.ts src/pages/useCompactMeter.ts src/pages/useCompactMeter.test.tsx src/pages/useSettings.ts src/pages/Settings.tsx src-tauri/src/main.rs src-tauri/tauri.conf.json
git commit -m "feat: default the compact meter to Korean"
```

Expected: Korean is the first-run language, 1080p reset produces 330x145 at x45/y470, and normal mode is click-through.

### Task 7: Rebrand, Document, Package, and Smoke-Test the MSI

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-hook/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `README.md`
- Preserve: `LICENSE`
- Create: `docs/testing/game-2.0.2-smoke-test.md`

**Interfaces:**
- Consumes: completed hook, parser and compact UI.
- Produces: version `0.1.0` MSI, SHA-256 hashes, Korean install/risk documentation and a repeatable game test checklist.

- [ ] **Step 1: Add a failing manifest consistency check**

Create a PowerShell verification command in the smoke-test document and run it before editing:

```powershell
$npmVersion = (Get-Content -Raw package.json | ConvertFrom-Json).version
$tauriVersion = (Get-Content -Raw src-tauri/tauri.conf.json | ConvertFrom-Json).package.version
$cargoVersion = (Select-String '^version = "(.+)"' src-tauri/Cargo.toml).Matches[0].Groups[1].Value
$versions = @($npmVersion, $tauriVersion, $cargoVersion) | Select-Object -Unique
if ($versions.Count -ne 1 -or $versions[0] -ne '0.1.0') { throw 'Version mismatch or unexpected release version' }
```

Expected: fails before rebranding because the imported manifests describe Awa Edition rather than version `0.1.0`.

- [ ] **Step 2: Apply product identity without removing upstream credit**

Set:

- npm name: `gbfr-korean-damage-meter`
- all application versions: `0.1.0`
- Tauri product name: `GBFR Korean Damage Meter`
- Tauri identifier: `com.azyu.gbfr-korean-damage-meter`
- hook CompanyName: `GBFR Korean Damage Meter contributors`
- hook LegalCopyright: `Copyright (C) 2024 False Spring; 2026 onelittlechildawa; 2026 GBFR Korean Damage Meter contributors`
- updater: `active: false`

Use `npm version 0.1.0 --no-git-tag-version --allow-same-version` so both npm manifest files agree. Keep the complete upstream MIT LICENSE unchanged.

- [ ] **Step 3: Replace README with Korean product instructions and risk disclosure**

Use this section structure and content:

````markdown
# GBFR Korean Damage Meter

Granblue Fantasy: Relink Endless Ragnarok 2.0.2용 Windows x64 파티 데미지 미터입니다. 캐릭터별 누적 피해, 상대 바와 DPS를 한국어 소형 오버레이로 표시합니다.

## 설치와 실행

1. 릴리스의 MSI를 설치합니다.
2. 게임을 먼저 실행합니다.
3. GBFR Korean Damage Meter를 실행합니다.
4. 트레이 메뉴의 클릭 통과를 끄면 창을 이동·조절할 수 있고, 다시 켜면 입력이 게임으로 전달됩니다.

제거는 Windows의 설치된 앱에서 수행합니다. 사용자 설정과 로그는 `%AppData%` 아래 애플리케이션 데이터 폴더에 남을 수 있습니다.

## 주의

이 도구는 DLL 주입, 게임 메모리 읽기와 런타임 코드 후킹을 사용합니다. Cygames가 공식 허용하거나 화이트리스트한 도구가 아니며 온라인 사용과 계정 제재 위험이 없다고 보증하지 않습니다. 먼저 오프라인 또는 비공개 환경에서 사용하십시오.

## 소스 빌드

Node.js 20, Visual Studio 2022 C++ Build Tools, Windows SDK, WebView2, rustup과 `rust-toolchain.toml`의 toolchain이 필요합니다.

```powershell
npm ci
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
npm test -- --run
npm run tauri build -- --bundles msi
```

## 크레딧과 라이선스

MIT 라이선스의 `false-spring/gbfr-logs`와 `onelittlechildawa/gbfr-logs` Awa Edition 1.8.6을 기반으로 합니다. 전체 저작권 및 허가문은 `LICENSE`를 참조하십시오.
````

- [ ] **Step 4: Write the manual game smoke-test checklist**

Create `docs/testing/game-2.0.2-smoke-test.md` with this table:

```markdown
# Game 2.0.2 Smoke Test

환경 기록: 게임 빌드 / 해상도 / 파티 구성 / MSI SHA-256 / hook SHA-256

| 완료 | 시나리오 | 기대 결과 | 실제 결과 |
|---|---|---|---|
| [ ] | 훈련장 단일 타격 | 화면 피해와 누적 피해가 일치한다 | |
| [ ] | 서로 다른 4캐릭터 | 네 행이 캐릭터별로 분리된다 | |
| [ ] | 온라인 동일 캐릭터 2명 | 두 actor가 별도 행으로 유지된다 | |
| [ ] | 추가 피해, DoT, SBA, 링크 어택 | 중복 또는 누락 없이 합산된다 | |
| [ ] | 적 2마리 퀘스트 | 두 대상 피해가 한 전투 총합에 포함된다 | |
| [ ] | 마지막 적 처치 후 결과 전환 | reward UI 직전까지 유지되고 진입 전에 사라진다 | |
| [ ] | 전투 중 게임 종료 | 미터가 숨고 게임·앱이 충돌하지 않는다 | |
| [ ] | 1920x1080 창 초기화 | 330x145, x45/y470이며 파티·적 HUD를 가리지 않는다 | |
| [ ] | 클릭 통과 전환 | 설정 모드에서 이동·크기 변경, 일반 모드에서 게임 입력 통과 | |
| [ ] | 재도전 | 새 전투가 누적 피해 0에서 시작한다 | |
```

- [ ] **Step 5: Run the complete automated gate**

Run:

```powershell
npm ci
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked --verbose
npm run tauri build -- --bundles msi
```

Expected: all commands exit 0 and an MSI appears under `target/release/bundle/msi/`.

- [ ] **Step 6: Prove the packaged hook is current and record hashes**

Run:

```powershell
$builtHook = Resolve-Path 'target\release\hook.dll'
$stagedHook = Resolve-Path 'src-tauri\hook.dll'
$msi = Get-ChildItem 'target\release\bundle\msi\*.msi' | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
$builtHash = (Get-FileHash -Algorithm SHA256 $builtHook).Hash
$stagedHash = (Get-FileHash -Algorithm SHA256 $stagedHook).Hash
if ($builtHash -ne $stagedHash) { throw 'Staged hook does not match release hook' }
Get-FileHash -Algorithm SHA256 $msi.FullName
```

Expected: the two hook hashes match and the MSI hash is printed for the release notes.

- [ ] **Step 7: Perform the game smoke test before claiming compatibility**

Run every item in `docs/testing/game-2.0.2-smoke-test.md` first in an offline/private environment. Record game build 2.0.2, display resolution, party composition, pass/fail, and any hook log path. Do not call the MSI 2.0.2-compatible until all required items pass.

- [ ] **Step 8: Commit the release-ready state**

Run:

```powershell
git add package.json package-lock.json src-tauri/Cargo.toml src-hook/Cargo.toml src-tauri/tauri.conf.json README.md LICENSE docs/testing/game-2.0.2-smoke-test.md
git commit -m "chore: prepare Korean damage meter 0.1.0"
git status --short
```

Expected: commit succeeds and the working tree is clean.

---

## Final Verification Matrix

| Requirement | Automated evidence | Manual evidence |
|---|---|---|
| First hit starts encounter | Rust parser test | Training-room first hit |
| Party actor separation | Existing + resilience Rust tests | 4-player and same-character party |
| Highest-damage relative bar | `compactMeterModel.test.ts` | Visual bar comparison |
| Korean names/default | i18n + component tests | Fresh-profile launch |
| 250ms / 150ms behavior | hook fake-timer test / CSS assertion | No visible flicker |
| Hold until reward boundary | parser + hook-order tests | Quest result transition |
| Pipe/hook failure isolation | parser reset tests | Close game during encounter |
| 330x145 left HUD placement | Rust geometry test | 1920x1080 screenshot |
| MSI contains current hook | SHA-256 equality gate | Install and run packaged MSI |
