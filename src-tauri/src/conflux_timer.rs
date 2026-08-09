use crate::equipment_probe::{
    memory::{MemoryReadError, MemoryReader, RemoteProcess},
    GAME_PROCESS_NAME,
};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[cfg(windows)]
use std::ffi::c_void;
#[cfg(windows)]
use windows::Win32::{
    Foundation::{CloseHandle, ERROR_ACCESS_DENIED, HANDLE},
    System::{
        Diagnostics::Debug::WriteProcessMemory,
        Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ,
            PROCESS_VM_WRITE,
        },
    },
};

const PINNED_CONFLUX_TIMER_SHA256: &str =
    "F827F3C13CAA90B290FAB2FE7E28165A80448FDE0A3F7A96D79DAC6B8343FF2A";

const TIMER_MANAGER_POINTER_RVA: usize = 0x07C2_2078;
const TIMER_NOTICE_THRESHOLD_OFFSET: usize = 0x2DA4;
const TIMER_DEFAULTS_OFFSET: usize = 0x2DA8;
const TIMER_MODE_OFFSET: usize = 0x2DE0;
const TIMER_INITIAL_OFFSET: usize = 0x346C;
const TIMER_CURRENT_OFFSET: usize = 0x3470;
const TIMER_CONFIG_FLOATS: usize = 12;
const TIMER_CONFIG_BYTES: usize = (TIMER_DEFAULTS_OFFSET - TIMER_NOTICE_THRESHOLD_OFFSET) + 11 * 4;
const TIMER_ACTIVE_BYTES: usize = (TIMER_CURRENT_OFFSET - TIMER_INITIAL_OFFSET) + 4;
const ENDLESS_MODE: u32 = 1;

const ORIGINAL_TIMER_CONFIG: [f32; TIMER_CONFIG_FLOATS] = [
    3.0, 60.0, 60.0, 30.0, 30.0, 30.0, 30.0, 30.0, 30.0, 60.0, 30.0, 30.0,
];
const FAST_TIMER_CONFIG: [f32; TIMER_CONFIG_FLOATS] =
    [1.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0];
