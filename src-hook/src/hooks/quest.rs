use std::sync::atomic::Ordering;

use anyhow::{anyhow, Result};
use protocol::Message;
use retour::static_detour;

use crate::{
    event,
    hooks::{ffi::QuestState, globals::QUEST_STATE_PTR},
    process::Process,
};

type OnLoadQuestStateFunc = unsafe extern "system" fn(*const usize) -> usize;
type OnShowResultScreenFunc = unsafe extern "system" fn(*const usize) -> usize;
type OnBattleEndFunc = unsafe extern "system" fn(*const usize);

static_detour! {
    static OnLoadQuestState: unsafe extern "system" fn(*const usize) -> usize;
    static OnShowResultScreen: unsafe extern "system" fn(*const usize) -> usize;
    static OnBattleEnd: unsafe extern "system" fn(*const usize);
}

const ON_LOAD_QUEST_STATE: &str =
    "48 8b 0d ? ? ? ? e8 $ { ' } c5 fb 12 ? ? ? ? ? c5 f8 11 ? ? ? ? ? c5 f8 11 ? ? ? ? ? 48 83 c4 48";
const ON_SHOW_RESULT_SCREEN_SIG: &str =
    "e8 $ { ' } b8 ? ? ? ? 23 87 ? ? 00 00 3d 00 00 60 00 0f 94 c0";

/// Game 2.0.2 `ui::action::fsm::ResultRewardSetup::execute`.
///
/// Unlike ResultEnableInputOperation, this operation belongs to the actual quest
/// reward/result flow and is not reused by fall recovery or boss-mechanic UI.
/// The signature is unique in the 2.0.2 executable and requires no quest-state reads.
const ON_BATTLE_END_SIG: &str =
    "41 56 56 57 53 48 83 ec 38 48 89 ce 48 8b 0d ? ? ? ? 48 8d 54 24 30 41 b8 ab 4e f1 51 e8 ? ? ? ? 48 8b 44 24 30 48 85 c0 0f 84 ? ? ? ? 48 8b 58 18 4c 8b 70 20 4c 39 f3 0f 84 ? ? ? ? 48 8d 7c 24 2c";

fn notify_before_original(notify: impl FnOnce(), original: impl FnOnce()) {
    notify();
    original();
}

/// Called once the quest result screen is ready for input.
#[derive(Clone)]
pub struct OnBattleEndHook {
    tx: event::Tx,
}

impl OnBattleEndHook {
    pub fn new(tx: event::Tx) -> Self {
        Self { tx }
    }

    pub fn setup(&self, process: &Process) -> Result<()> {
        let cloned_self = self.clone();
        let on_battle_end = process
            .search_match_address(ON_BATTLE_END_SIG)
            .map_err(|_| anyhow!("Could not find game 2.0.2 result reward setup"))?;

        #[cfg(feature = "console")]
        println!("Found game 2.0.2 result reward setup");

        unsafe {
            let func: OnBattleEndFunc = std::mem::transmute(on_battle_end);
            OnBattleEnd.initialize(func, move |a1| cloned_self.run(a1))?;
            OnBattleEnd.enable()?;
        }

        Ok(())
    }

    fn run(&self, a1: *const usize) {
        notify_before_original(
            || {
                super::reset_battle_identity_state();
                let _ = self.tx.send(Message::OnBattleEnd);
            },
            || unsafe { OnBattleEnd.call(a1) },
        );
    }
}

/// Called while loading into a quest.
#[derive(Clone)]
pub struct OnLoadQuestHook {}

impl OnLoadQuestHook {
    pub fn new() -> Self {
        OnLoadQuestHook {}
    }

    pub fn setup(&self, process: &Process) -> Result<()> {
        let cloned_self = self.clone();

        if let Ok(on_load_quest_state) = process.search_address(ON_LOAD_QUEST_STATE) {
            #[cfg(feature = "console")]
            println!("Found on load quest state");

            unsafe {
                let func: OnLoadQuestStateFunc = std::mem::transmute(on_load_quest_state);
                OnLoadQuestState.initialize(func, move |a1| cloned_self.run(a1))?;
                OnLoadQuestState.enable()?;
            }
        } else {
            return Err(anyhow!("Could not find on_load_quest_state"));
        }

        Ok(())
    }

    fn run(&self, a1: *const usize) -> usize {
        #[cfg(feature = "console")]
        println!("on load quest state");

        let ret = unsafe { OnLoadQuestState.call(a1) };
        let quest_state_ptr = unsafe { a1.byte_add(0x1D8) } as *mut QuestState;

        if quest_state_ptr.is_null() {
            return ret;
        }

        QUEST_STATE_PTR.store(quest_state_ptr, std::sync::atomic::Ordering::Relaxed);

        ret
    }
}

/// Called whenever the result screen is shown for the quest.
#[derive(Clone)]
pub struct OnQuestCompleteHook {
    tx: event::Tx,
}

impl OnQuestCompleteHook {
    pub fn new(tx: event::Tx) -> Self {
        OnQuestCompleteHook { tx }
    }

    pub fn setup(&self, process: &Process) -> Result<()> {
        let cloned_self = self.clone();

        if let Ok(on_show_result_screen) = process.search_address(ON_SHOW_RESULT_SCREEN_SIG) {
            #[cfg(feature = "console")]
            println!("Found on show result screen");

            unsafe {
                let func: OnShowResultScreenFunc = std::mem::transmute(on_show_result_screen);
                OnShowResultScreen.initialize(func, move |a1| cloned_self.run(a1))?;
                OnShowResultScreen.enable()?;
            }
        } else {
            return Err(anyhow!("Could not find on_show_result_screen"));
        }

        Ok(())
    }

    fn run(&self, a1: *const usize) -> usize {
        #[cfg(feature = "console")]
        println!("on show result screen");

        let quest_state_ptr = QUEST_STATE_PTR.load(Ordering::Relaxed);

        if !quest_state_ptr.is_null() {
            #[cfg(feature = "console")]
            println!("quest_state_ptr: {:p}", quest_state_ptr);

            let quest_state = unsafe { quest_state_ptr.read() };
            let quest_id = quest_state.quest_id;
            let timer = quest_state.elapsed_time;

            let _ = self
                .tx
                .send(Message::OnQuestComplete(protocol::QuestCompleteEvent {
                    quest_id,
                    elapsed_time_in_secs: timer,
                }));
        }

        unsafe { OnShowResultScreen.call(a1) }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::notify_before_original;

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
