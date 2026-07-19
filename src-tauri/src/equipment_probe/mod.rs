use std::sync::Mutex;

use protocol::LocalEquipmentSnapshotEvent;
use tauri::{AppHandle, Manager};

mod compare;
mod locator;
mod memory;

#[derive(Debug, Default)]
pub(crate) struct ProbeState(Mutex<compare::ProbeComparator>);

pub(crate) fn record_hook_snapshot(app: &AppHandle, event: LocalEquipmentSnapshotEvent) {
    app.state::<ProbeState>()
        .0
        .lock()
        .expect("equipment probe comparator lock poisoned")
        .record_hook(event);
}
