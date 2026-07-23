// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

use anyhow::Context;
use db::logs::LogEntry;
use dll_syringe::{
    process::{OwnedProcess, Process},
    Syringe,
};
use game_search::{
    pipe_wait_decision, GameSearchState, PipeWaitDecision, ProcessSearchBudget,
    ProcessSearchDecision, PIPE_CONNECT_INTERVAL, PROCESS_SEARCH_INTERVAL,
};
use interprocess::os::windows::named_pipe::tokio::RecvPipeStream;
use log::{info, warn, LevelFilter};
use parser::{
    constants::{CharacterType, EnemyType},
    v1::{self, PlayerData},
};
use protocol::{HookStatus, Message};
use rusqlite::params_from_iter;
use serde::{Deserialize, Serialize};
use tauri::{
    api::dialog::blocking::FileDialogBuilder, AppHandle, CustomMenuItem, LogicalPosition,
    LogicalSize, Manager, Position, Size, State, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};
use tauri_plugin_log::LogTarget;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};
use tokio_stream::StreamExt;
use tokio_util::codec::FramedRead;

mod db;
mod equipment;
mod equipment_probe;
mod game_search;
mod parser;
mod repeat_quest;
mod update_install;

struct ClickThrough(AtomicBool);
struct CloseToTray(AtomicBool);
struct DebugMode(AtomicBool);
struct ConnectionStatus(Mutex<ConnectionState>);
struct EquipmentStatus(Mutex<equipment::EquipmentState>);

const DEFAULT_CLICK_THROUGH: bool = false;
const DEFAULT_CLOSE_TO_TRAY: bool = true;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MeterWindowAction {
    Show,
    Hide,
}