const FAST_PROGRESS_SECONDS: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ObservedTimerState {
    Off,
    On,
    Mixed,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActiveTimer {
    initial_seconds: f32,
    current_seconds: f32,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum ConfluxTimerError {
    #[error("game is not running")]
    GameNotRunning,
    #[error("unsupported game executable")]
    UnsupportedGame,
    #[error("Conflux timer manager is unavailable")]
    ManagerUnavailable,
    #[error("game is not in Endless mode")]
    NotEndlessMode,
    #[error("timer addresses overflowed")]
    AddressOverflow,
    #[error("timer values are neither original nor patched")]
    UnexpectedValues,
    #[error("read failed at {address:#x}: {detail}")]
    Read { address: usize, detail: String },
    #[error("write failed at {address:#x}: {detail}")]
    Write { address: usize, detail: String },
    #[error("write returned {actual} of {expected} bytes")]
    PartialWrite { expected: usize, actual: usize },
    #[error("timer read-back did not match the requested state")]
    ReadBackMismatch,
    #[error("enable failed and rollback did not restore the original values")]
    Rollback,
    #[error("pinned SHA-256 constant is invalid")]
    InvalidPinnedHash,
    #[error("process access denied")]
    AccessDenied,
    #[error("process operation failed: {0}")]
    Process(String),
}

fn parse_sha256(value: &str) -> Result<[u8; 32], ConfluxTimerError> {
    if value.len() != 64 || !value.is_ascii() {
        return Err(ConfluxTimerError::InvalidPinnedHash);
    }
    let mut output = [0u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16)
            .map_err(|_| ConfluxTimerError::InvalidPinnedHash)?;
    }
    Ok(output)
}

fn verify_game_hash(actual: &[u8; 32]) -> Result<(), ConfluxTimerError> {
    if *actual == parse_sha256(PINNED_CONFLUX_TIMER_SHA256)? {
        Ok(())
    } else {
        Err(ConfluxTimerError::UnsupportedGame)
    }
}

fn encode_config(values: [f32; TIMER_CONFIG_FLOATS]) -> [u8; TIMER_CONFIG_BYTES] {
    let mut bytes = [0u8; TIMER_CONFIG_BYTES];
    for (index, value) in values.into_iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn decode_active(bytes: [u8; TIMER_ACTIVE_BYTES]) -> ActiveTimer {
    ActiveTimer {
        initial_seconds: f32::from_le_bytes(bytes[..4].try_into().expect("four initial bytes")),
        current_seconds: f32::from_le_bytes(bytes[4..].try_into().expect("four current bytes")),
    }
}

fn encode_active(timer: ActiveTimer) -> [u8; TIMER_ACTIVE_BYTES] {
    let mut bytes = [0u8; TIMER_ACTIVE_BYTES];
    bytes[..4].copy_from_slice(&timer.initial_seconds.to_le_bytes());
    bytes[4..].copy_from_slice(&timer.current_seconds.to_le_bytes());
    bytes
}

fn classify_config(bytes: [u8; TIMER_CONFIG_BYTES]) -> ObservedTimerState {
    let original = encode_config(ORIGINAL_TIMER_CONFIG);
    let fast = encode_config(FAST_TIMER_CONFIG);
    if bytes == original {
        return ObservedTimerState::Off;
    }
    if bytes == fast {
        return ObservedTimerState::On;
    }

    let mut original_fields = 0;
    let mut fast_fields = 0;
    for index in 0..TIMER_CONFIG_FLOATS {
        let field = &bytes[index * 4..index * 4 + 4];
        if field == &original[index * 4..index * 4 + 4] {
            original_fields += 1;
        } else if field == &fast[index * 4..index * 4 + 4] {
            fast_fields += 1;
        } else {
            return ObservedTimerState::Unknown;
        }
    }
    if original_fields > 0 && fast_fields > 0 {
        ObservedTimerState::Mixed
    } else {
        ObservedTimerState::Unknown
    }
}

fn shortened_active(timer: ActiveTimer) -> Result<ActiveTimer, ConfluxTimerError> {
    if !timer.initial_seconds.is_finite()
        || !timer.current_seconds.is_finite()
        || timer.initial_seconds < 0.0
        || timer.current_seconds < 0.0
    {
        return Err(ConfluxTimerError::UnexpectedValues);
    }
    Ok(ActiveTimer {
        initial_seconds: timer.initial_seconds.min(FAST_PROGRESS_SECONDS),
        current_seconds: timer.current_seconds.min(FAST_PROGRESS_SECONDS),
    })
}

trait TimerMemory {
    fn read_config(&self) -> Result<[u8; TIMER_CONFIG_BYTES], ConfluxTimerError>;
    fn write_config(&mut self, bytes: [u8; TIMER_CONFIG_BYTES]) -> Result<(), ConfluxTimerError>;
    fn read_active(&self) -> Result<[u8; TIMER_ACTIVE_BYTES], ConfluxTimerError>;
    fn write_active(&mut self, bytes: [u8; TIMER_ACTIVE_BYTES]) -> Result<(), ConfluxTimerError>;
}

fn observe_timer(memory: &impl TimerMemory) -> Result<ObservedTimerState, ConfluxTimerError> {
    Ok(classify_config(memory.read_config()?))
}

fn restore_timer(memory: &mut impl TimerMemory) -> Result<ObservedTimerState, ConfluxTimerError> {
    match observe_timer(memory)? {
        ObservedTimerState::Off => return Ok(ObservedTimerState::Off),
        ObservedTimerState::On | ObservedTimerState::Mixed => {}
        ObservedTimerState::Unknown => return Err(ConfluxTimerError::UnexpectedValues),
    }
    memory.write_config(encode_config(ORIGINAL_TIMER_CONFIG))?;
    if observe_timer(memory)? == ObservedTimerState::Off {
        Ok(ObservedTimerState::Off)
    } else {
        Err(ConfluxTimerError::ReadBackMismatch)
    }
}

fn rollback_enable(
    memory: &mut impl TimerMemory,
    previous_active: [u8; TIMER_ACTIVE_BYTES],
) -> Result<(), ConfluxTimerError> {
    let config_result = memory.write_config(encode_config(ORIGINAL_TIMER_CONFIG));
    let active_result = memory.write_active(previous_active);
    if config_result.is_ok()
        && active_result.is_ok()
        && observe_timer(memory) == Ok(ObservedTimerState::Off)
        && memory.read_active() == Ok(previous_active)
    {
        Ok(())
    } else {
        Err(ConfluxTimerError::Rollback)
    }
}

fn enable_timer(memory: &mut impl TimerMemory) -> Result<ObservedTimerState, ConfluxTimerError> {
    let observed = observe_timer(memory)?;
    if matches!(
        observed,
        ObservedTimerState::Mixed | ObservedTimerState::Unknown
    ) {
        return Err(ConfluxTimerError::UnexpectedValues);
    }

    let previous_active = memory.read_active()?;
    let active = shortened_active(decode_active(previous_active))?;
    if observed == ObservedTimerState::Off {
        if let Err(error) = memory.write_config(encode_config(FAST_TIMER_CONFIG)) {
            rollback_enable(memory, previous_active)?;
            return Err(error);
        }
    }
    if let Err(error) = memory.write_active(encode_active(active)) {
        rollback_enable(memory, previous_active)?;
        return Err(error);
    }
    if observe_timer(memory)? != ObservedTimerState::On
        || memory.read_active()? != encode_active(active)
    {
        rollback_enable(memory, previous_active)?;
        return Err(ConfluxTimerError::ReadBackMismatch);
    }
    Ok(ObservedTimerState::On)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TimerSites {
    config: usize,
    mode: usize,
    active: usize,
}

fn timer_sites(manager: usize) -> Result<TimerSites, ConfluxTimerError> {
    Ok(TimerSites {
        config: manager
            .checked_add(TIMER_NOTICE_THRESHOLD_OFFSET)
            .ok_or(ConfluxTimerError::AddressOverflow)?,
        mode: manager
            .checked_add(TIMER_MODE_OFFSET)
            .ok_or(ConfluxTimerError::AddressOverflow)?,
        active: manager
            .checked_add(TIMER_INITIAL_OFFSET)
            .ok_or(ConfluxTimerError::AddressOverflow)?,
    })
}

#[cfg(windows)]
fn map_memory_error(error: MemoryReadError, address: usize) -> ConfluxTimerError {
    let detail = error.to_string();
    if detail.contains("0x80070005") {
        ConfluxTimerError::AccessDenied
    } else {
        ConfluxTimerError::Read { address, detail }
    }
}

#[cfg(windows)]
fn resolve_process_sites() -> Result<(RemoteProcess, TimerSites), ConfluxTimerError> {
    let process = RemoteProcess::find(GAME_PROCESS_NAME)
        .map_err(|error| map_memory_error(error, 0))?
        .ok_or(ConfluxTimerError::GameNotRunning)?;
    let hash = process
        .executable_sha256()
        .map_err(|error| map_memory_error(error, process.module_base))?;
    verify_game_hash(&hash)?;
    let pointer_address = process
        .module_base
        .checked_add(TIMER_MANAGER_POINTER_RVA)
        .ok_or(ConfluxTimerError::AddressOverflow)?;
    let mut pointer_bytes = [0u8; 8];
    process
        .read_exact(pointer_address, &mut pointer_bytes)
        .map_err(|error| map_memory_error(error, pointer_address))?;
    let manager = usize::from_le_bytes(pointer_bytes);
    if manager == 0 {
        return Err(ConfluxTimerError::ManagerUnavailable);
    }
    if !process
        .is_running()
        .map_err(|error| map_memory_error(error, process.module_base))?
    {
        return Err(ConfluxTimerError::GameNotRunning);
    }
    Ok((process, timer_sites(manager)?))
}

#[cfg(windows)]
#[derive(Debug)]
struct WritableHandle(HANDLE);

#[cfg(windows)]
impl Drop for WritableHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
struct RemoteTimerMemory<'a> {
    reader: &'a RemoteProcess,
    sites: TimerSites,
    writer: Option<WritableHandle>,
}

#[cfg(windows)]
impl<'a> RemoteTimerMemory<'a> {
    fn read_only(reader: &'a RemoteProcess, sites: TimerSites) -> Self {
        Self {
            reader,
            sites,
            writer: None,
        }
    }

    fn writable(reader: &'a RemoteProcess, sites: TimerSites) -> Result<Self, ConfluxTimerError> {
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_INFORMATION
                    | PROCESS_VM_READ
                    | PROCESS_VM_WRITE
                    | PROCESS_VM_OPERATION,
                false,
                reader.pid,
            )
        }
        .map_err(map_open_error)?;
        Ok(Self {
            reader,
            sites,
            writer: Some(WritableHandle(handle)),
        })
    }

    fn read<const N: usize>(&self, address: usize) -> Result<[u8; N], ConfluxTimerError> {
        let mut bytes = [0u8; N];
        self.reader
            .read_exact(address, &mut bytes)
            .map_err(|error| map_memory_error(error, address))?;
        Ok(bytes)
    }

    fn write(&self, address: usize, bytes: &[u8]) -> Result<(), ConfluxTimerError> {
        let writer = self
            .writer
            .as_ref()
            .ok_or_else(|| ConfluxTimerError::Write {
                address,
                detail: "read-only timer adapter".to_owned(),
            })?;
        let mut written = 0usize;
        unsafe {
            WriteProcessMemory(
                writer.0,
                address as *const c_void,
                bytes.as_ptr().cast::<c_void>(),
                bytes.len(),
                Some(&mut written),
            )
        }
        .map_err(|error| ConfluxTimerError::Write {
            address,
            detail: error.to_string(),
        })?;
        if written != bytes.len() {
            return Err(ConfluxTimerError::PartialWrite {
                expected: bytes.len(),
                actual: written,
            });
        }
        Ok(())
    }

    fn mode(&self) -> Result<u32, ConfluxTimerError> {
        Ok(u32::from_le_bytes(self.read(self.sites.mode)?))
    }
}

