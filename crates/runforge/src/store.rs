//! Store package LocalState. An unpackaged process does not use this path.

use std::path::{Path, PathBuf};

/// The running package's LocalState folder, when Windows says this process is packaged.
pub fn packaged_local_state() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        windows_local_state()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
fn windows_local_state() -> Option<PathBuf> {
    let family = package_family_name()?;
    let local = local_app_data()?;
    local_state_dir(&family, &local)
}

/// LocalState for one package family. A family that could escape the Packages folder is refused.
pub(crate) fn local_state_dir(family: &str, local_app_data: &Path) -> Option<PathBuf> {
    if family.is_empty() || family.contains('\\') || family.contains('/') || family.contains("..") {
        return None;
    }
    Some(
        local_app_data
            .join("Packages")
            .join(family)
            .join("LocalState"),
    )
}

fn family_from_buffer(status: i32, buffer: &[u16]) -> Option<String> {
    if status != 0 {
        return None;
    }
    let end = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    String::from_utf16(&buffer[..end]).ok()
}

#[cfg(windows)]
fn package_family_name() -> Option<String> {
    use std::ptr;
    const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
    unsafe extern "system" {
        fn GetCurrentPackageFamilyName(length: *mut u32, name: *mut u16) -> i32;
    }
    unsafe {
        let mut length = 0u32;
        let first = GetCurrentPackageFamilyName(&mut length, ptr::null_mut());
        if first != ERROR_INSUFFICIENT_BUFFER || length == 0 {
            return None;
        }
        let mut buffer = vec![0u16; length as usize];
        let second = GetCurrentPackageFamilyName(&mut length, buffer.as_mut_ptr());
        family_from_buffer(second, &buffer)
    }
}

#[cfg(windows)]
fn local_app_data() -> Option<PathBuf> {
    use std::ffi::c_void;
    use std::ptr;
    #[repr(C)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }
    const LOCAL_APP_DATA: Guid = Guid {
        data1: 0xF1B3_2785,
        data2: 0x6FBA,
        data3: 0x4FCF,
        data4: [0x9D, 0x55, 0x7B, 0x8E, 0x7F, 0x15, 0x70, 0x91],
    };
    unsafe extern "system" {
        fn SHGetKnownFolderPath(
            id: *const Guid,
            flags: u32,
            token: *mut c_void,
            path: *mut *mut u16,
        ) -> i32;
    }
    unsafe extern "system" {
        fn CoTaskMemFree(pointer: *mut c_void);
    }
    unsafe {
        let mut raw = ptr::null_mut();
        let result = SHGetKnownFolderPath(&LOCAL_APP_DATA, 0, ptr::null_mut(), &mut raw);
        if result != 0 || raw.is_null() {
            return None;
        }
        let mut length = 0usize;
        while *raw.add(length) != 0 {
            length += 1;
        }
        let text = String::from_utf16(std::slice::from_raw_parts(raw, length)).ok();
        CoTaskMemFree(raw.cast());
        text.map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::{family_from_buffer, local_state_dir, packaged_local_state};
    use std::path::Path;

    #[test]
    fn an_unpackaged_process_has_no_local_state() {
        assert!(packaged_local_state().is_none());
    }

    #[test]
    fn a_family_that_could_escape_packages_is_refused() {
        let local = Path::new("Local");
        assert!(local_state_dir("", local).is_none());
        assert!(local_state_dir(r"a\b", local).is_none());
        assert!(local_state_dir("a/b", local).is_none());
        assert!(local_state_dir("a/../b", local).is_none());
        let path = local_state_dir("mcp-tool-shop.RunForge-Desktop_yn6b8xqrexa5j", local).unwrap();
        assert_eq!(
            path,
            Path::new("Local")
                .join("Packages")
                .join("mcp-tool-shop.RunForge-Desktop_yn6b8xqrexa5j")
                .join("LocalState")
        );
    }

    #[test]
    fn the_family_buffer_stops_at_the_nul() {
        assert!(family_from_buffer(1, &[b'a' as u16, 0]).is_none());
        assert_eq!(
            family_from_buffer(0, &[b'a' as u16, b'b' as u16, 0, b'c' as u16]).as_deref(),
            Some("ab")
        );
        assert_eq!(family_from_buffer(0, &[b'a' as u16]).as_deref(), Some("a"));
        assert!(family_from_buffer(0, &[0xD800]).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn local_app_data_is_an_absolute_folder() {
        let path = super::local_app_data();
        assert!(path.is_some_and(|path| path.is_absolute()));
    }
}
