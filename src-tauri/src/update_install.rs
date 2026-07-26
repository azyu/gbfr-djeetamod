use crate::{
    conflux_timer::{ConfluxTimerState, ConfluxTimerStatus, ConfluxTimerStatusKind},
    equipment_probe::GAME_PROCESS_NAME,
    repeat_quest::{RepeatQuestState, RepeatQuestStatus, RepeatQuestStatusKind},
};
use dll_syringe::process::OwnedProcess;
use serde::Serialize;
use std::time::Duration;

use log::warn;
use tauri::AppHandle;

const UPDATE_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum UpdateInstallReadiness {
    Ready,
    GameRunning,
    RepeatQuestRestoreFailed,
    ConfluxTimerRestoreFailed,
}

fn decide_readiness(
    repeat_restore_status: Option<RepeatQuestStatus>,
    timer_restore_status: Option<ConfluxTimerStatus>,
    running_after_restore: bool,
) -> UpdateInstallReadiness {
    if !running_after_restore {
        return UpdateInstallReadiness::Ready;
    }

    if !matches!(
        repeat_restore_status,
        Some(RepeatQuestStatus {
            state: RepeatQuestStatusKind::Off,
            ..
        })
    ) {
        return UpdateInstallReadiness::RepeatQuestRestoreFailed;
    }

    if !matches!(
        timer_restore_status,
        Some(ConfluxTimerStatus {
            state: ConfluxTimerStatusKind::Off,
            ..
        })
    ) {
        return UpdateInstallReadiness::ConfluxTimerRestoreFailed;
    }

    UpdateInstallReadiness::GameRunning
}

fn game_is_running() -> bool {
    OwnedProcess::find_first_by_name(GAME_PROCESS_NAME).is_some()
}

#[tauri::command]
pub(crate) async fn prepare_update_install(
    repeat_state: tauri::State<'_, RepeatQuestState>,
    timer_state: tauri::State<'_, ConfluxTimerState>,
) -> Result<UpdateInstallReadiness, ()> {
    let repeat_state = repeat_state.inner().clone();
    let timer_state = timer_state.inner().clone();
    if !game_is_running() && !game_is_running() {
        return Ok(UpdateInstallReadiness::Ready);
    }

    Ok(tauri::async_runtime::spawn_blocking(move || {
        let repeat_restored = repeat_state.restore_for_update();
        let timer_restored = timer_state.restore_for_update();
        decide_readiness(
            Some(repeat_restored),
            Some(timer_restored),
            game_is_running(),
        )
    })
    .await
    .unwrap_or(UpdateInstallReadiness::ConfluxTimerRestoreFailed))
}

#[tauri::command]
pub(crate) async fn install_available_update(app: AppHandle) -> Result<(), String> {
    warn!(
        "UPDATER INSTALL stage=check timeout_seconds={}",
        UPDATE_REQUEST_TIMEOUT.as_secs()
    );
    let update = tauri::updater::builder(app)
        .timeout(UPDATE_REQUEST_TIMEOUT)
        .check()
        .await
        .map_err(|error| {
            warn!("UPDATER INSTALL stage=check result=failed error={error}");
            error.to_string()
        })?;

    if !update.is_update_available() {
        warn!("UPDATER INSTALL stage=check result=up-to-date");
        return Err("No update is available".to_string());
    }

    warn!(
        "UPDATER INSTALL stage=download version={} result=started",
        update.latest_version()
    );
    update.download_and_install().await.map_err(|error| {
        warn!("UPDATER INSTALL stage=download result=failed error={error}");
        error.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conflux_timer::{ConfluxTimerReason, ConfluxTimerStatusKind};
    use crate::repeat_quest::{RepeatQuestReason, RepeatQuestStatusKind};

    fn status(
        state: RepeatQuestStatusKind,
        reason: Option<RepeatQuestReason>,
    ) -> RepeatQuestStatus {
        RepeatQuestStatus { state, reason }
    }

    fn timer_status(
        state: ConfluxTimerStatusKind,
        reason: Option<ConfluxTimerReason>,
    ) -> ConfluxTimerStatus {
        ConfluxTimerStatus { state, reason }
    }

    #[test]
    fn stopped_game_is_ready_without_restoration() {
        assert_eq!(
            decide_readiness(None, None, false),
            UpdateInstallReadiness::Ready
        );
    }

    #[test]
    fn restored_running_game_stays_blocked_until_closed() {
        assert_eq!(
            decide_readiness(
                Some(status(RepeatQuestStatusKind::Off, None)),
                Some(timer_status(ConfluxTimerStatusKind::Off, None)),
                true,
            ),
            UpdateInstallReadiness::GameRunning
        );
    }

    #[test]
    fn restoration_failure_blocks_a_still_running_game() {
        assert_eq!(
            decide_readiness(
                Some(status(
                    RepeatQuestStatusKind::Unavailable,
                    Some(RepeatQuestReason::RestoreFailed),
                )),
                Some(timer_status(ConfluxTimerStatusKind::Off, None)),
                true,
            ),
            UpdateInstallReadiness::RepeatQuestRestoreFailed
        );
    }

    #[test]
    fn timer_restoration_failure_blocks_a_still_running_game() {
        assert_eq!(
            decide_readiness(
                Some(status(RepeatQuestStatusKind::Off, None)),
                Some(timer_status(
                    ConfluxTimerStatusKind::Unavailable,
                    Some(ConfluxTimerReason::RestoreFailed),
                )),
                true,
            ),
            UpdateInstallReadiness::ConfluxTimerRestoreFailed
        );
    }

    #[test]
    fn process_exit_during_restoration_is_ready() {
        assert_eq!(
            decide_readiness(
                Some(status(
                    RepeatQuestStatusKind::Unavailable,
                    Some(RepeatQuestReason::GameNotRunning),
                )),
                Some(timer_status(
                    ConfluxTimerStatusKind::Unavailable,
                    Some(ConfluxTimerReason::GameNotRunning),
                )),
                false,
            ),
            UpdateInstallReadiness::Ready
        );
    }
}