#[cfg(windows)]
impl TimerMemory for RemoteTimerMemory<'_> {
    fn read_config(&self) -> Result<[u8; TIMER_CONFIG_BYTES], ConfluxTimerError> {
        self.read(self.sites.config)
    }

    fn write_config(&mut self, bytes: [u8; TIMER_CONFIG_BYTES]) -> Result<(), ConfluxTimerError> {
        self.write(self.sites.config, &bytes)
    }

    fn read_active(&self) -> Result<[u8; TIMER_ACTIVE_BYTES], ConfluxTimerError> {
        self.read(self.sites.active)
    }

    fn write_active(&mut self, bytes: [u8; TIMER_ACTIVE_BYTES]) -> Result<(), ConfluxTimerError> {
        self.write(self.sites.active, &bytes)
    }
}

#[cfg(windows)]
fn map_open_error(error: windows::core::Error) -> ConfluxTimerError {
    if error.code() == ERROR_ACCESS_DENIED.to_hresult() {
        ConfluxTimerError::AccessDenied
    } else {
        ConfluxTimerError::Process(error.to_string())
    }
}

#[cfg(windows)]
fn observe_current() -> Result<ObservedTimerState, ConfluxTimerError> {
    let (process, sites) = resolve_process_sites()?;
    observe_timer(&RemoteTimerMemory::read_only(&process, sites))
}

