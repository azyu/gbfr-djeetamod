use thiserror::Error;

#[cfg(windows)]
use sha2::{Digest, Sha256};
#[cfg(windows)]
use std::{
    ffi::{c_void, OsString},
    fs::File,
    io::Read,
    os::windows::ffi::OsStringExt,
    path::{Path, PathBuf},
};
#[cfg(windows)]
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE, STILL_ACTIVE},
    System::{
        Diagnostics::{
            Debug::ReadProcessMemory,
            ToolHelp::{
                CreateToolhelp32Snapshot, Module32FirstW, Process32FirstW, Process32NextW,
                MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32,
                TH32CS_SNAPPROCESS,
            },
        },
        Memory::{VirtualQueryEx, MEMORY_BASIC_INFORMATION},
        Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
};

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum MemoryReadError {
    #[error("memory at {0:#x} is unavailable")]
    Unavailable(usize),
    #[error("read at {address:#x} returned {actual} of {expected} bytes")]
    PartialRead {
        address: usize,
        expected: usize,
        actual: usize,
    },
    #[error("invalid remote PE image: {0}")]
    InvalidPe(&'static str),
    #[error("Windows process read failed: {0}")]
    Windows(String),
}

pub(crate) trait MemoryReader {
    fn read_exact(&self, address: usize, output: &mut [u8]) -> Result<(), MemoryReadError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PeSection {
    pub rva: usize,
    pub size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemoryRegion {
    pub base_address: usize,
    pub size: usize,
}

impl MemoryRegion {
    pub(crate) fn end(self) -> Option<usize> {
        self.base_address.checked_add(self.size)
    }
}

pub(crate) fn is_readable_private_region(state: u32, kind: u32, protect: u32) -> bool {
    const MEM_COMMIT_VALUE: u32 = 0x1000;
    const MEM_PRIVATE_VALUE: u32 = 0x20000;
    const PAGE_NOACCESS_VALUE: u32 = 0x01;
    const PAGE_GUARD_VALUE: u32 = 0x100;
    const READABLE: [u32; 6] = [0x02, 0x04, 0x08, 0x20, 0x40, 0x80];

    state == MEM_COMMIT_VALUE
        && kind == MEM_PRIVATE_VALUE
        && protect & PAGE_GUARD_VALUE == 0
        && protect & 0xFF != PAGE_NOACCESS_VALUE
        && READABLE.contains(&(protect & 0xFF))
}

#[cfg(windows)]
#[derive(Debug)]
struct OwnedHandle(HANDLE);

#[cfg(windows)]
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
#[derive(Debug)]
pub(crate) struct RemoteProcess {
    handle: OwnedHandle,
    pub pid: u32,
    pub module_base: usize,
    pub module_size: usize,
    pub module_path: PathBuf,
}

#[cfg(windows)]
impl RemoteProcess {
    pub fn find(name: &str) -> Result<Option<Self>, MemoryReadError> {
        let Some(pid) = find_process_id(name)? else {
            return Ok(None);
        };
        let (module_base, module_size, executable_path) = find_main_module(pid, name)?;
        let handle =
            unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) }
                .map_err(windows_error)?;

        Ok(Some(Self {
            handle: OwnedHandle(handle),
            pid,
            module_base,
            module_size,
            module_path: executable_path,
        }))
    }

    pub fn read_text_section(&self) -> Result<(usize, Vec<u8>), MemoryReadError> {
        let header_size = self.module_size.min(0x1000);
        let mut headers = vec![0; header_size];
        self.read_exact(self.module_base, &mut headers)?;
        let section = parse_text_section(&headers)?;
        let end = section
            .rva
            .checked_add(section.size)
            .ok_or(MemoryReadError::InvalidPe("text section range overflow"))?;
        if end > self.module_size {
            return Err(MemoryReadError::InvalidPe(
                "text section exceeds module image",
            ));
        }
        let address = self
            .module_base
            .checked_add(section.rva)
            .ok_or(MemoryReadError::InvalidPe("text address overflow"))?;
        let mut text = vec![0; section.size];
        self.read_exact(address, &mut text)?;
        Ok((address, text))
    }

    pub fn executable_sha256(&self) -> Result<[u8; 32], MemoryReadError> {
        sha256_file(&self.module_path)
    }

    pub fn is_running(&self) -> Result<bool, MemoryReadError> {
        let mut exit_code = 0;
        unsafe { GetExitCodeProcess(self.handle.0, &mut exit_code) }.map_err(windows_error)?;
        Ok(exit_code == STILL_ACTIVE.0 as u32)
    }

    pub(crate) fn readable_private_regions(&self) -> Result<Vec<MemoryRegion>, MemoryReadError> {
        const MAX_USER_ADDRESS: usize = 0x0000_7FFF_FFFF_FFFF;

        let mut regions = Vec::new();
        let mut address = 0usize;
        while address < MAX_USER_ADDRESS {
            let mut info = MEMORY_BASIC_INFORMATION::default();
            let queried = unsafe {
                VirtualQueryEx(
                    self.handle.0,
                    Some(address as *const c_void),
                    &mut info,
                    std::mem::size_of_val(&info),
                )
            };
            if queried == 0 {
                return Err(windows_error(windows::core::Error::from_win32()));
            }
            if info.RegionSize == 0 {
                return Err(MemoryReadError::Windows(
                    "VirtualQueryEx returned an empty region".to_owned(),
                ));
            }
            if is_readable_private_region(info.State.0, info.Type.0, info.Protect.0) {
                regions.push(MemoryRegion {
                    base_address: info.BaseAddress as usize,
                    size: info.RegionSize,
                });
            }
            address = (info.BaseAddress as usize)
                .checked_add(info.RegionSize)
                .ok_or(MemoryReadError::InvalidPe("memory region range overflow"))?;
        }
        Ok(regions)
    }
}

#[cfg(windows)]
impl MemoryReader for RemoteProcess {
    fn read_exact(&self, address: usize, output: &mut [u8]) -> Result<(), MemoryReadError> {
        if output.is_empty() {
            return Ok(());
        }
        let mut actual = 0;
        let result = unsafe {
            ReadProcessMemory(
                self.handle.0,
                address as *const c_void,
                output.as_mut_ptr().cast::<c_void>(),
                output.len(),
                Some(&mut actual),
            )
        };
        if let Err(error) = result {
            return if actual == 0 {
                Err(MemoryReadError::Unavailable(address))
            } else if actual != output.len() {
                validate_read_count(address, output.len(), actual)
            } else {
                Err(windows_error(error))
            };
        }
        validate_read_count(address, output.len(), actual)
    }
}

#[cfg(windows)]
fn windows_error(error: windows::core::Error) -> MemoryReadError {
    MemoryReadError::Windows(error.to_string())
}

#[cfg(windows)]
fn wide_string(value: &[u16]) -> OsString {
    let len = value
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(value.len());
    OsString::from_wide(&value[..len])
}

#[cfg(windows)]
fn find_process_id(name: &str) -> Result<Option<u32>, MemoryReadError> {
    let snapshot = OwnedHandle(
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.map_err(windows_error)?,
    );
    (|| {
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        unsafe { Process32FirstW(snapshot.0, &mut entry) }.map_err(windows_error)?;
        loop {
            if wide_string(&entry.szExeFile)
                .to_string_lossy()
                .eq_ignore_ascii_case(name)
            {
                return Ok(Some(entry.th32ProcessID));
            }
            if unsafe { Process32NextW(snapshot.0, &mut entry) }.is_err() {
                break;
            }
        }
        Ok(None)
    })()
}

#[cfg(windows)]
fn find_main_module(pid: u32, name: &str) -> Result<(usize, usize, PathBuf), MemoryReadError> {
    let snapshot = OwnedHandle(
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid) }
            .map_err(windows_error)?,
    );
    (|| {
        let mut entry = MODULEENTRY32W {
            dwSize: std::mem::size_of::<MODULEENTRY32W>() as u32,
            ..Default::default()
        };
        unsafe { Module32FirstW(snapshot.0, &mut entry) }.map_err(windows_error)?;
        let module_name = wide_string(&entry.szModule).to_string_lossy().into_owned();
        if !module_name.eq_ignore_ascii_case(name) {
            return Err(MemoryReadError::Windows(format!(
                "main module {module_name} does not match {name}"
            )));
        }
        Ok((
            entry.modBaseAddr as usize,
            entry.modBaseSize as usize,
            PathBuf::from(wide_string(&entry.szExePath)),
        ))
    })()
}

