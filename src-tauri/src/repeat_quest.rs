use crate::equipment_probe::{
    memory::{MemoryReadError, MemoryReader, RemoteProcess},
    GAME_PROCESS_NAME, PINNED_GAME_SHA256,
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
        Diagnostics::Debug::{FlushInstructionCache, WriteProcessMemory},
        Memory::{VirtualProtectEx, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS},
        Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ,
            PROCESS_VM_WRITE,
        },
    },
};

const RESET_PREFIX: &[u8] = &[
    0x48, 0x83, 0xB8, 0x08, 0xC1, 0x01, 0x00, 0x00, 0xC7, 0x80, 0x24, 0xC1, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x0F, 0x84,
];
const RESET_SUFFIX: &[u8] = &[
    0xC6, 0x87, 0x24, 0x06, 0x00, 0x00, 0x00, 0x48, 0x8D, 0x8F, 0x28, 0x06, 0x00, 0x00, 0x31, 0xD2,
    0x45, 0x31, 0xC0, 0x44, 0x89, 0x01, 0x85, 0xDB, 0x75, 0x15,
];
const GETTER_PREFIX: &[u8] = &[
    0x48, 0x83, 0xC1, 0x15, 0xEB, 0x0C, 0xB9, 0x24, 0x06, 0x00, 0x00, 0x48, 0x03, 0x0D,
];
const GETTER_SUFFIX: &[u8] = &[0x0F, 0xB6, 0x01, 0x48, 0x83, 0xC4, 0x20];
const SIGNATURE_WILDCARD_BYTES: usize = 4;
const RESET_PATCH_OFFSET: usize = 0x28;
const GETTER_PATCH_OFFSET: usize = 0x12;
const RESET_ORIGINAL: [u8; 3] = [0x45, 0x31, 0xC0];
const RESET_PATCHED: [u8; 3] = [0x44, 0x8B, 0x01];
const GETTER_ORIGINAL: [u8; 3] = [0x0F, 0xB6, 0x01];
const GETTER_PATCHED: [u8; 3] = [0xB0, 0x01, 0x90];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PatchSiteName {
    Reset,
    Getter,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum RepeatQuestError {
    #[error("game is not running")]
    GameNotRunning,
    #[error("unsupported game executable")]
    UnsupportedGame,
    #[error("{site:?} signature count was {count}")]
    SignatureCount { site: PatchSiteName, count: usize },
    #[error("patch address overflow")]
    AddressOverflow,
    #[error("target bytes are neither original nor patched")]
    UnexpectedBytes,
    #[error("read failed at {address:#x}: {detail}")]
    Read { address: usize, detail: String },
    #[error("write failed at {address:#x}: {detail}")]
    Write { address: usize, detail: String },
    #[error("write returned {actual} of {expected} bytes")]
    PartialWrite { expected: usize, actual: usize },
    #[error("page protection failed at {address:#x}: {detail}")]
    Protection { address: usize, detail: String },
    #[error("write and page-protection restoration both failed at {address:#x}")]
    WriteAndProtectionRestore { address: usize },
    #[error("instruction-cache flush failed at {address:#x}: {detail}")]
    Flush { address: usize, detail: String },
    #[error("final byte read-back did not match the requested state")]
    ReadBackMismatch,
    #[error("enable failed and rollback did not restore OFF")]
    Rollback,
    #[error("pinned SHA-256 constant is invalid")]
    InvalidPinnedHash,
    #[error("process access denied")]
    AccessDenied,
    #[error("process operation failed: {0}")]
    Process(String),
}

fn parse_sha256(value: &str) -> Result<[u8; 32], RepeatQuestError> {
    if value.len() != 64 || !value.is_ascii() {
        return Err(RepeatQuestError::InvalidPinnedHash);
    }
    let mut output = [0u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16)
            .map_err(|_| RepeatQuestError::InvalidPinnedHash)?;
    }
    Ok(output)
}