#[cfg(windows)]
fn enable_current() -> Result<ObservedTimerState, ConfluxTimerError> {
    let (process, sites) = resolve_process_sites()?;
    let mut memory = RemoteTimerMemory::writable(&process, sites)?;
    if memory.mode()? != ENDLESS_MODE {
        return Err(ConfluxTimerError::NotEndlessMode);
    }
    enable_timer(&mut memory)
}

#[cfg(windows)]
fn restore_current() -> Result<ObservedTimerState, ConfluxTimerError> {
    let (process, sites) = resolve_process_sites()?;
    restore_timer(&mut RemoteTimerMemory::writable(&process, sites)?)
}

trait ConfluxTimerBackend: Send + Sync {
    fn observe(&self) -> Result<ObservedTimerState, ConfluxTimerError>;
    fn enable(&self) -> Result<ObservedTimerState, ConfluxTimerError>;
    fn restore(&self) -> Result<ObservedTimerState, ConfluxTimerError>;
}

struct LiveConfluxTimerBackend;

impl ConfluxTimerBackend for LiveConfluxTimerBackend {
    fn observe(&self) -> Result<ObservedTimerState, ConfluxTimerError> {
        observe_current()
    }

    fn enable(&self) -> Result<ObservedTimerState, ConfluxTimerError> {
        enable_current()
    }

