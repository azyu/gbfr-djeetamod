use crate::{
    equipment_probe::GAME_PROCESS_NAME,
    repeat_quest::{RepeatQuestState, RepeatQuestStatus, RepeatQuestStatusKind},
};
use dll_syringe::process::OwnedProcess;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum UpdateInstallReadiness {
    Ready,
    GameRunning,
    RepeatQuestRestoreFailed,
}

fn decide_readiness(
    restore_status: Option<RepeatQuestStatus>,
    running_after_restore: bool,
) -> UpdateInstallReadiness {
    if !running_after_restore {
        return UpdateInstallReadiness::Ready;
    }

    match restore_status {
        Some(RepeatQuestStatus {
            state: RepeatQuestStatusKind::Off,
            ..
        }) => UpdateInstallReadiness::GameRunning,
        _ => UpdateInstallReadiness::RepeatQuestRestoreFailed,
    }
}

fn game_is_running() -> bool {
    OwnedProcess::find_first_by_name(GAME_PROCESS_NAME).is_some()
}

#[tauri::command]
pub(crate) async fn prepare_update_install(
    state: tauri::State<'_, RepeatQuestState>,
) -> Result<UpdateInstallReadiness, ()> {
    let state = state.inner().clone();
    if !game_is_running() && !game_is_running() {
        return Ok(UpdateInstallReadiness::Ready);
    }

    Ok(tauri::async_runtime::spawn_blocking(move || {
        let restored = state.restore_for_update();
        decide_readiness(Some(restored), game_is_running())
    })
    .await
    .unwrap_or(UpdateInstallReadiness::RepeatQuestRestoreFailed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repeat_quest::{RepeatQuestReason, RepeatQuestStatusKind};

    fn status(
        state: RepeatQuestStatusKind,
        reason: Option<RepeatQuestReason>,
    ) -> RepeatQuestStatus {
        RepeatQuestStatus { state, reason }
    }

    #[test]
    fn stopped_game_is_ready_without_restoration() {
        assert_eq!(decide_readiness(None, false), UpdateInstallReadiness::Ready);
    }

    #[test]
    fn restored_running_game_stays_blocked_until_closed() {
        assert_eq!(
            decide_readiness(Some(status(RepeatQuestStatusKind::Off, None)), true),
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
                true,
            ),
            UpdateInstallReadiness::RepeatQuestRestoreFailed
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
                false,
            ),
            UpdateInstallReadiness::Ready
        );
    }
}
