use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

pub const PROCESS_SEARCH_ATTEMPTS: u8 = 10;
pub const PROCESS_SEARCH_INTERVAL: Duration = Duration::from_secs(1);
pub const PIPE_CONNECT_INTERVAL: Duration = Duration::from_millis(100);
pub const PIPE_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, PartialEq, Eq)]
pub enum ProcessSearchDecision {
    Found,
    Retry,
    NotFound,
}

pub struct ProcessSearchBudget {
    attempts: u8,
}

impl ProcessSearchBudget {
    pub fn new() -> Self {
        Self { attempts: 0 }
    }

    pub fn record(&mut self, found: bool) -> ProcessSearchDecision {
        self.attempts = self.attempts.saturating_add(1);

        if found {
            ProcessSearchDecision::Found
        } else if self.attempts >= PROCESS_SEARCH_ATTEMPTS {
            ProcessSearchDecision::NotFound
        } else {
            ProcessSearchDecision::Retry
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PipeWaitDecision {
    Connected,
    Retry,
    ProcessExited,
    TimedOut,
}

pub fn pipe_wait_decision(
    connected: bool,
    process_alive: bool,
    elapsed: Duration,
) -> PipeWaitDecision {
    if connected {
        PipeWaitDecision::Connected
    } else if !process_alive {
        PipeWaitDecision::ProcessExited
    } else if elapsed >= PIPE_CONNECT_TIMEOUT {
        PipeWaitDecision::TimedOut
    } else {
        PipeWaitDecision::Retry
    }
}

#[derive(Clone, Default)]
pub struct GameSearchState(Arc<AtomicBool>);

impl GameSearchState {
    pub fn try_begin(&self) -> Option<GameSearchRun> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| GameSearchRun(self.0.clone()))
    }
}

pub struct GameSearchRun(Arc<AtomicBool>);

impl Drop for GameSearchRun {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        pipe_wait_decision, GameSearchState, PipeWaitDecision, ProcessSearchBudget,
        ProcessSearchDecision, PIPE_CONNECT_TIMEOUT, PROCESS_SEARCH_ATTEMPTS,
    };
    use std::time::Duration;

    #[test]
    fn tenth_missing_process_exhausts_the_search() {
        let mut budget = ProcessSearchBudget::new();

        for _ in 1..PROCESS_SEARCH_ATTEMPTS {
            assert_eq!(budget.record(false), ProcessSearchDecision::Retry);
        }
        assert_eq!(budget.record(false), ProcessSearchDecision::NotFound);
    }

    #[test]
    fn finding_a_process_on_the_tenth_attempt_wins_over_exhaustion() {
        let mut budget = ProcessSearchBudget::new();

        for _ in 1..PROCESS_SEARCH_ATTEMPTS {
            assert_eq!(budget.record(false), ProcessSearchDecision::Retry);
        }
        assert_eq!(budget.record(true), ProcessSearchDecision::Found);
    }

    #[test]
    fn search_run_guard_rejects_overlap_and_releases_on_drop() {
        let state = GameSearchState::default();
        let run = state.try_begin().expect("first run should start");

        assert!(state.try_begin().is_none());
        drop(run);
        assert!(state.try_begin().is_some());
    }

    #[test]
    fn dead_process_leaves_pipe_wait_immediately() {
        assert_eq!(
            pipe_wait_decision(false, false, Duration::ZERO),
            PipeWaitDecision::ProcessExited
        );
    }

    #[test]
    fn live_process_times_out_at_the_exact_deadline() {
        assert_eq!(
            pipe_wait_decision(false, true, PIPE_CONNECT_TIMEOUT - Duration::from_millis(1)),
            PipeWaitDecision::Retry
        );
        assert_eq!(
            pipe_wait_decision(false, true, PIPE_CONNECT_TIMEOUT),
            PipeWaitDecision::TimedOut
        );
    }

    #[test]
    fn connected_pipe_wins_before_the_deadline() {
        assert_eq!(
            pipe_wait_decision(true, true, PIPE_CONNECT_TIMEOUT - Duration::from_millis(1)),
            PipeWaitDecision::Connected
        );
    }
}