#[cfg(windows)]
fn sha256_file(path: &Path) -> Result<[u8; 32], MemoryReadError> {
    let mut file = File::open(path).map_err(|error| MemoryReadError::Windows(error.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| MemoryReadError::Windows(error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, MemoryReadError> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or(MemoryReadError::InvalidPe("truncated u16 field"))?;
    Ok(u16::from_le_bytes(
        value.try_into().expect("two-byte PE field"),
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, MemoryReadError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(MemoryReadError::InvalidPe("truncated u32 field"))?;
    Ok(u32::from_le_bytes(
        value.try_into().expect("four-byte PE field"),
    ))
}

pub(crate) fn parse_text_section(headers: &[u8]) -> Result<PeSection, MemoryReadError> {
    if headers.get(..2) != Some(b"MZ") {
        return Err(MemoryReadError::InvalidPe("missing DOS signature"));
    }
    let pe_offset = usize::try_from(read_u32(headers, 0x3C)?)
        .map_err(|_| MemoryReadError::InvalidPe("PE offset does not fit usize"))?;
    if headers.get(pe_offset..pe_offset + 4) != Some(b"PE\0\0") {
        return Err(MemoryReadError::InvalidPe("missing PE signature"));
    }

    let section_count = usize::from(read_u16(headers, pe_offset + 6)?);
    let optional_size = usize::from(read_u16(headers, pe_offset + 20)?);
    let optional_header = pe_offset
        .checked_add(24)
        .ok_or(MemoryReadError::InvalidPe("optional header overflow"))?;
    if read_u16(headers, optional_header)? != 0x20B {
        return Err(MemoryReadError::InvalidPe("image is not PE32+"));
    }
    let section_table = optional_header
        .checked_add(optional_size)
        .ok_or(MemoryReadError::InvalidPe("section table overflow"))?;

    for index in 0..section_count {
        let offset = section_table
            .checked_add(
                index
                    .checked_mul(40)
                    .ok_or(MemoryReadError::InvalidPe("section index overflow"))?,
            )
            .ok_or(MemoryReadError::InvalidPe("section offset overflow"))?;
        let section = headers
            .get(offset..offset + 40)
            .ok_or(MemoryReadError::InvalidPe("truncated section table"))?;
        if &section[..8] == b".text\0\0\0" {
            let size_u32 =
                u32::from_le_bytes(section[8..12].try_into().expect("section virtual size"));
            let rva_u32 = u32::from_le_bytes(section[12..16].try_into().expect("section RVA"));
            rva_u32
                .checked_add(size_u32)
                .ok_or(MemoryReadError::InvalidPe("text section range overflow"))?;
            let size = usize::try_from(size_u32)
                .map_err(|_| MemoryReadError::InvalidPe("text size does not fit usize"))?;
            let rva = usize::try_from(rva_u32)
                .map_err(|_| MemoryReadError::InvalidPe("text RVA does not fit usize"))?;
            if size == 0 {
                return Err(MemoryReadError::InvalidPe("text section is empty"));
            }
            rva.checked_add(size)
                .ok_or(MemoryReadError::InvalidPe("text section range overflow"))?;
            return Ok(PeSection { rva, size });
        }
    }

    Err(MemoryReadError::InvalidPe("text section is missing"))
}

pub(crate) fn validate_read_count(
    address: usize,
    expected: usize,
    actual: usize,
) -> Result<(), MemoryReadError> {
    if actual == expected {
        Ok(())
    } else {
        Err(MemoryReadError::PartialRead {
            address,
            expected,
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_readable_private_region, parse_text_section, validate_read_count, MemoryReadError,
        MemoryRegion, PeSection,
    };

    fn pe_fixture() -> Vec<u8> {
        let mut bytes = vec![0u8; 0x1B0];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3C..0x40].copy_from_slice(&0x80_u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x86..0x88].copy_from_slice(&1_u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xF0_u16.to_le_bytes());
        bytes[0x98..0x9A].copy_from_slice(&0x20B_u16.to_le_bytes());
        let section = 0x188;
        bytes[section..section + 8].copy_from_slice(b".text\0\0\0");
        bytes[section + 8..section + 12].copy_from_slice(&0x2000_u32.to_le_bytes());
        bytes[section + 12..section + 16].copy_from_slice(&0x1000_u32.to_le_bytes());
        bytes
    }

    #[test]
    fn parses_remote_text_section() {
        assert_eq!(
            parse_text_section(&pe_fixture()).unwrap(),
            PeSection {
                rva: 0x1000,
                size: 0x2000,
            }
        );
    }

    #[test]
    fn rejects_invalid_or_truncated_pe_headers() {
        let mut invalid_magic = pe_fixture();
        invalid_magic[0] = 0;
        assert!(matches!(
            parse_text_section(&invalid_magic),
            Err(MemoryReadError::InvalidPe(_))
        ));
        assert!(matches!(
            parse_text_section(&pe_fixture()[..0x100]),
            Err(MemoryReadError::InvalidPe(_))
        ));

        let mut invalid_pe = pe_fixture();
        invalid_pe[0x80] = 0;
        assert!(matches!(
            parse_text_section(&invalid_pe),
            Err(MemoryReadError::InvalidPe(_))
        ));

        let mut missing_text = pe_fixture();
        missing_text[0x188..0x190].copy_from_slice(b".data\0\0\0");
        assert!(matches!(
            parse_text_section(&missing_text),
            Err(MemoryReadError::InvalidPe(_))
        ));

        let mut overflowing_text = pe_fixture();
        overflowing_text[0x190..0x194].copy_from_slice(&0x2000_u32.to_le_bytes());
        overflowing_text[0x194..0x198].copy_from_slice(&0xFFFF_F000_u32.to_le_bytes());
        assert!(matches!(
            parse_text_section(&overflowing_text),
            Err(MemoryReadError::InvalidPe(_))
        ));
    }

    #[test]
    fn partial_reads_are_rejected() {
        assert_eq!(validate_read_count(0x1234, 16, 16), Ok(()));
        assert_eq!(
            validate_read_count(0x1234, 16, 8),
            Err(MemoryReadError::PartialRead {
                address: 0x1234,
                expected: 16,
                actual: 8,
            })
        );
    }

    #[test]
    fn includes_only_committed_private_readable_regions() {
        let committed = 0x1000;
        let private = 0x20000;
        let readwrite = 0x04;
        assert!(is_readable_private_region(committed, private, readwrite));
        assert!(!is_readable_private_region(0x10000, private, readwrite));
        assert!(!is_readable_private_region(committed, 0x40000, readwrite));
        assert!(!is_readable_private_region(committed, private, 0x01));
        assert!(!is_readable_private_region(
            committed,
            private,
            readwrite | 0x100
        ));
    }

    #[test]
    fn memory_region_rejects_overflowing_end_addresses() {
        assert_eq!(
            MemoryRegion {
                base_address: 0x1000,
                size: 0x2000,
            }
            .end(),
            Some(0x3000)
        );
        assert_eq!(
            MemoryRegion {
                base_address: usize::MAX,
                size: 2,
            }
            .end(),
            None
        );
    }
}