fn verify_game_hash(actual: &[u8; 32]) -> Result<(), RepeatQuestError> {
    if *actual == parse_sha256(PINNED_GAME_SHA256)? {
        Ok(())
    } else {
        Err(RepeatQuestError::UnsupportedGame)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PatchOffsets {
    reset: usize,
    getter: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PatchSites {
    reset: usize,
    getter: usize,
}

fn patch_sites(text_base: usize, offsets: PatchOffsets) -> Result<PatchSites, RepeatQuestError> {
    Ok(PatchSites {
        reset: text_base
            .checked_add(offsets.reset)
            .ok_or(RepeatQuestError::AddressOverflow)?,
        getter: text_base
            .checked_add(offsets.getter)
            .ok_or(RepeatQuestError::AddressOverflow)?,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SiteBytes {
    Original,
    Patched,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ObservedPatchState {
    Off,
    On,
    Mixed,
    Unknown,
}

fn classify_site(bytes: [u8; 3], original: [u8; 3], patched: [u8; 3]) -> SiteBytes {
    if bytes == original {
        SiteBytes::Original
    } else if bytes == patched {
        SiteBytes::Patched
    } else {
        SiteBytes::Unknown
    }
}

fn classify_pair(reset: [u8; 3], getter: [u8; 3]) -> ObservedPatchState {
    match (
        classify_site(reset, RESET_ORIGINAL, RESET_PATCHED),
        classify_site(getter, GETTER_ORIGINAL, GETTER_PATCHED),
    ) {
        (SiteBytes::Original, SiteBytes::Original) => ObservedPatchState::Off,
        (SiteBytes::Patched, SiteBytes::Patched) => ObservedPatchState::On,
        (SiteBytes::Unknown, _) | (_, SiteBytes::Unknown) => ObservedPatchState::Unknown,
        _ => ObservedPatchState::Mixed,
    }
}

trait PatchMemory {
    fn read_site(&self, address: usize) -> Result<[u8; 3], RepeatQuestError>;
    fn write_site(&mut self, address: usize, bytes: [u8; 3]) -> Result<(), RepeatQuestError>;
}

fn observe_patch(
    memory: &impl PatchMemory,
    sites: PatchSites,
) -> Result<ObservedPatchState, RepeatQuestError> {
    Ok(classify_pair(
        memory.read_site(sites.reset)?,
        memory.read_site(sites.getter)?,
    ))
}

fn enable_patch(
    memory: &mut impl PatchMemory,
    sites: PatchSites,
) -> Result<ObservedPatchState, RepeatQuestError> {
    match observe_patch(memory, sites)? {
        ObservedPatchState::On => return Ok(ObservedPatchState::On),
        ObservedPatchState::Off => {}
        ObservedPatchState::Mixed | ObservedPatchState::Unknown => {
            return Err(RepeatQuestError::UnexpectedBytes)
        }
    }
    if let Err(error) = memory.write_site(sites.reset, RESET_PATCHED) {
        rollback_enable(memory, sites)?;
        return Err(error);
    }
    if let Err(error) = memory.write_site(sites.getter, GETTER_PATCHED) {
        rollback_enable(memory, sites)?;
        return Err(error);
    }
    let observed = observe_patch(memory, sites)?;
    if observed == ObservedPatchState::On {
        Ok(observed)
    } else {
        rollback_enable(memory, sites)?;
        Err(RepeatQuestError::ReadBackMismatch)
    }
}

fn rollback_enable(
    memory: &mut impl PatchMemory,
    sites: PatchSites,
) -> Result<(), RepeatQuestError> {
    match restore_patch(memory, sites) {
        Ok(ObservedPatchState::Off) => Ok(()),
        _ => Err(RepeatQuestError::Rollback),
    }
}

fn restore_patch(
    memory: &mut impl PatchMemory,
    sites: PatchSites,
) -> Result<ObservedPatchState, RepeatQuestError> {
    let reset = classify_site(
        memory.read_site(sites.reset)?,
        RESET_ORIGINAL,
        RESET_PATCHED,
    );
    let getter = classify_site(
        memory.read_site(sites.getter)?,
        GETTER_ORIGINAL,
        GETTER_PATCHED,
    );
    if reset == SiteBytes::Unknown || getter == SiteBytes::Unknown {
        return Err(RepeatQuestError::UnexpectedBytes);
    }
    if reset == SiteBytes::Patched {
        memory.write_site(sites.reset, RESET_ORIGINAL)?;
    }
    if getter == SiteBytes::Patched {
        memory.write_site(sites.getter, GETTER_ORIGINAL)?;
    }
    let observed = observe_patch(memory, sites)?;
    if observed == ObservedPatchState::Off {
        Ok(observed)
    } else {
        Err(RepeatQuestError::ReadBackMismatch)
    }
}

#[cfg(windows)]
fn map_memory_error(error: MemoryReadError, address: usize) -> RepeatQuestError {
    let detail = error.to_string();
    if detail.contains("0x80070005") {
        RepeatQuestError::AccessDenied
    } else {
        RepeatQuestError::Read { address, detail }
    }
}

#[cfg(windows)]
fn resolve_process_sites() -> Result<(RemoteProcess, PatchSites), RepeatQuestError> {
    let process = RemoteProcess::find(GAME_PROCESS_NAME)
        .map_err(|error| map_memory_error(error, 0))?
        .ok_or(RepeatQuestError::GameNotRunning)?;
    let hash = process
        .executable_sha256()
        .map_err(|error| map_memory_error(error, process.module_base))?;
    verify_game_hash(&hash)?;
    let (text_base, text) = process
        .read_text_section()
        .map_err(|error| map_memory_error(error, process.module_base))?;
    let sites = patch_sites(text_base, find_patch_offsets(&text)?)?;
    if !process
        .is_running()
        .map_err(|error| map_memory_error(error, process.module_base))?
    {
        return Err(RepeatQuestError::GameNotRunning);
    }
    Ok((process, sites))
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
struct WritablePatchMemory<'a> {
    handle: WritableHandle,
    reader: &'a RemoteProcess,
}

#[cfg(windows)]
impl<'a> WritablePatchMemory<'a> {
    fn open(reader: &'a RemoteProcess) -> Result<Self, RepeatQuestError> {
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
            handle: WritableHandle(handle),
            reader,
        })
    }

    fn write_protected_once(&self, address: usize, bytes: [u8; 3]) -> Result<(), RepeatQuestError> {
        let mut old = PAGE_PROTECTION_FLAGS::default();
        unsafe {
            VirtualProtectEx(
                self.handle.0,
                address as *const c_void,
                bytes.len(),
                PAGE_EXECUTE_READWRITE,
                &mut old,
            )
        }
        .map_err(|error| RepeatQuestError::Protection {
            address,
            detail: error.to_string(),
        })?;

        let mut written = 0usize;
        let write_result = unsafe {
            WriteProcessMemory(
                self.handle.0,
                address as *const c_void,
                bytes.as_ptr().cast::<c_void>(),
                bytes.len(),
                Some(&mut written),
            )
        };
        let flush_result = if write_result.is_ok() || written > 0 {
            unsafe {
                FlushInstructionCache(self.handle.0, Some(address as *const c_void), bytes.len())
            }
        } else {
            Ok(())
        };

        let mut ignored = PAGE_PROTECTION_FLAGS::default();
        let mut restore_result = unsafe {
            VirtualProtectEx(
                self.handle.0,
                address as *const c_void,
                bytes.len(),
                old,
                &mut ignored,
            )
        };
        if restore_result.is_err() {
            restore_result = unsafe {
                VirtualProtectEx(
                    self.handle.0,
                    address as *const c_void,
                    bytes.len(),
                    old,
                    &mut ignored,
                )
            };
        }

        match (write_result, restore_result) {
            (Err(_), Err(_)) => {
                return Err(RepeatQuestError::WriteAndProtectionRestore { address })
            }
            (Err(error), Ok(())) => {
                return Err(RepeatQuestError::Write {
                    address,
                    detail: error.to_string(),
                })
            }
            (Ok(()), Err(error)) => {
                return Err(RepeatQuestError::Protection {
                    address,
                    detail: error.to_string(),
                })
            }
            (Ok(()), Ok(())) => {}
        }
        if written != bytes.len() {
            return Err(RepeatQuestError::PartialWrite {
                expected: bytes.len(),
                actual: written,
            });
        }
        flush_result.map_err(|error| RepeatQuestError::Flush {
            address,
            detail: error.to_string(),
        })?;
        Ok(())
    }

    fn restore_previous(&self, address: usize, previous: [u8; 3]) -> Result<(), RepeatQuestError> {
        self.write_protected_once(address, previous)?;
        if self.read_site(address)? == previous {
            Ok(())
        } else {
            Err(RepeatQuestError::Rollback)
        }
    }
}

#[cfg(windows)]
impl PatchMemory for WritablePatchMemory<'_> {
    fn read_site(&self, address: usize) -> Result<[u8; 3], RepeatQuestError> {
        let mut bytes = [0u8; 3];
        self.reader
            .read_exact(address, &mut bytes)
            .map_err(|error| map_memory_error(error, address))?;
        Ok(bytes)
    }

    fn write_site(&mut self, address: usize, bytes: [u8; 3]) -> Result<(), RepeatQuestError> {
        let previous = self.read_site(address)?;
        if let Err(error) = self.write_protected_once(address, bytes) {
            if self.read_site(address)? != previous {
                self.restore_previous(address, previous)
                    .map_err(|_| RepeatQuestError::Rollback)?;
            }
            return Err(error);
        }
        if self.read_site(address)? == bytes {
            Ok(())
        } else {
            self.restore_previous(address, previous)?;
            Err(RepeatQuestError::ReadBackMismatch)
        }
    }
}

#[cfg(windows)]
fn map_open_error(error: windows::core::Error) -> RepeatQuestError {
    if error.code() == ERROR_ACCESS_DENIED.to_hresult() {
        RepeatQuestError::AccessDenied
    } else {
        RepeatQuestError::Process(error.to_string())
    }
}

#[cfg(windows)]
fn observe_current() -> Result<ObservedPatchState, RepeatQuestError> {
    let (process, sites) = resolve_process_sites()?;
    observe_patch(&RemotePatchReader(&process), sites)
}

#[cfg(windows)]
fn enable_current() -> Result<ObservedPatchState, RepeatQuestError> {
    let (process, sites) = resolve_process_sites()?;
    let mut memory = WritablePatchMemory::open(&process)?;
    enable_patch(&mut memory, sites)
}

#[cfg(windows)]
fn restore_current() -> Result<ObservedPatchState, RepeatQuestError> {
    let (process, sites) = resolve_process_sites()?;
    let mut memory = WritablePatchMemory::open(&process)?;
    restore_patch(&mut memory, sites)
}

trait RepeatQuestBackend: Send + Sync {
    fn observe(&self) -> Result<ObservedPatchState, RepeatQuestError>;
    fn enable(&self) -> Result<ObservedPatchState, RepeatQuestError>;
    fn restore(&self) -> Result<ObservedPatchState, RepeatQuestError>;
}

struct LiveRepeatQuestBackend;

impl RepeatQuestBackend for LiveRepeatQuestBackend {
    fn observe(&self) -> Result<ObservedPatchState, RepeatQuestError> {
        observe_current()
    }

    fn enable(&self) -> Result<ObservedPatchState, RepeatQuestError> {
        enable_current()
    }

    fn restore(&self) -> Result<ObservedPatchState, RepeatQuestError> {
        restore_current()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepeatQuestStatus {
    pub state: RepeatQuestStatusKind,
    pub reason: Option<RepeatQuestReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RepeatQuestStatusKind {
    Unavailable,
    Off,
    On,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RepeatQuestReason {
    Busy,
    GameNotRunning,
    UnsupportedGame,
    SignatureMissing,
    SignatureAmbiguous,
    UnexpectedBytes,
    AccessDenied,
    PatchFailed,
    RestoreFailed,
    Internal,
}

impl RepeatQuestStatus {
    fn busy() -> Self {
        Self {
            state: RepeatQuestStatusKind::Unavailable,
            reason: Some(RepeatQuestReason::Busy),
        }
    }

    fn internal() -> Self {
        Self {
            state: RepeatQuestStatusKind::Unavailable,
            reason: Some(RepeatQuestReason::Internal),
        }
    }

    fn observed(state: ObservedPatchState) -> Self {
        match state {
            ObservedPatchState::Off => Self {
                state: RepeatQuestStatusKind::Off,
                reason: None,
            },
            ObservedPatchState::On => Self {
                state: RepeatQuestStatusKind::On,
                reason: None,
            },
            ObservedPatchState::Mixed | ObservedPatchState::Unknown => Self {
                state: RepeatQuestStatusKind::Unavailable,
                reason: Some(RepeatQuestReason::UnexpectedBytes),
            },
        }
    }

    fn error(error: &RepeatQuestError) -> Self {
        let reason = match error {
            RepeatQuestError::GameNotRunning => RepeatQuestReason::GameNotRunning,
            RepeatQuestError::UnsupportedGame => RepeatQuestReason::UnsupportedGame,
            RepeatQuestError::SignatureCount { count: 0, .. } => {
                RepeatQuestReason::SignatureMissing
            }
            RepeatQuestError::SignatureCount { .. } => RepeatQuestReason::SignatureAmbiguous,
            RepeatQuestError::UnexpectedBytes => RepeatQuestReason::UnexpectedBytes,
            RepeatQuestError::AccessDenied => RepeatQuestReason::AccessDenied,
            _ => RepeatQuestReason::Internal,
        };
        Self {
            state: RepeatQuestStatusKind::Unavailable,
            reason: Some(reason),
        }
    }

    fn operation_error(error: &RepeatQuestError, enabling: bool) -> RepeatQuestReason {
        match error {
            RepeatQuestError::GameNotRunning => RepeatQuestReason::GameNotRunning,
            RepeatQuestError::UnsupportedGame => RepeatQuestReason::UnsupportedGame,
            RepeatQuestError::SignatureCount { count: 0, .. } => {
                RepeatQuestReason::SignatureMissing
            }
            RepeatQuestError::SignatureCount { .. } => RepeatQuestReason::SignatureAmbiguous,
            RepeatQuestError::UnexpectedBytes => RepeatQuestReason::UnexpectedBytes,
            RepeatQuestError::AccessDenied => RepeatQuestReason::AccessDenied,
            RepeatQuestError::Rollback => RepeatQuestReason::RestoreFailed,
            _ if enabling => RepeatQuestReason::PatchFailed,
            _ => RepeatQuestReason::RestoreFailed,
        }
    }
}

struct RepeatQuestInner {
    backend: Arc<dyn RepeatQuestBackend>,
    operation: Mutex<()>,
    may_be_patched: AtomicBool,
    cleanup_started: AtomicBool,
}

#[derive(Clone)]
pub(crate) struct RepeatQuestState(Arc<RepeatQuestInner>);

impl Default for RepeatQuestState {
    fn default() -> Self {
        Self::with_backend(Arc::new(LiveRepeatQuestBackend))
    }
}

impl RepeatQuestState {
    fn with_backend(backend: Arc<dyn RepeatQuestBackend>) -> Self {
        Self(Arc::new(RepeatQuestInner {
            backend,
            operation: Mutex::new(()),
            may_be_patched: AtomicBool::new(false),
            cleanup_started: AtomicBool::new(false),
        }))
    }

    fn status(&self) -> RepeatQuestStatus {
        let Ok(_operation) = self.0.operation.lock() else {
            return RepeatQuestStatus::internal();
        };
        match self.0.backend.observe() {
            Ok(observed) => RepeatQuestStatus::observed(observed),
            Err(error) => RepeatQuestStatus::error(&error),
        }
    }

    pub(crate) fn restore_on_startup(&self) {
        let Ok(_operation) = self.0.operation.lock() else {
            return;
        };
        match self.0.backend.restore() {
            Ok(ObservedPatchState::Off) | Err(RepeatQuestError::GameNotRunning) => {
                self.0.may_be_patched.store(false, Ordering::Release);
            }
            Ok(_) | Err(_) => {
                self.0.may_be_patched.store(true, Ordering::Release);
            }
        }
    }

    fn set_enabled(&self, enabled: bool) -> RepeatQuestStatus {
        let Ok(_operation) = self.0.operation.try_lock() else {
            return RepeatQuestStatus::busy();
        };
        let result = if enabled {
            self.0.backend.enable()
        } else {
            self.0.backend.restore()
        };
        match result {
            Ok(observed) => {
                self.0.may_be_patched.store(
                    matches!(observed, ObservedPatchState::On | ObservedPatchState::Mixed),
                    Ordering::Release,
                );
                RepeatQuestStatus::observed(observed)
            }
            Err(error) => {
                let reason = RepeatQuestStatus::operation_error(&error, enabled);
                let mut status = match self.0.backend.observe() {
                    Ok(observed) => {
                        self.0.may_be_patched.store(
                            matches!(observed, ObservedPatchState::On | ObservedPatchState::Mixed),
                            Ordering::Release,
                        );
                        RepeatQuestStatus::observed(observed)
                    }
                    Err(observe_error) => RepeatQuestStatus::error(&observe_error),
                };
                status.reason = Some(reason);
                status
            }
        }
    }

    pub(crate) fn restore_on_exit(&self) {
        if !self.0.may_be_patched.load(Ordering::Acquire)
            || self.0.cleanup_started.swap(true, Ordering::AcqRel)
        {
            return;
        }
        let Ok(_operation) = self.0.operation.lock() else {
            log::warn!("REPEAT QUEST restore stage=exit-lock result=failed");
            return;
        };
        match self.0.backend.restore() {
            Ok(ObservedPatchState::Off) => {
                self.0.may_be_patched.store(false, Ordering::Release);
            }
            Ok(observed) => {
                log::warn!("REPEAT QUEST restore stage=exit result={observed:?}");
            }
            Err(error) => {
                log::warn!("REPEAT QUEST restore stage=exit error={error}");
            }
        }
    }

    #[cfg(test)]
    fn lock_operation_for_test(&self) -> std::sync::MutexGuard<'_, ()> {
        self.0.operation.lock().unwrap()
    }
}

#[tauri::command]
pub(crate) async fn get_repeat_quest_status(
    state: tauri::State<'_, RepeatQuestState>,
) -> Result<RepeatQuestStatus, ()> {
    let state = state.inner().clone();
    Ok(tauri::async_runtime::spawn_blocking(move || state.status())
        .await
        .unwrap_or_else(|_| RepeatQuestStatus::internal()))
}

#[tauri::command]
pub(crate) async fn set_repeat_quest_enabled(
    state: tauri::State<'_, RepeatQuestState>,
    enabled: bool,
) -> Result<RepeatQuestStatus, ()> {
    let state = state.inner().clone();
    Ok(
        tauri::async_runtime::spawn_blocking(move || state.set_enabled(enabled))
            .await
            .unwrap_or_else(|_| RepeatQuestStatus::internal()),
    )
}

#[cfg(windows)]
struct RemotePatchReader<'a>(&'a RemoteProcess);

#[cfg(windows)]
impl PatchMemory for RemotePatchReader<'_> {
    fn read_site(&self, address: usize) -> Result<[u8; 3], RepeatQuestError> {
        let mut bytes = [0u8; 3];
        self.0
            .read_exact(address, &mut bytes)
            .map_err(|error| map_memory_error(error, address))?;
        Ok(bytes)
    }

    fn write_site(&mut self, address: usize, _bytes: [u8; 3]) -> Result<(), RepeatQuestError> {
        Err(RepeatQuestError::Write {
            address,
            detail: "read-only process adapter".to_owned(),
        })
    }
}

fn unique_signature_offset(
    text: &[u8],
    site: PatchSiteName,
    prefix: &[u8],
    suffix: &[u8],
) -> Result<usize, RepeatQuestError> {
    let signature_len = prefix
        .len()
        .checked_add(SIGNATURE_WILDCARD_BYTES)
        .and_then(|len| len.checked_add(suffix.len()))
        .ok_or(RepeatQuestError::AddressOverflow)?;
    let mut found = None;
    let mut count = 0;
    for (offset, window) in text.windows(signature_len).enumerate() {
        if window.get(..prefix.len()) == Some(prefix)
            && window.get(prefix.len() + SIGNATURE_WILDCARD_BYTES..) == Some(suffix)
        {
            found = Some(offset);
            count += 1;
        }
    }
    if count == 1 {
        Ok(found.expect("one signature match has an offset"))
    } else {
        Err(RepeatQuestError::SignatureCount { site, count })
    }
}

fn find_patch_offsets(text: &[u8]) -> Result<PatchOffsets, RepeatQuestError> {
    let reset = unique_signature_offset(text, PatchSiteName::Reset, RESET_PREFIX, RESET_SUFFIX)?
        .checked_add(RESET_PATCH_OFFSET)
        .ok_or(RepeatQuestError::AddressOverflow)?;
    let getter =
        unique_signature_offset(text, PatchSiteName::Getter, GETTER_PREFIX, GETTER_SUFFIX)?
            .checked_add(GETTER_PATCH_OFFSET)
            .ok_or(RepeatQuestError::AddressOverflow)?;
    Ok(PatchOffsets { reset, getter })
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
    };

    const RESET_SIGNATURE: &[u8] = &[
        0x48, 0x83, 0xB8, 0x08, 0xC1, 0x01, 0x00, 0x00, 0xC7, 0x80, 0x24, 0xC1, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x0F, 0x84, 0x11, 0x22, 0x33, 0x44, 0xC6, 0x87, 0x24, 0x06, 0x00, 0x00,
        0x00, 0x48, 0x8D, 0x8F, 0x28, 0x06, 0x00, 0x00, 0x31, 0xD2, 0x45, 0x31, 0xC0, 0x44, 0x89,
        0x01, 0x85, 0xDB, 0x75, 0x15,
    ];
    const GETTER_SIGNATURE: &[u8] = &[
        0x48, 0x83, 0xC1, 0x15, 0xEB, 0x0C, 0xB9, 0x24, 0x06, 0x00, 0x00, 0x48, 0x03, 0x0D, 0x55,
        0x66, 0x77, 0x88, 0x0F, 0xB6, 0x01, 0x48, 0x83, 0xC4, 0x20,
    ];

    fn signature_fixture_with_counts(reset_count: usize, getter_count: usize) -> Vec<u8> {
        let mut text = vec![0u8; 0x700];
        for offset in [0x100, 0x300].into_iter().take(reset_count) {
            text[offset..offset + RESET_SIGNATURE.len()].copy_from_slice(RESET_SIGNATURE);
        }
        for offset in [0x200, 0x500].into_iter().take(getter_count) {
            text[offset..offset + GETTER_SIGNATURE.len()].copy_from_slice(GETTER_SIGNATURE);
        }
        text
    }

    fn signature_fixture() -> Vec<u8> {
        signature_fixture_with_counts(1, 1)
    }

    #[derive(Default)]
    struct FakePatchMemory {
        bytes: HashMap<usize, [u8; 3]>,
        writes: Vec<(usize, [u8; 3])>,
        fail_write_once_at: Option<usize>,
        fail_write_value_once: Option<(usize, [u8; 3])>,
        ignore_write_once_at: Option<usize>,
    }

    impl FakePatchMemory {
        fn original(sites: super::PatchSites) -> Self {
            Self {
                bytes: HashMap::from([
                    (sites.reset, super::RESET_ORIGINAL),
                    (sites.getter, super::GETTER_ORIGINAL),
                ]),
                writes: Vec::new(),
                fail_write_once_at: None,
                fail_write_value_once: None,
                ignore_write_once_at: None,
            }
        }

        fn bytes_at(&self, address: usize) -> [u8; 3] {
            self.bytes[&address]
        }

        fn set(&mut self, address: usize, bytes: [u8; 3]) {
            self.bytes.insert(address, bytes);
        }
    }

    impl super::PatchMemory for FakePatchMemory {
        fn read_site(&self, address: usize) -> Result<[u8; 3], super::RepeatQuestError> {
            self.bytes
                .get(&address)
                .copied()
                .ok_or_else(|| super::RepeatQuestError::Read {
                    address,
                    detail: "missing fake site".to_owned(),
                })
        }

        fn write_site(
            &mut self,
            address: usize,
            bytes: [u8; 3],
        ) -> Result<(), super::RepeatQuestError> {
            if self.fail_write_value_once == Some((address, bytes)) {
                self.fail_write_value_once = None;
                return Err(super::RepeatQuestError::Write {
                    address,
                    detail: "injected value failure".to_owned(),
                });
            }
            if self.fail_write_once_at == Some(address) {
                self.fail_write_once_at = None;
                return Err(super::RepeatQuestError::Write {
                    address,
                    detail: "injected failure".to_owned(),
                });
            }
            self.writes.push((address, bytes));
            if self.ignore_write_once_at == Some(address) {
                self.ignore_write_once_at = None;
                return Ok(());
            }
            self.bytes.insert(address, bytes);
            Ok(())
        }
    }

    struct FakeBackend {
        observed: Mutex<super::ObservedPatchState>,
        enable_calls: AtomicUsize,
        restore_calls: AtomicUsize,
    }

    impl FakeBackend {
        fn patched() -> Self {
            Self {
                observed: Mutex::new(super::ObservedPatchState::On),
                enable_calls: AtomicUsize::new(0),
                restore_calls: AtomicUsize::new(0),
            }
        }

        fn original() -> Self {
            Self {
                observed: Mutex::new(super::ObservedPatchState::Off),
                enable_calls: AtomicUsize::new(0),
                restore_calls: AtomicUsize::new(0),
            }
        }

        fn restore_calls(&self) -> usize {
            self.restore_calls.load(Ordering::Acquire)
        }
    }

    impl super::RepeatQuestBackend for FakeBackend {
        fn observe(&self) -> Result<super::ObservedPatchState, super::RepeatQuestError> {
            Ok(*self.observed.lock().unwrap())
        }

        fn enable(&self) -> Result<super::ObservedPatchState, super::RepeatQuestError> {
            self.enable_calls.fetch_add(1, Ordering::AcqRel);
            *self.observed.lock().unwrap() = super::ObservedPatchState::On;
            Ok(super::ObservedPatchState::On)
        }

        fn restore(&self) -> Result<super::ObservedPatchState, super::RepeatQuestError> {
            self.restore_calls.fetch_add(1, Ordering::AcqRel);
            *self.observed.lock().unwrap() = super::ObservedPatchState::Off;
            Ok(super::ObservedPatchState::Off)
        }
    }

    #[test]
    fn finds_each_repeat_quest_signature_once() {
        assert_eq!(
            super::find_patch_offsets(&signature_fixture()).unwrap(),
            super::PatchOffsets {
                reset: 0x128,
                getter: 0x212,
            }
        );
    }

    #[test]
    fn rejects_missing_or_duplicate_signatures() {
        assert_eq!(
            super::find_patch_offsets(&signature_fixture_with_counts(0, 1)),
            Err(super::RepeatQuestError::SignatureCount {
                site: super::PatchSiteName::Reset,
                count: 0,
            })
        );
        assert_eq!(
            super::find_patch_offsets(&signature_fixture_with_counts(1, 2)),
            Err(super::RepeatQuestError::SignatureCount {
                site: super::PatchSiteName::Getter,
                count: 2,
            })
        );
    }

    #[test]
    fn classifies_original_patched_mixed_and_unknown_bytes() {
        assert_eq!(
            super::classify_pair(super::RESET_ORIGINAL, super::GETTER_ORIGINAL),
            super::ObservedPatchState::Off
        );
        assert_eq!(
            super::classify_pair(super::RESET_PATCHED, super::GETTER_PATCHED),
            super::ObservedPatchState::On
        );
        assert_eq!(
            super::classify_pair(super::RESET_PATCHED, super::GETTER_ORIGINAL),
            super::ObservedPatchState::Mixed
        );
        assert_eq!(
            super::classify_pair([0x90; 3], super::GETTER_ORIGINAL),
            super::ObservedPatchState::Unknown
        );
    }

    #[test]
    fn enable_writes_both_sites_and_verifies_on() {
        let sites = super::PatchSites {
            reset: 0x1000,
            getter: 0x2000,
        };
        let mut memory = FakePatchMemory::original(sites);

        assert_eq!(
            super::enable_patch(&mut memory, sites).unwrap(),
            super::ObservedPatchState::On
        );
        assert_eq!(
            memory.writes,
            vec![
                (sites.reset, super::RESET_PATCHED),
                (sites.getter, super::GETTER_PATCHED),
            ]
        );
        assert_eq!(memory.bytes_at(sites.reset), super::RESET_PATCHED);
        assert_eq!(memory.bytes_at(sites.getter), super::GETTER_PATCHED);
    }

    #[test]
    fn second_enable_write_failure_rolls_back_the_first() {
        let sites = super::PatchSites {
            reset: 0x1000,
            getter: 0x2000,
        };
        let mut memory = FakePatchMemory::original(sites);
        memory.fail_write_once_at = Some(sites.getter);

        assert!(matches!(
            super::enable_patch(&mut memory, sites),
            Err(super::RepeatQuestError::Write {
                address: 0x2000,
                ..
            })
        ));
        assert_eq!(memory.bytes_at(sites.reset), super::RESET_ORIGINAL);
        assert_eq!(memory.bytes_at(sites.getter), super::GETTER_ORIGINAL);
    }

    #[test]
    fn restore_repairs_mixed_state_without_touching_original_site() {
        let sites = super::PatchSites {
            reset: 0x1000,
            getter: 0x2000,
        };
        let mut memory = FakePatchMemory::original(sites);
        memory.set(sites.reset, super::RESET_PATCHED);

        assert_eq!(
            super::restore_patch(&mut memory, sites).unwrap(),
            super::ObservedPatchState::Off
        );
        assert_eq!(memory.writes, vec![(sites.reset, super::RESET_ORIGINAL)]);
    }

    #[test]
    fn read_back_mismatch_restores_off_before_returning_error() {
        let sites = super::PatchSites {
            reset: 0x1000,
            getter: 0x2000,
        };
        let mut memory = FakePatchMemory::original(sites);
        memory.ignore_write_once_at = Some(sites.getter);

        assert_eq!(
            super::enable_patch(&mut memory, sites),
            Err(super::RepeatQuestError::ReadBackMismatch)
        );
        assert_eq!(memory.bytes_at(sites.reset), super::RESET_ORIGINAL);
        assert_eq!(memory.bytes_at(sites.getter), super::GETTER_ORIGINAL);
    }

    #[test]
    fn unknown_bytes_are_never_overwritten() {
        let sites = super::PatchSites {
            reset: 0x1000,
            getter: 0x2000,
        };
        let mut memory = FakePatchMemory::original(sites);
        memory.set(sites.getter, [0x90; 3]);

        assert_eq!(
            super::enable_patch(&mut memory, sites),
            Err(super::RepeatQuestError::UnexpectedBytes)
        );
        assert_eq!(
            super::restore_patch(&mut memory, sites),
            Err(super::RepeatQuestError::UnexpectedBytes)
        );
        assert!(memory.writes.is_empty());
    }

    #[test]
    fn enable_and_restore_are_idempotent_at_the_requested_state() {
        let sites = super::PatchSites {
            reset: 0x1000,
            getter: 0x2000,
        };
        let mut off = FakePatchMemory::original(sites);
        assert_eq!(
            super::restore_patch(&mut off, sites).unwrap(),
            super::ObservedPatchState::Off
        );
        assert!(off.writes.is_empty());

        let mut on = FakePatchMemory::original(sites);
        on.set(sites.reset, super::RESET_PATCHED);
        on.set(sites.getter, super::GETTER_PATCHED);
        assert_eq!(
            super::enable_patch(&mut on, sites).unwrap(),
            super::ObservedPatchState::On
        );
        assert!(on.writes.is_empty());
    }

    #[test]
    fn mixed_state_cannot_be_enabled() {
        let sites = super::PatchSites {
            reset: 0x1000,
            getter: 0x2000,
        };
        let mut memory = FakePatchMemory::original(sites);
        memory.set(sites.reset, super::RESET_PATCHED);

        assert_eq!(
            super::enable_patch(&mut memory, sites),
            Err(super::RepeatQuestError::UnexpectedBytes)
        );
        assert!(memory.writes.is_empty());
    }

    #[test]
    fn rollback_failure_is_reported_instead_of_the_original_write_error() {
        let sites = super::PatchSites {
            reset: 0x1000,
            getter: 0x2000,
        };
        let mut memory = FakePatchMemory::original(sites);
        memory.fail_write_once_at = Some(sites.getter);
        memory.fail_write_value_once = Some((sites.reset, super::RESET_ORIGINAL));

        assert_eq!(
            super::enable_patch(&mut memory, sites),
            Err(super::RepeatQuestError::Rollback)
        );
    }

    #[test]
    fn accepts_only_the_pinned_game_hash() {
        let pinned =
            super::parse_sha256("63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F")
                .unwrap();
        assert_eq!(super::verify_game_hash(&pinned), Ok(()));
        assert_eq!(
            super::verify_game_hash(&[0; 32]),
            Err(super::RepeatQuestError::UnsupportedGame)
        );
    }

    #[test]
    fn converts_text_offsets_to_checked_remote_addresses() {
        assert_eq!(
            super::patch_sites(
                0x1_4000_1000,
                super::PatchOffsets {
                    reset: 0x28,
                    getter: 0x112,
                },
            )
            .unwrap(),
            super::PatchSites {
                reset: 0x1_4000_1028,
                getter: 0x1_4000_1112,
            }
        );
        assert_eq!(
            super::patch_sites(
                usize::MAX,
                super::PatchOffsets {
                    reset: 1,
                    getter: 0,
                },
            ),
            Err(super::RepeatQuestError::AddressOverflow)
        );
    }

    #[test]
    fn every_new_runtime_restores_off_on_startup() {
        let backend = Arc::new(FakeBackend::patched());
        let state = super::RepeatQuestState::with_backend(backend.clone());

        state.restore_on_startup();

        assert_eq!(backend.restore_calls(), 1);
        assert_eq!(state.status().state, super::RepeatQuestStatusKind::Off);
    }

    #[test]
    fn normal_exit_restores_once_only_after_successful_enable() {
        let backend = Arc::new(FakeBackend::original());
        let state = super::RepeatQuestState::with_backend(backend.clone());

        assert_eq!(
            state.set_enabled(true).state,
            super::RepeatQuestStatusKind::On
        );
        state.restore_on_exit();
        state.restore_on_exit();

        assert_eq!(backend.enable_calls.load(Ordering::Acquire), 1);
        assert_eq!(backend.restore_calls(), 1);
    }

    #[test]
    fn a_busy_operation_returns_busy_without_a_second_write() {
        let backend = Arc::new(FakeBackend::original());
        let state = super::RepeatQuestState::with_backend(backend.clone());
        let _operation = state.lock_operation_for_test();

        assert_eq!(
            state.set_enabled(true).reason,
            Some(super::RepeatQuestReason::Busy)
        );
        assert_eq!(backend.enable_calls.load(Ordering::Acquire), 0);
    }

    #[test]
    fn maps_backend_failures_to_stable_frontend_reasons() {
        assert_eq!(
            super::RepeatQuestStatus::error(&super::RepeatQuestError::GameNotRunning).reason,
            Some(super::RepeatQuestReason::GameNotRunning)
        );
        assert_eq!(
            super::RepeatQuestStatus::error(&super::RepeatQuestError::SignatureCount {
                site: super::PatchSiteName::Reset,
                count: 0,
            })
            .reason,
            Some(super::RepeatQuestReason::SignatureMissing)
        );
        assert_eq!(
            super::RepeatQuestStatus::operation_error(&super::RepeatQuestError::Rollback, true),
            super::RepeatQuestReason::RestoreFailed
        );
        assert_eq!(
            super::RepeatQuestStatus::operation_error(
                &super::RepeatQuestError::ReadBackMismatch,
                true,
            ),
            super::RepeatQuestReason::PatchFailed
        );
    }

    #[test]
    fn serializes_status_for_the_tauri_frontend_contract() {
        let value = serde_json::to_value(super::RepeatQuestStatus {
            state: super::RepeatQuestStatusKind::Unavailable,
            reason: Some(super::RepeatQuestReason::AccessDenied),
        })
        .unwrap();
        assert_eq!(value["state"], "unavailable");
        assert_eq!(value["reason"], "accessDenied");
    }
}