    fn restore(&self) -> Result<ObservedTimerState, ConfluxTimerError> {
        restore_current()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfluxTimerStatus {
    pub state: ConfluxTimerStatusKind,
    pub reason: Option<ConfluxTimerReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ConfluxTimerStatusKind {
    Unavailable,
    Off,
    On,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ConfluxTimerReason {
    Busy,
    GameNotRunning,
    UnsupportedGame,
    NotEndlessMode,
    UnexpectedValues,
    AccessDenied,
    PatchFailed,
    RestoreFailed,
    Internal,
}

impl ConfluxTimerStatus {
    fn busy() -> Self {
        Self {
            state: ConfluxTimerStatusKind::Unavailable,
            reason: Some(ConfluxTimerReason::Busy),
        }
    }

    fn internal() -> Self {
        Self {
            state: ConfluxTimerStatusKind::Unavailable,
            reason: Some(ConfluxTimerReason::Internal),
        }
    }

    fn observed(state: ObservedTimerState) -> Self {
        match state {
            ObservedTimerState::Off => Self {
                state: ConfluxTimerStatusKind::Off,
                reason: None,
            },
            ObservedTimerState::On => Self {
                state: ConfluxTimerStatusKind::On,
                reason: None,
            },
            ObservedTimerState::Mixed | ObservedTimerState::Unknown => Self {
                state: ConfluxTimerStatusKind::Unavailable,
                reason: Some(ConfluxTimerReason::UnexpectedValues),
            },
        }
    }

    fn error(error: &ConfluxTimerError) -> Self {
        let reason = match error {
            ConfluxTimerError::GameNotRunning => ConfluxTimerReason::GameNotRunning,
            ConfluxTimerError::UnsupportedGame => ConfluxTimerReason::UnsupportedGame,
            ConfluxTimerError::NotEndlessMode => ConfluxTimerReason::NotEndlessMode,
            ConfluxTimerError::UnexpectedValues => ConfluxTimerReason::UnexpectedValues,
            ConfluxTimerError::AccessDenied => ConfluxTimerReason::AccessDenied,
            _ => ConfluxTimerReason::Internal,
        };
        Self {
            state: ConfluxTimerStatusKind::Unavailable,
            reason: Some(reason),
        }
    }

    fn operation_error(error: &ConfluxTimerError, enabling: bool) -> ConfluxTimerReason {
        match error {
            ConfluxTimerError::GameNotRunning => ConfluxTimerReason::GameNotRunning,
            ConfluxTimerError::UnsupportedGame => ConfluxTimerReason::UnsupportedGame,
            ConfluxTimerError::NotEndlessMode => ConfluxTimerReason::NotEndlessMode,
            ConfluxTimerError::UnexpectedValues => ConfluxTimerReason::UnexpectedValues,
            ConfluxTimerError::AccessDenied => ConfluxTimerReason::AccessDenied,
            ConfluxTimerError::Rollback => ConfluxTimerReason::RestoreFailed,
            _ if enabling => ConfluxTimerReason::PatchFailed,
            _ => ConfluxTimerReason::RestoreFailed,
        }
    }
}

struct ConfluxTimerInner {
    backend: Arc<dyn ConfluxTimerBackend>,
    operation: Mutex<()>,
    may_be_patched: AtomicBool,
    cleanup_started: AtomicBool,
}

#[derive(Clone)]
pub(crate) struct ConfluxTimerState(Arc<ConfluxTimerInner>);

impl Default for ConfluxTimerState {
    fn default() -> Self {
        Self::with_backend(Arc::new(LiveConfluxTimerBackend))
    }
}

impl ConfluxTimerState {
    fn with_backend(backend: Arc<dyn ConfluxTimerBackend>) -> Self {
        Self(Arc::new(ConfluxTimerInner {
            backend,
            operation: Mutex::new(()),
            may_be_patched: AtomicBool::new(false),
            cleanup_started: AtomicBool::new(false),
        }))
    }

    fn status(&self) -> ConfluxTimerStatus {
        let Ok(_operation) = self.0.operation.lock() else {
            return ConfluxTimerStatus::internal();
        };
        match self.0.backend.observe() {
            Ok(observed) => ConfluxTimerStatus::observed(observed),
            Err(error) => ConfluxTimerStatus::error(&error),
        }
    }

    pub(crate) fn restore_on_startup(&self) {
        let Ok(_operation) = self.0.operation.lock() else {
            return;
        };
        match self.0.backend.restore() {
            Ok(ObservedTimerState::Off) | Err(ConfluxTimerError::GameNotRunning) => {
                self.0.may_be_patched.store(false, Ordering::Release);
            }
            Ok(_) | Err(_) => {
                self.0.may_be_patched.store(true, Ordering::Release);
            }
        }
    }

    fn set_enabled(&self, enabled: bool) -> ConfluxTimerStatus {
        let Ok(_operation) = self.0.operation.try_lock() else {
            return ConfluxTimerStatus::busy();
        };
        let result = if enabled {
            self.0.backend.enable()
        } else {
            self.0.backend.restore()
        };
        match result {
            Ok(observed) => {
                self.0.may_be_patched.store(
                    matches!(observed, ObservedTimerState::On | ObservedTimerState::Mixed),
                    Ordering::Release,
                );
                ConfluxTimerStatus::observed(observed)
            }
            Err(error) => {
                let reason = ConfluxTimerStatus::operation_error(&error, enabled);
                let mut status = match self.0.backend.observe() {
                    Ok(observed) => {
                        self.0.may_be_patched.store(
                            matches!(observed, ObservedTimerState::On | ObservedTimerState::Mixed),
                            Ordering::Release,
                        );
                        ConfluxTimerStatus::observed(observed)
                    }
                    Err(observe_error) => ConfluxTimerStatus::error(&observe_error),
                };
                status.reason = Some(reason);
                status
            }
        }
    }

    pub(crate) fn restore_for_update(&self) -> ConfluxTimerStatus {
        self.set_enabled(false)
    }

    pub(crate) fn restore_on_exit(&self) {
        if !self.0.may_be_patched.load(Ordering::Acquire)
            || self.0.cleanup_started.swap(true, Ordering::AcqRel)
        {
            return;
        }
        let Ok(_operation) = self.0.operation.lock() else {
            log::warn!("CONFLUX TIMER restore stage=exit-lock result=failed");
            return;
        };
        match self.0.backend.restore() {
            Ok(ObservedTimerState::Off) => {
                self.0.may_be_patched.store(false, Ordering::Release);
            }
            Ok(observed) => {
                log::warn!("CONFLUX TIMER restore stage=exit result={observed:?}");
            }
            Err(error) => {
                log::warn!("CONFLUX TIMER restore stage=exit error={error}");
            }
        }
    }

    #[cfg(test)]
    fn lock_operation_for_test(&self) -> std::sync::MutexGuard<'_, ()> {
        self.0.operation.lock().unwrap()
    }
}

#[tauri::command]
pub(crate) async fn get_conflux_timer_status(
    state: tauri::State<'_, ConfluxTimerState>,
) -> Result<ConfluxTimerStatus, ()> {
    let state = state.inner().clone();
    Ok(tauri::async_runtime::spawn_blocking(move || state.status())
        .await
        .unwrap_or_else(|_| ConfluxTimerStatus::internal()))
}

#[tauri::command]
pub(crate) async fn set_conflux_timer_enabled(
    state: tauri::State<'_, ConfluxTimerState>,
    enabled: bool,
) -> Result<ConfluxTimerStatus, ()> {
    let state = state.inner().clone();
    Ok(
        tauri::async_runtime::spawn_blocking(move || state.set_enabled(enabled))
            .await
            .unwrap_or_else(|_| ConfluxTimerStatus::internal()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    #[derive(Clone)]
    struct TestMemory {
        config: [u8; TIMER_CONFIG_BYTES],
        active: [u8; TIMER_ACTIVE_BYTES],
        fail_write: bool,
    }

    impl TestMemory {
        fn original(active: ActiveTimer) -> Self {
            Self {
                config: encode_config(ORIGINAL_TIMER_CONFIG),
                active: encode_active(active),
                fail_write: false,
            }
        }
    }

    impl TimerMemory for TestMemory {
        fn read_config(&self) -> Result<[u8; TIMER_CONFIG_BYTES], ConfluxTimerError> {
            Ok(self.config)
        }

        fn write_config(
            &mut self,
            bytes: [u8; TIMER_CONFIG_BYTES],
        ) -> Result<(), ConfluxTimerError> {
            self.config = bytes;
            Ok(())
        }

        fn read_active(&self) -> Result<[u8; TIMER_ACTIVE_BYTES], ConfluxTimerError> {
            Ok(self.active)
        }

        fn write_active(
            &mut self,
            bytes: [u8; TIMER_ACTIVE_BYTES],
        ) -> Result<(), ConfluxTimerError> {
            if self.fail_write {
                return Err(ConfluxTimerError::Write {
                    address: 0,
                    detail: "injected failure".to_owned(),
                });
            }
            self.active = bytes;
            Ok(())
        }
    }

    #[test]
    fn classifies_only_the_pinned_original_and_fast_configurations() {
        assert_eq!(
            classify_config(encode_config(ORIGINAL_TIMER_CONFIG)),
            ObservedTimerState::Off
        );
        assert_eq!(
            classify_config(encode_config(FAST_TIMER_CONFIG)),
            ObservedTimerState::On
        );
        let mut mixed = encode_config(ORIGINAL_TIMER_CONFIG);
        mixed[..4].copy_from_slice(&1.0f32.to_le_bytes());
        assert_eq!(classify_config(mixed), ObservedTimerState::Mixed);
        mixed[8..12].copy_from_slice(&99.0f32.to_le_bytes());
        assert_eq!(classify_config(mixed), ObservedTimerState::Unknown);
    }

    #[test]
    fn enable_patches_defaults_and_shortens_the_active_countdown() {
        let mut memory = TestMemory::original(ActiveTimer {
            initial_seconds: 60.0,
            current_seconds: 13.5,
        });

        assert_eq!(enable_timer(&mut memory), Ok(ObservedTimerState::On));
        assert_eq!(memory.config, encode_config(FAST_TIMER_CONFIG));
        assert_eq!(
            decode_active(memory.active),
            ActiveTimer {
                initial_seconds: 2.0,
                current_seconds: 2.0,
            }
        );
    }

    #[test]
    fn enable_does_not_extend_a_countdown_already_below_two_seconds() {
        let mut memory = TestMemory::original(ActiveTimer {
            initial_seconds: 60.0,
            current_seconds: 0.75,
        });

        assert_eq!(enable_timer(&mut memory), Ok(ObservedTimerState::On));
        assert_eq!(decode_active(memory.active).current_seconds, 0.75);
    }

    #[test]
    fn restore_reinstates_the_pinned_original_defaults() {
        let mut memory = TestMemory {
            config: encode_config(FAST_TIMER_CONFIG),
            active: encode_active(ActiveTimer {
                initial_seconds: 2.0,
                current_seconds: 1.0,
            }),
            fail_write: false,
        };

        assert_eq!(restore_timer(&mut memory), Ok(ObservedTimerState::Off));
        assert_eq!(memory.config, encode_config(ORIGINAL_TIMER_CONFIG));
    }

    struct TestBackend {
        observed: Mutex<ObservedTimerState>,
        enables: AtomicUsize,
        restores: AtomicUsize,
    }

    impl ConfluxTimerBackend for TestBackend {
        fn observe(&self) -> Result<ObservedTimerState, ConfluxTimerError> {
            Ok(*self.observed.lock().unwrap())
        }

        fn enable(&self) -> Result<ObservedTimerState, ConfluxTimerError> {
            self.enables.fetch_add(1, AtomicOrdering::Relaxed);
            *self.observed.lock().unwrap() = ObservedTimerState::On;
            Ok(ObservedTimerState::On)
        }

        fn restore(&self) -> Result<ObservedTimerState, ConfluxTimerError> {
            self.restores.fetch_add(1, AtomicOrdering::Relaxed);
            *self.observed.lock().unwrap() = ObservedTimerState::Off;
            Ok(ObservedTimerState::Off)
        }
    }

    #[test]
    fn state_starts_off_and_serializes_mutations() {
        let backend = Arc::new(TestBackend {
            observed: Mutex::new(ObservedTimerState::Off),
            enables: AtomicUsize::new(0),
            restores: AtomicUsize::new(0),
        });
        let state = ConfluxTimerState::with_backend(backend.clone());

        assert_eq!(state.status().state, ConfluxTimerStatusKind::Off);
        let guard = state.lock_operation_for_test();
        assert_eq!(
            state.set_enabled(true),
            ConfluxTimerStatus {
                state: ConfluxTimerStatusKind::Unavailable,
                reason: Some(ConfluxTimerReason::Busy),
            }
        );
        drop(guard);
        assert_eq!(state.set_enabled(true).state, ConfluxTimerStatusKind::On);
        assert_eq!(backend.enables.load(AtomicOrdering::Relaxed), 1);
    }

    #[test]
    fn accepts_only_the_pinned_game_hash() {
        let pinned = parse_sha256(PINNED_CONFLUX_TIMER_SHA256).unwrap();
        assert_eq!(verify_game_hash(&pinned), Ok(()));
        assert_eq!(
            verify_game_hash(&[0; 32]),
            Err(ConfluxTimerError::UnsupportedGame)
        );
    }

    #[test]
    #[ignore = "writes only the pinned timer fields in an offline/private live session"]
    fn live_timer_patch_round_trip() {
        assert_eq!(
            std::env::var("DJEETA_CONFLUX_TIMER_WRITE_TEST")
                .ok()
                .as_deref(),
            Some("1"),
            "explicit live-write opt-in is required"
        );

        let result = (|| {
            assert_eq!(restore_current()?, ObservedTimerState::Off);
            assert_eq!(enable_current()?, ObservedTimerState::On);
            let (process, sites) = resolve_process_sites()?;
            let memory = RemoteTimerMemory::read_only(&process, sites);
            assert_eq!(
                classify_config(memory.read_config()?),
                ObservedTimerState::On
            );
            let active = decode_active(memory.read_active()?);
            assert!(active.initial_seconds <= FAST_PROGRESS_SECONDS);
            assert!(active.current_seconds <= FAST_PROGRESS_SECONDS);
            Ok::<_, ConfluxTimerError>(())
        })();

        let restore = restore_current();
        assert_eq!(restore, Ok(ObservedTimerState::Off));
        result.expect("live timer patch must apply and read back");
    }
}