fn meter_window_action(enabled: bool) -> MeterWindowAction {
    if enabled {
        MeterWindowAction::Show
    } else {
        MeterWindowAction::Hide
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseRequestAction {
    Hide,
    Exit,
}

fn close_request_action(window_label: &str, close_to_tray: bool) -> CloseRequestAction {
    if window_label == "logs" && !close_to_tray {
        CloseRequestAction::Exit
    } else {
        CloseRequestAction::Hide
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ConnectionState {
    Searching,
    Connected,
    Disconnected,
    Unsupported,
    NotFound,
}

fn retry_allowed(state: ConnectionState) -> bool {
    state == ConnectionState::NotFound
}

#[derive(Debug, PartialEq, Eq)]
enum HookHandshakeState {
    Awaiting,
    Ready,
    Unsupported,
    Legacy,
}

fn accept_hook_message(state: &mut HookHandshakeState, message: &Message) -> bool {
    match message {
        Message::HookStatus(HookStatus::Ready) => {
            *state = HookHandshakeState::Ready;
            false
        }
        Message::HookStatus(HookStatus::Unsupported) => {
            *state = HookHandshakeState::Unsupported;
            false
        }
        _ if *state == HookHandshakeState::Unsupported => false,
        _ if *state == HookHandshakeState::Awaiting => {
            *state = HookHandshakeState::Legacy;
            true
        }
        _ => true,
    }
}

#[derive(Debug, PartialEq)]
struct MeterGeometry {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn meter_geometry(screen_width: f64, screen_height: f64) -> MeterGeometry {
    let scale = (screen_width / 1920.0).min(screen_height / 1080.0);
    let position_scale = scale.clamp(0.75, 1.5);
    let size_scale = scale.clamp(1.0, 1.5);

    MeterGeometry {
        x: 45.0 * position_scale,
        y: 470.0 * position_scale,
        width: 330.0 * size_scale,
        height: 145.0 * size_scale,
    }
}

fn set_meter_size(window: &tauri::Window) -> anyhow::Result<()> {
    let monitor = window
        .current_monitor()?
        .context("No monitor available for the meter window")?;
    let screen = monitor.size().to_logical::<f64>(monitor.scale_factor());
    let geometry = meter_geometry(screen.width, screen.height);

    window.set_size(Size::Logical(LogicalSize {
        width: geometry.width,
        height: geometry.height,
    }))?;
    Ok(())
}

fn emit_connection_state(app: &AppHandle, state: ConnectionState) {
    *app.state::<ConnectionStatus>().0.lock().unwrap() = state;
    let _ = app.emit_all("connection-state", state);
}

#[tauri::command]
fn get_connection_state(state: State<ConnectionStatus>) -> ConnectionState {
    *state.0.lock().unwrap()
}

#[tauri::command]
fn fetch_equipment_analysis(state: State<EquipmentStatus>) -> equipment::EquipmentAnalysisResponse {
    state.0.lock().unwrap().response()
}

fn update_equipment_connection(app: &AppHandle, connected: bool) {
    let response = {
        let equipment = app.state::<EquipmentStatus>();
        let mut state = equipment.0.lock().unwrap();
        if connected {
            state.connect();
        } else {
            state.disconnect();
        }
        state.response()
    };
    let _ = app.emit_all("equipment-analysis-update", response);
}

#[tauri::command]
fn reset_meter_geometry(window: tauri::Window) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No monitor available for the meter window".to_string())?;
    let screen = monitor.size().to_logical::<f64>(monitor.scale_factor());
    let geometry = meter_geometry(screen.width, screen.height);

    window
        .set_position(Position::Logical(LogicalPosition {
            x: geometry.x,
            y: geometry.y,
        }))
        .map_err(|error| error.to_string())?;
    set_meter_size(&window).map_err(|error| error.to_string())
}

#[tauri::command]
fn set_debug_mode(app: AppHandle, state: State<DebugMode>, enabled: bool) {
    if let Some(window) = app.get_window("logs") {
        if enabled {
            window.open_devtools()
        } else {
            window.close_devtools()
        }
    }

    state.0.store(enabled, Ordering::Release);
}

#[tauri::command]
async fn delete_all_logs() -> Result<(), String> {
    let conn = db::connect_to_db().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM logs", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn export_damage_log_to_file(id: u32, options: ParseOptions) -> Result<(), String> {
    let file_path = FileDialogBuilder::new()
        .add_filter("csv", &["csv"])
        .set_file_name(&format!("{id}_damage_log.csv"))
        .set_title("Export Damage Log")
        .save_file()
        .ok_or("No file selected!")?;

    let conn = db::connect_to_db().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT data, version FROM logs WHERE id = ?")
        .map_err(|e| e.to_string())?;

    let (blob, version): (Vec<u8>, u8) = stmt
        .query_row([id], |row| Ok((row.get(0)?, row.get(1)?)))
        .context("Failed to fetch log from database")
        .map_err(|e| e.to_string())?;

    let parser = parser::deserialize_version(&blob, version).map_err(|e| e.to_string())?;

    let file = File::create(file_path).map_err(|e| e.to_string())?;

    // @TODO(false): Split formatting into a separate function.
    let mut writer = std::io::BufWriter::new(file);

    writeln!(
        writer,
        "timestamp,source_type,child_source_type,source_index,target_type,target_index,action_id,flags,damage"
    )
    .map_err(|e| e.to_string())?;

    for (event_ts, event) in parser.encounter.event_log() {
        if let Message::DamageEvent(damage_event) = event {
            let timestamp = event_ts - parser.start_time();
            let target_type = EnemyType::from_hash(damage_event.target.parent_actor_type);
            let parent_character_type =
                CharacterType::from_hash(damage_event.source.parent_actor_type);
            let child_character_type = CharacterType::from_hash(damage_event.source.actor_type);

            if options.targets.is_empty() || options.targets.contains(&target_type) {
                writeln!(
                    writer,
                    "{},{},{},{},{},{},{},{},{}",
                    timestamp,
                    parent_character_type,
                    child_character_type,
                    damage_event.source.parent_index,
                    target_type,
                    damage_event.target.parent_index,
                    damage_event.action_id,
                    damage_event.flags,
                    damage_event.damage
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    writer.flush().map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResult {
    logs: Vec<LogEntry>,
    page: u32,
    page_count: u32,
    log_count: i32,
    /// IDs of the enemies that can be filtered by.
    enemy_ids: Vec<u32>,
    /// IDs of the quests that can be filtered by.
    quest_ids: Vec<u32>,
    /// Names of the Players that can be filtered by.
    player_ids: Vec<String>,
    /// Names of the Characters that can be filtered by.
    player_types: Vec<String>,
}

#[tauri::command]
fn fetch_logs(
    page: Option<u32>,
    filter_by_enemy_id: Option<u32>,
    filter_by_quest_id: Option<u32>,
    sort_direction: Option<String>,
    sort_type: Option<String>,
    quest_completed: Option<bool>,
    filter_by_player_id: Option<String>,
    filter_by_player_character: Option<String>,
) -> Result<SearchResult, String> {
    let conn = db::connect_to_db().map_err(|e| e.to_string())?;
    let page = page.unwrap_or(1);
    let per_page = 10;
    let offset = page.saturating_sub(1) * per_page;

    let sort_type_param = sort_type
        .map(|s| match s.as_str() {
            "time" => db::logs::SortType::Time,
            "duration" => db::logs::SortType::Duration,
            "quest-elapsed-time" => db::logs::SortType::QuestElapsedTime,
            _ => db::logs::SortType::Time,
        })
        .unwrap_or(db::logs::SortType::Time);

    let sort_direction_param = sort_direction
        .map(|s| match s.as_str() {
            "asc" => db::logs::SortDirection::Ascending,
            _ => db::logs::SortDirection::Descending,
        })
        .unwrap_or(db::logs::SortDirection::Descending);

    let logs = db::logs::get_logs(
        &conn,
        filter_by_enemy_id,
        filter_by_quest_id,
        per_page,
        offset,
        &sort_type_param,
        &sort_direction_param,
        quest_completed,
        &filter_by_player_id,
        &filter_by_player_character,
    )
    .map_err(|e| e.to_string())?;

    let log_count = db::logs::get_logs_count(
        &conn,
        filter_by_enemy_id,
        filter_by_quest_id,
        quest_completed,
        &filter_by_player_id,
        &filter_by_player_character,
    )
    .map_err(|e| e.to_string())?;

    let page_count = (log_count as f64 / per_page as f64).ceil() as u32;

    let mut enemy_ids = Vec::new();
    let mut quest_ids = Vec::new();
    let mut player_ids = Vec::new();
    let mut player_types = Vec::new();

    let mut query = conn
        .prepare("SELECT primary_target, quest_id, p1_name, p1_type, p2_name, p2_type, p3_name, p3_type, p4_name, p4_type from logs")
        .map_err(|e| e.to_string())?;

    let rows = query
        .query_map([], |row| {
            Ok((
                row.get::<usize, Option<u32>>(0)?,    // primary_target
                row.get::<usize, Option<u32>>(1)?,    // quest_id
                row.get::<usize, Option<String>>(2)?, // p1_name
                row.get::<usize, Option<String>>(3)?, // p1_type
                row.get::<usize, Option<String>>(4)?, // p2_name
                row.get::<usize, Option<String>>(5)?, // p2_type
                row.get::<usize, Option<String>>(6)?, // p3_name
                row.get::<usize, Option<String>>(7)?, // p3_type
                row.get::<usize, Option<String>>(8)?, // p4_name
                row.get::<usize, Option<String>>(9)?, // p4_type
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (
            primary_target,
            quest_id,
            p1_name,
            p1_type,
            p2_name,
            p2_type,
            p3_name,
            p3_type,
            p4_name,
            p4_type,
        ) = row.map_err(|e| e.to_string())?;

        if let Some(primary_target) = primary_target {
            if !enemy_ids.contains(&primary_target) {
                enemy_ids.push(primary_target);
            }
        }

        if let Some(quest_id) = quest_id {
            if !quest_ids.contains(&quest_id) {
                quest_ids.push(quest_id);
            }
        }

        for p_name in [p1_name, p2_name, p3_name, p4_name] {
            if let Some(p_name) = p_name {
                if !player_ids.contains(&p_name) {
                    player_ids.push(p_name)
                }
            }
        }

        for p_type in [p1_type, p2_type, p3_type, p4_type] {
            if let Some(p_type) = p_type {
                if !player_types.contains(&p_type) {
                    player_types.push(p_type)
                }
            }
        }
    }

    Ok(SearchResult {
        logs,
        page,
        page_count,
        log_count,
        enemy_ids,
        quest_ids,
        player_ids,
        player_types,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EncounterStateResponse {
    encounter_state: v1::DerivedEncounterState,
    players: [Option<PlayerData>; 4],
    quest_id: Option<u32>,
    quest_timer: Option<u32>,
    quest_completed: bool,
    targets: Vec<EnemyType>,
    dps_chart: HashMap<u32, Vec<i32>>,
    sba_chart: HashMap<u32, Vec<f32>>,
    sba_events: Vec<(i64, protocol::Message)>,
    death_events: Vec<(i64, protocol::Message)>,
    chart_len: usize,
    sba_chart_len: usize,
}

#[derive(Debug, Deserialize)]
struct ParseOptions {
    targets: Vec<EnemyType>,
}

#[tauri::command]
fn fetch_encounter_state(id: u64, options: ParseOptions) -> Result<EncounterStateResponse, String> {
    let conn = db::connect_to_db().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT data, version FROM logs WHERE id = ?")
        .map_err(|e| e.to_string())?;

    let (blob, version): (Vec<u8>, u8) = stmt
        .query_row([id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?;

    // @TODO(false): If we deserialize from an older version, we should save it back into the DB as the newer format.
    let mut parser = parser::deserialize_version(&blob, version).map_err(|e| e.to_string())?;

    parser.reparse_with_options(&options.targets);

    let duration = parser.derived_state.duration();

    let mut player_dps: HashMap<u32, Vec<i32>> = HashMap::new();

    const DPS_INTERVAL: i64 = 3 * 1_000;
    const SBA_INTERVAL: i64 = 1_000;

    for player in parser.derived_state.party.values() {
        player_dps.insert(
            player.index,
            vec![0; (duration / DPS_INTERVAL) as usize + 1],
        );
    }

    let mut targets = Vec::new();
    let start_time = parser.start_time();

    for (timestamp, event) in parser.encounter.event_log() {
        match event {
            Message::DamageEvent(damage_event) => {
                let index = ((timestamp - start_time) / DPS_INTERVAL) as usize;
                let target_type = EnemyType::from_hash(damage_event.target.parent_actor_type);

                if !targets.contains(&target_type) {
                    targets.push(target_type);
                }

                if let Some(chart) = player_dps.get_mut(&damage_event.source.parent_index) {
                    // Check to see if the target is in the list of targets to filter by.
                    if options.targets.is_empty() || options.targets.contains(&target_type) {
                        chart[index] += damage_event.damage;
                    }
                }
            }
            _ => continue,
        }
    }

    let sba_chart = parser.generate_sba_chart(SBA_INTERVAL);

    let sba_events = parser
        .encounter
        .event_log()
        .filter(|(_, e)| {
            matches!(
                e,
                Message::OnContinueSBAChain(_)
                    | Message::OnAttemptSBA(_)
                    | Message::OnPerformSBA(_)
            )
        })
        .map(|(ts, e)| (*ts - start_time, e.clone()))
        .collect();

    let death_events = parser
        .encounter
        .event_log()
        .filter(|(_, e)| matches!(e, Message::OnDeathEvent(_)))
        .map(|(ts, e)| (*ts - start_time, e.clone()))
        .collect();

    Ok(EncounterStateResponse {
        encounter_state: parser.derived_state,
        players: parser.encounter.player_data,
        quest_id: parser.encounter.quest_id,
        quest_timer: parser.encounter.quest_timer,
        quest_completed: parser.encounter.quest_completed,
        dps_chart: player_dps,
        chart_len: (duration / DPS_INTERVAL) as usize + 1,
        sba_chart_len: (duration / SBA_INTERVAL) as usize + 1,
        sba_chart,
        sba_events,
        death_events,
        targets,
    })
}

#[tauri::command]
fn delete_logs(ids: Vec<u64>) -> Result<(), String> {
    let conn = db::connect_to_db().map_err(|e| e.to_string())?;

    let id_params: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
    let param = id_params.join(",");

    let sql = format!("DELETE FROM logs WHERE id IN ({})", param);
    let mut statement = conn.prepare_cached(&sql).map_err(|e| e.to_string())?;
    statement
        .execute(params_from_iter(ids))
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn spawn_game_search(app: AppHandle, delay: Duration) {
    let search_state = app.state::<GameSearchState>().inner().clone();
    let Some(run) = search_state.try_begin() else {
        return;
    };

    if delay.is_zero() {
        emit_connection_state(&app, ConnectionState::Searching);
    }

    tauri::async_runtime::spawn(async move {
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
            emit_connection_state(&app, ConnectionState::Searching);
        }

        check_and_perform_hook(app).await;
        drop(run);
    });
}

#[tauri::command]
fn retry_game_search(app: AppHandle, state: State<ConnectionStatus>) -> bool {
    if !retry_allowed(*state.0.lock().unwrap()) {
        return false;
    }

    spawn_game_search(app, Duration::ZERO);
    true
}

// Check for the game process up to the configured attempt limit and inject the DLL when found.
async fn check_and_perform_hook(app: AppHandle) {
    let mut budget = ProcessSearchBudget::new();

    loop {
        match OwnedProcess::find_first_by_name("granblue_fantasy_relink.exe") {
            Some(target) => {
                let pipe_target = match target.try_clone() {
                    Ok(target) => target,
                    Err(error) => {
                        warn!("Could not duplicate game process handle: {:?}", error);
                        emit_connection_state(&app, ConnectionState::Unsupported);
                        return;
                    }
                };
                let syringe = Syringe::for_process(target);
                let debug_dll_path = Path::new("hook-dbg.dll");
                let mut dll_path = Path::new("hook.dll");

                // Only development builds may opt into the debug hook. A
                // stale debug DLL beside an installed release must never take
                // precedence over the packaged production hook.
                if cfg!(debug_assertions) && debug_dll_path.exists() {
                    dll_path = debug_dll_path;
                }

                info!("Found game process, injecting DLL: {:?}", dll_path);

                match syringe.inject(dll_path) {
                    Ok(_) => {
                        let _ = app.emit_all("success-alert", "Found game..");
                    }
                    Err(error) => {
                        // An older Hook may already be loaded in a game that
                        // stayed open during an app upgrade. Still try its pipe;
                        // the compatibility decoder below can consume it.
                        warn!("Could not inject Hook {:?}: {:?}", dll_path, error);
                        emit_connection_state(&app, ConnectionState::Unsupported);
                        let _ = app.emit_all(
                            "error-alert",
                            "Hook injection failed; trying the existing game connection.",
                        );
                    }
                }

                connect_and_run_parser(app, pipe_target);
                return;
            }
            None => match budget.record(false) {
                ProcessSearchDecision::Retry => {
                    tokio::time::sleep(PROCESS_SEARCH_INTERVAL).await;
                }
                ProcessSearchDecision::NotFound => {
                    emit_connection_state(&app, ConnectionState::NotFound);
                    return;
                }
                ProcessSearchDecision::Found => unreachable!("missing process cannot be found"),
            },
        }
    }
}

// Connect to the game hook event channel and listen for damage events.
fn connect_and_run_parser(app: AppHandle, target: OwnedProcess) {
    let window = app.get_window("main").expect("Window not found");
    let logs_window = app.get_window("logs").expect("Logs window not found");

    let database = db::connect_to_db().expect("Could not connect to database");
    let mut state = v1::Parser::new(app.clone(), window.clone(), database);

    tauri::async_runtime::spawn(async move {
        let pipe_wait_started = Instant::now();

        loop {
            let connection = RecvPipeStream::connect_by_path(protocol::PIPE_NAME).await;
            match pipe_wait_decision(
                connection.is_ok(),
                target.is_alive(),
                pipe_wait_started.elapsed(),
            ) {
                PipeWaitDecision::Connected => {
                    let stream = connection.expect("connected pipe decision requires a stream");
                    info!("Connected to the game pipe; awaiting hook status");
                    equipment_probe::begin_hook_session(&app);

                    let decoder = tokio_util::codec::LengthDelimitedCodec::new();
                    let mut reader = FramedRead::new(stream, decoder);
                    let mut handshake_state = HookHandshakeState::Awaiting;
                    let mut inactivity_check =
                        tokio::time::interval(std::time::Duration::from_secs(1));
                    inactivity_check
                        .set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                    loop {
                        tokio::select! {
                            message = reader.next() => {
                                let Some(Ok(msg)) = message else {
                                    break;
                                };

                                // Handle EOF when the game closes.
                                if msg.is_empty() {
                                    break;
                                }

                                let debug_mode = app.state::<DebugMode>().0.load(Ordering::Relaxed);

                                match protocol::deserialize_message(&msg) {
                                    Ok(msg) => {
                                        if debug_mode {
                                            let _ = logs_window.emit("debug-event", &msg);
                                        }

                                        let was_awaiting =
                                            handshake_state == HookHandshakeState::Awaiting;
                                        let accepted =
                                            accept_hook_message(&mut handshake_state, &msg);
                                        if accepted && was_awaiting {
                                            emit_connection_state(
                                                &app,
                                                ConnectionState::Connected,
                                            );
                                            update_equipment_connection(&app, true);
                                        }

                                        match msg {
                                        protocol::Message::HookStatus(HookStatus::Ready) => {
                                            emit_connection_state(
                                                &app,
                                                ConnectionState::Connected,
                                            );
                                            update_equipment_connection(&app, true);
                                            let _ = app.emit_all(
                                                "success-alert",
                                                "Connected to game.",
                                            );
                                        }
                                        protocol::Message::HookStatus(HookStatus::Unsupported) => {
                                            update_equipment_connection(&app, false);
                                            emit_connection_state(
                                                &app,
                                                ConnectionState::Unsupported,
                                            );
                                        }
                                        _ if !accepted => {}
                                        protocol::Message::DamageEvent(event) => {
                                            state.on_damage_event(event);
                                        }
                                        protocol::Message::OnAreaEnter(event) => {
                                            state.on_area_enter_event(event);
                                        }
                                        protocol::Message::PlayerLoadEvent(event) => {
                                            state.on_player_load_event(event);
                                        }
                                        protocol::Message::PlayerIdentityEvent(event) => {
                                            state.on_player_identity_event(event);
                                        }
                                        protocol::Message::LocalEquipmentSnapshot(event) => {
                                            equipment_probe::record_hook_snapshot(
                                                &app,
                                                event.clone(),
                                            );
                                            let response = {
                                                let equipment = app.state::<EquipmentStatus>();
                                                let mut equipment = equipment.0.lock().unwrap();
                                                equipment.apply(event);
                                                equipment.response()
                                            };
                                            let _ = logs_window.emit(
                                                "equipment-analysis-update",
                                                response,
                                            );
                                        }
                                        protocol::Message::OnQuestComplete(event) => {
                                            state.on_quest_complete_event(event);
                                        }
                                        protocol::Message::OnUpdateSBA(event) => {
                                            state.on_sba_update(event);
                                        }
                                        protocol::Message::OnAttemptSBA(event) => {
                                            state.on_sba_attempt(event);
                                        }
                                        protocol::Message::OnPerformSBA(event) => {
                                            state.on_sba_perform(event);
                                        }
                                        protocol::Message::OnContinueSBAChain(event) => {
                                            state.on_continue_sba_chain(event);
                                        }
                                        protocol::Message::OnDeathEvent(event) => {
                                            state.on_death_event(event);
                                        }
                                        protocol::Message::OnBattleEnd => {
                                            state.on_battle_end_event();
                                        }
                                        }
                                    }
                                    Err(error) => {
                                        warn!(
                                            "Could not decode Hook message ({} bytes): {:?}",
                                            msg.len(),
                                            error
                                        );
                                    }
                                }
                            }
                            _ = inactivity_check.tick() => {
                                state.auto_save_if_inactive(chrono::Utc::now().timestamp_millis());
                            }
                        }
                    }

                    info!("Game has closed.");

                    // The game has closed, so we should go back to waiting for the game to reopen.
                    state.on_connection_lost();
                    update_equipment_connection(&app, false);
                    emit_connection_state(&app, ConnectionState::Disconnected);
                    let _ = app.emit_all("error-alert", "Game connection closed");
                    break;
                }
                PipeWaitDecision::Retry => {
                    tokio::time::sleep(PIPE_CONNECT_INTERVAL).await;
                }
                PipeWaitDecision::ProcessExited => {
                    state.on_connection_lost();
                    update_equipment_connection(&app, false);
                    emit_connection_state(&app, ConnectionState::Disconnected);
                    spawn_game_search(app, PROCESS_SEARCH_INTERVAL);
                    return;
                }
                PipeWaitDecision::TimedOut => {
                    update_equipment_connection(&app, false);
                    emit_connection_state(&app, ConnectionState::Unsupported);
                    return;
                }
            }
        }

        // Check for the game process again.
        spawn_game_search(app, PROCESS_SEARCH_INTERVAL);
    });
}

fn system_tray_with_menu() -> SystemTray {
    let management = CustomMenuItem::new("open_management", "Djeeta MOD 열기");
    let toggle_clickthrough = CustomMenuItem::new("toggle_clickthrough", "클릭 통과");
    let reset_windows = CustomMenuItem::new("reset_windows", "창 위치 초기화");
    let quit = CustomMenuItem::new("quit", "종료");

    let menu = SystemTrayMenu::new()
        .add_item(management)
        .add_item(toggle_clickthrough)
        .add_item(reset_windows)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    SystemTray::new().with_menu(menu)
}

fn toggle_window_visibility(handle: &AppHandle, id: &str, focus: Option<bool>) {
    if let Some(window) = handle.get_window(id) {
        if let Some(focus_value) = focus {
            if focus_value {
                window.set_focus().unwrap();
            }
        }

        match window.is_visible().unwrap() {
            true => window.hide().unwrap(),
            false => window.show().unwrap(),
        }
    }
}

#[tauri::command]
fn set_meter_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let window = app
        .get_window("main")
        .ok_or_else(|| "meter window not found".to_string())?;
    match meter_window_action(enabled) {
        MeterWindowAction::Show => {
            window
                .set_always_on_top(true)
                .map_err(|error| error.to_string())?;
            window.show().map_err(|error| error.to_string())
        }
        MeterWindowAction::Hide => window.hide().map_err(|error| error.to_string()),
    }
}

#[tauri::command]
fn set_close_to_tray(state: State<CloseToTray>, enabled: bool) {
    state.0.store(enabled, Ordering::Release);
}

#[tauri::command]
fn toggle_clickthrough(window: tauri::Window, state: State<ClickThrough>) {
    let click_through = &state.0;
    let new_state = !click_through.load(Ordering::Acquire);
    click_through.store(new_state, Ordering::Release);
    window.set_ignore_cursor_events(new_state).unwrap();
    let _ = window.emit("on-clickthrough", new_state);
    let _ = window
        .app_handle()
        .tray_handle()
        .get_item("toggle_clickthrough")
        .set_title(if new_state {
            "클릭 통과 ✓"
        } else {
            "클릭 통과"
        });
}

fn menu_tray_handler(handle: &AppHandle, event: SystemTrayEvent) {
    let should_focus = true;
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            toggle_window_visibility(handle, "logs", Some(should_focus))
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "open_management" => toggle_window_visibility(handle, "logs", Some(should_focus)),
            "toggle_clickthrough" => toggle_clickthrough(
                handle.get_window("main").unwrap(),
                handle.state::<ClickThrough>(),
            ),
            "reset_windows" => {
                if let Some(window) = handle.get_window("main") {
                    let _ = window.unminimize();
                    let _ = reset_meter_geometry(window);
                }

                if let Some(window) = handle.get_window("logs") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_size(Size::Logical(LogicalSize {
                        width: 800.0,
                        height: 600.0,
                    }));
                }
            }
            "quit" => {
                let _ = handle.save_window_state(StateFlags::all());
                handle.exit(0)
            }
            _ => {}
        },
        _ => {} // Ignore rest of the events.
    }
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_window("logs") {
        let _ = window.show();
    }
}

fn main() {
    info!("Starting application..");

    // Setup the database.
    db::setup_db().expect("Failed to setup database");

    info!("Database setup complete, launching application..");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_window(app);
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([LogTarget::Folder("logs".into()), LogTarget::Stdout])
                .level(LevelFilter::Warn)
                .level_for("tao", LevelFilter::Error)
                .build(),
        )
        .manage(ClickThrough(AtomicBool::new(DEFAULT_CLICK_THROUGH)))
        .manage(CloseToTray(AtomicBool::new(DEFAULT_CLOSE_TO_TRAY)))
        .manage(DebugMode(AtomicBool::new(false)))
        .manage(ConnectionStatus(Mutex::new(ConnectionState::Searching)))
        .manage(GameSearchState::default())
        .manage(equipment_probe::ProbeState::default())
        .manage(equipment_probe::inventory::InventoryProbeState::default())
        .manage(repeat_quest::RepeatQuestState::default())
        .manage(EquipmentStatus(Mutex::new(
            equipment::EquipmentState::from_bundled_catalog()
                .expect("bundled trait cap catalog must be valid"),
        )))
        .system_tray(system_tray_with_menu())
        .on_system_tray_event(menu_tray_handler)
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                let close_to_tray = event
                    .window()
                    .state::<CloseToTray>()
                    .0
                    .load(Ordering::Acquire);

                match close_request_action(event.window().label(), close_to_tray) {
                    CloseRequestAction::Hide => {
                        event.window().hide().unwrap();
                        api.prevent_close();
                    }
                    CloseRequestAction::Exit => {
                        let handle = event.window().app_handle();
                        let _ = handle.save_window_state(StateFlags::all());
                        handle.exit(0);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            fetch_encounter_state,
            fetch_logs,
            delete_logs,
            delete_all_logs,
            set_meter_enabled,
            set_close_to_tray,
            export_damage_log_to_file,
            set_debug_mode,
            reset_meter_geometry,
            get_connection_state,
            retry_game_search,
            fetch_equipment_analysis,
            equipment_probe::inventory::inventory_probe_available,
            equipment_probe::inventory::capture_inventory_probe,
            repeat_quest::get_repeat_quest_status,
            repeat_quest::set_repeat_quest_enabled,
            update_install::prepare_update_install,
        ])
        .setup(|app| {
            app.state::<repeat_quest::RepeatQuestState>()
                .restore_on_startup();

            if let Some(window) = app.get_window("main") {
                window.set_skip_taskbar(true)?;
                window.set_always_on_top(true)?;
                set_meter_size(&window)?;
                window.set_ignore_cursor_events(DEFAULT_CLICK_THROUGH)?;
            }

            // Perform the game hook check in a separate thread.
            spawn_game_search(app.handle(), Duration::ZERO);
            tauri::async_runtime::spawn(equipment_probe::run_if_enabled(app.handle()));

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|handle, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                handle
                    .state::<repeat_quest::RepeatQuestState>()
                    .restore_on_exit();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::{
        accept_hook_message, close_request_action, meter_geometry, meter_window_action,
        retry_allowed, CloseRequestAction, ConnectionState, HookHandshakeState, HookStatus, Message,
        MeterGeometry, MeterWindowAction, DEFAULT_CLICK_THROUGH,
    };

    #[test]
    fn meter_geometry_matches_the_1080p_design() {
        let geometry = meter_geometry(1920.0, 1080.0);
        assert_eq!(
            geometry,
            MeterGeometry {
                x: 45.0,
                y: 470.0,
                width: 330.0,
                height: 145.0,
            }
        );
    }

    #[test]
    fn meter_geometry_does_not_shrink_below_the_four_row_design() {
        let geometry = meter_geometry(1536.0, 864.0);

        assert_eq!(geometry.x, 36.0);
        assert_eq!(geometry.y, 376.0);
        assert_eq!(geometry.width, 330.0);
        assert_eq!(geometry.height, 145.0);
    }

    #[test]
    fn meter_geometry_scales_but_does_not_exceed_one_and_a_half() {
        let geometry = meter_geometry(3840.0, 2160.0);
        assert_eq!(geometry.width, 495.0);
        assert_eq!(geometry.height, 217.5);
    }

    #[test]
    fn unsupported_handshake_rejects_later_gameplay() {
        let mut state = HookHandshakeState::Awaiting;

        assert!(!accept_hook_message(
            &mut state,
            &Message::HookStatus(HookStatus::Unsupported)
        ));
        assert!(!accept_hook_message(&mut state, &Message::OnBattleEnd));
        assert_eq!(state, HookHandshakeState::Unsupported);
    }

    #[test]
    fn gameplay_before_a_handshake_enables_legacy_compatibility() {
        let mut state = HookHandshakeState::Awaiting;

        assert!(accept_hook_message(&mut state, &Message::OnBattleEnd));
        assert_eq!(state, HookHandshakeState::Legacy);
    }

    #[test]
    fn meter_visibility_maps_to_explicit_window_actions() {
        assert_eq!(meter_window_action(true), MeterWindowAction::Show);
        assert_eq!(meter_window_action(false), MeterWindowAction::Hide);
    }

    #[test]
    fn management_close_hides_when_close_to_tray_is_enabled() {
        assert_eq!(
            close_request_action("logs", true),
            CloseRequestAction::Hide
        );
    }

    #[test]
    fn management_close_exits_when_close_to_tray_is_disabled() {
        assert_eq!(
            close_request_action("logs", false),
            CloseRequestAction::Exit
        );
    }

    #[test]
    fn unrelated_window_close_keeps_existing_hide_behavior() {
        assert_eq!(
            close_request_action("main", false),
            CloseRequestAction::Hide
        );
    }

    #[test]
    fn click_through_starts_disabled_for_dragging() {
        assert!(!DEFAULT_CLICK_THROUGH);
    }

    #[test]
    fn not_found_connection_state_uses_the_frontend_wire_name() {
        assert_eq!(
            serde_json::to_string(&ConnectionState::NotFound).unwrap(),
            "\"not-found\""
        );
    }

    #[test]
    fn retry_is_only_allowed_after_search_exhaustion() {
        assert!(retry_allowed(ConnectionState::NotFound));
        assert!(!retry_allowed(ConnectionState::Searching));
        assert!(!retry_allowed(ConnectionState::Connected));
        assert!(!retry_allowed(ConnectionState::Disconnected));
        assert!(!retry_allowed(ConnectionState::Unsupported));
    }
}
