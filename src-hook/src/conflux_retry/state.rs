use protocol::{
    ConfluxAutomationReason, ConfluxAutomationStage, ConfluxAutomationState,
    ConfluxAutomationStatus,
};

const TRANSITION_TIMEOUT_MS: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Observation {
    None,
    RewardReady { screen_id: u64, target_index: u32 },
    RewardSelected { screen_id: u64, selected_index: u32 },
    TotalResultsReady { screen_id: u64 },
    ReturnDestinationReady { screen_id: u64 },
    TredameLoaded,
    TredameGateReady { screen_id: u64 },
    PartyReady { screen_id: u64 },
    DifficultyReady { screen_id: u64 },
    BattleLoaded,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestedAction {
    SelectReward(u32),
    ConfirmReward,
    AdvanceTotalResults,
    ConfirmTredame,
    ActivateGate,
    ConfirmCurrentParty,
    ConfirmFocusedDepth,
    Disable(ConfluxAutomationReason),
}

#[derive(Debug)]
pub(crate) struct ConfluxStateMachine {
    status: ConfluxAutomationStatus,
    reward_screen_id: Option<u64>,
    reward_target_index: Option<u32>,
    last_screen_id: Option<u64>,
    deadline_ms: Option<u64>,
}

impl Default for ConfluxStateMachine {
    fn default() -> Self {
        Self {
            status: ConfluxAutomationStatus {
                state: ConfluxAutomationState::Off,
                stage: ConfluxAutomationStage::Off,
                reason: None,
                reward_id: None,
                revision: 0,
            },
            reward_screen_id: None,
            reward_target_index: None,
            last_screen_id: None,
            deadline_ms: None,
        }
    }
}

impl ConfluxStateMachine {
    pub(crate) fn configure(
        &mut self,
        enabled: bool,
        reward_id: u32,
        revision: u64,
        _now_ms: u64,
    ) -> Result<(), ConfluxAutomationReason> {
        if !enabled {
            self.set_off();
            return Ok(());
        }
        if reward_id == 0 {
            return Err(ConfluxAutomationReason::InvalidPreference);
        }

        self.status = ConfluxAutomationStatus {
            state: ConfluxAutomationState::On,
            stage: ConfluxAutomationStage::Armed,
            reason: None,
            reward_id: Some(reward_id),
            revision,
        };
        self.clear_pending();
        Ok(())
    }

    pub(crate) fn status(&self) -> ConfluxAutomationStatus {
        self.status.clone()
    }

    pub(crate) fn observe(
        &mut self,
        observation: Observation,
        now_ms: u64,
    ) -> Option<RequestedAction> {
        if self.status.state != ConfluxAutomationState::On {
            return None;
        }
        if observation == Observation::Invalid {
            return self.disable(ConfluxAutomationReason::InvalidObservation);
        }
        if observation == Observation::None {
            return self
                .deadline_ms
                .filter(|deadline| now_ms > *deadline)
                .and_then(|_| self.disable(ConfluxAutomationReason::TransitionTimeout));
        }

        let action = match self.status.stage {
            ConfluxAutomationStage::Armed => match observation {
                Observation::RewardReady {
                    screen_id,
                    target_index,
                } => {
                    self.reward_screen_id = Some(screen_id);
                    self.reward_target_index = Some(target_index);
                    self.status.stage = ConfluxAutomationStage::RewardSelection;
                    RequestedAction::SelectReward(target_index)
                }
                Observation::BattleLoaded => return None,
                _ => return self.disable(ConfluxAutomationReason::UnexpectedSuccessor),
            },
            ConfluxAutomationStage::RewardSelection => match observation {
                Observation::RewardReady { screen_id, .. }
                    if Some(screen_id) == self.reward_screen_id =>
                {
                    return None
                }
                Observation::RewardSelected {
                    screen_id,
                    selected_index,
                } if Some(screen_id) == self.reward_screen_id
                    && Some(selected_index) == self.reward_target_index =>
                {
                    self.last_screen_id = Some(screen_id);
                    self.status.stage = ConfluxAutomationStage::TotalResults;
                    RequestedAction::ConfirmReward
                }
                _ => return self.disable(ConfluxAutomationReason::UnexpectedSuccessor),
            },
            ConfluxAutomationStage::TotalResults => match observation {
                Observation::RewardSelected { screen_id, .. }
                    if Some(screen_id) == self.last_screen_id =>
                {
                    return None
                }
                Observation::TotalResultsReady { screen_id } => {
                    self.last_screen_id = Some(screen_id);
                    self.status.stage = ConfluxAutomationStage::ReturnDestination;
                    RequestedAction::AdvanceTotalResults
                }
                _ => return self.disable(ConfluxAutomationReason::UnexpectedSuccessor),
            },
            ConfluxAutomationStage::ReturnDestination => match observation {
                Observation::TotalResultsReady { screen_id }
                    if Some(screen_id) == self.last_screen_id =>
                {
                    return None
                }
                Observation::ReturnDestinationReady { screen_id } => {
                    self.last_screen_id = Some(screen_id);
                    self.status.stage = ConfluxAutomationStage::TredameGate;
                    RequestedAction::ConfirmTredame
                }
                _ => return self.disable(ConfluxAutomationReason::UnexpectedSuccessor),
            },
            ConfluxAutomationStage::TredameGate => match observation {
                Observation::ReturnDestinationReady { screen_id }
                    if Some(screen_id) == self.last_screen_id =>
                {
                    return None
                }
                Observation::TredameLoaded => {
                    self.deadline_ms = Some(now_ms.saturating_add(TRANSITION_TIMEOUT_MS));
                    return None;
                }
                Observation::TredameGateReady { screen_id } => {
                    self.last_screen_id = Some(screen_id);
                    self.status.stage = ConfluxAutomationStage::PartyFormation;
                    RequestedAction::ActivateGate
                }
                _ => return self.disable(ConfluxAutomationReason::UnexpectedSuccessor),
            },
            ConfluxAutomationStage::PartyFormation => match observation {
                Observation::TredameGateReady { screen_id }
                    if Some(screen_id) == self.last_screen_id =>
                {
                    return None
                }
                Observation::PartyReady { screen_id } => {
                    self.last_screen_id = Some(screen_id);
                    self.status.stage = ConfluxAutomationStage::DifficultyConfirmation;
                    RequestedAction::ConfirmCurrentParty
                }
                _ => return self.disable(ConfluxAutomationReason::UnexpectedSuccessor),
            },
            ConfluxAutomationStage::DifficultyConfirmation => match observation {
                Observation::PartyReady { screen_id } if Some(screen_id) == self.last_screen_id => {
                    return None
                }
                Observation::DifficultyReady { screen_id } => {
                    self.last_screen_id = Some(screen_id);
                    RequestedAction::ConfirmFocusedDepth
                }
                Observation::BattleLoaded => {
                    self.status.stage = ConfluxAutomationStage::Armed;
                    self.clear_pending();
                    return None;
                }
                _ => return self.disable(ConfluxAutomationReason::UnexpectedSuccessor),
            },
            ConfluxAutomationStage::Off | ConfluxAutomationStage::Unavailable => return None,
        };

        self.deadline_ms = Some(now_ms.saturating_add(TRANSITION_TIMEOUT_MS));
        Some(action)
    }

    fn set_off(&mut self) {
        self.status = ConfluxAutomationStatus {
            state: ConfluxAutomationState::Off,
            stage: ConfluxAutomationStage::Off,
            reason: None,
            reward_id: None,
            revision: 0,
        };
        self.clear_pending();
    }

    fn disable(&mut self, reason: ConfluxAutomationReason) -> Option<RequestedAction> {
        self.status = ConfluxAutomationStatus {
            state: ConfluxAutomationState::Unavailable,
            stage: ConfluxAutomationStage::Unavailable,
            reason: Some(reason),
            reward_id: None,
            revision: 0,
        };
        self.clear_pending();
        Some(RequestedAction::Disable(reason))
    }

    fn clear_pending(&mut self) {
        self.reward_screen_id = None;
        self.reward_target_index = None;
        self.last_screen_id = None;
        self.deadline_ms = None;
    }
}

#[cfg(test)]
mod tests {
    use protocol::{ConfluxAutomationReason, ConfluxAutomationStage, ConfluxAutomationState};

    use super::{ConfluxStateMachine, Observation, RequestedAction};

    const REWARD_ID: u32 = 0x1234_5678;

    fn configured() -> ConfluxStateMachine {
        let mut machine = ConfluxStateMachine::default();
        machine.configure(true, REWARD_ID, 7, 1_000).unwrap();
        machine
    }

    #[test]
    fn complete_sequence_emits_one_action_at_each_verified_boundary() {
        let mut machine = configured();

        assert_eq!(
            machine.observe(
                Observation::RewardReady {
                    screen_id: 10,
                    target_index: 2,
                },
                1_100,
            ),
            Some(RequestedAction::SelectReward(2))
        );
        assert_eq!(
            machine.observe(
                Observation::RewardSelected {
                    screen_id: 10,
                    selected_index: 2,
                },
                1_200,
            ),
            Some(RequestedAction::ConfirmReward)
        );
        assert_eq!(
            machine.observe(Observation::TotalResultsReady { screen_id: 20 }, 1_300,),
            Some(RequestedAction::AdvanceTotalResults)
        );
        assert_eq!(
            machine.observe(Observation::ReturnDestinationReady { screen_id: 30 }, 1_400,),
            Some(RequestedAction::ConfirmTredame)
        );
        assert_eq!(machine.observe(Observation::TredameLoaded, 1_500), None);
        assert_eq!(
            machine.observe(Observation::TredameGateReady { screen_id: 40 }, 1_600),
            Some(RequestedAction::ActivateGate)
        );
        assert_eq!(
            machine.observe(Observation::PartyReady { screen_id: 50 }, 1_700),
            Some(RequestedAction::ConfirmCurrentParty)
        );
        assert_eq!(
            machine.observe(Observation::DifficultyReady { screen_id: 60 }, 1_800),
            Some(RequestedAction::ConfirmFocusedDepth)
        );
        assert_eq!(machine.observe(Observation::BattleLoaded, 1_900), None);

        let status = machine.status();
        assert_eq!(status.state, ConfluxAutomationState::On);
        assert_eq!(status.stage, ConfluxAutomationStage::Armed);
        assert_eq!(status.reward_id, Some(REWARD_ID));
        assert_eq!(status.revision, 7);
    }

    #[test]
    fn duplicate_screen_does_not_repeat_an_action() {
        let mut machine = configured();
        let observation = Observation::RewardReady {
            screen_id: 10,
            target_index: 2,
        };

        assert_eq!(
            machine.observe(observation, 1_100),
            Some(RequestedAction::SelectReward(2))
        );
        assert_eq!(machine.observe(observation, 1_200), None);
    }

    #[test]
    fn out_of_order_successor_disables_automation() {
        let mut machine = configured();

        assert_eq!(
            machine.observe(Observation::PartyReady { screen_id: 50 }, 1_100),
            Some(RequestedAction::Disable(
                ConfluxAutomationReason::UnexpectedSuccessor
            ))
        );
        assert_eq!(machine.status().state, ConfluxAutomationState::Unavailable);
    }

    #[test]
    fn pending_transition_timeout_disables_automation() {
        let mut machine = configured();
        assert_eq!(
            machine.observe(
                Observation::RewardReady {
                    screen_id: 10,
                    target_index: 2,
                },
                1_100,
            ),
            Some(RequestedAction::SelectReward(2))
        );

        assert_eq!(
            machine.observe(Observation::None, 11_101),
            Some(RequestedAction::Disable(
                ConfluxAutomationReason::TransitionTimeout
            ))
        );
    }

    #[test]
    fn off_state_ignores_observations_and_clears_preference() {
        let mut machine = configured();
        machine.configure(false, REWARD_ID, 8, 1_100).unwrap();

        assert_eq!(
            machine.observe(
                Observation::RewardReady {
                    screen_id: 10,
                    target_index: 2,
                },
                1_200,
            ),
            None
        );
        let status = machine.status();
        assert_eq!(status.state, ConfluxAutomationState::Off);
        assert_eq!(status.stage, ConfluxAutomationStage::Off);
        assert_eq!(status.reward_id, None);
        assert_eq!(status.revision, 0);
    }

    #[test]
    fn zero_reward_id_is_rejected() {
        let mut machine = ConfluxStateMachine::default();

        assert_eq!(
            machine.configure(true, 0, 1, 1_000),
            Err(ConfluxAutomationReason::InvalidPreference)
        );
        assert_eq!(machine.status().state, ConfluxAutomationState::Off);
    }

    #[test]
    fn invalid_observation_disables_automation() {
        let mut machine = configured();

        assert_eq!(
            machine.observe(Observation::Invalid, 1_100),
            Some(RequestedAction::Disable(
                ConfluxAutomationReason::InvalidObservation
            ))
        );
    }
}
