use crate::domain::speech_model::Machine;
use std::ffi::{CStr, CString};

pub fn detect() -> Machine {
    Machine {
        chip: text("machdep.cpu.brand_string").unwrap_or_default(),
        memory_bytes: number::<u64>("hw.memsize").unwrap_or(0),
        apple_silicon: number::<i32>("hw.optional.arm64") == Some(1),
    }
}

fn number<T: Default + Copy>(name: &str) -> Option<T> {
    let name = CString::new(name).ok()?;
    let mut value = T::default();
    let mut size = std::mem::size_of::<T>();
    let status = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            (&mut value as *mut T).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    (status == 0 && size == std::mem::size_of::<T>()).then_some(value)
}

fn text(name: &str) -> Option<String> {
    let name = CString::new(name).ok()?;
    let mut size = 0;
    let probe = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if probe != 0 || size == 0 {
        return None;
    }
    let mut buffer = vec![0_u8; size];
    let status = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            buffer.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if status != 0 {
        return None;
    }
    let text = CStr::from_bytes_until_nul(&buffer).ok()?;
    Some(text.to_string_lossy().trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_chip_and_memory_of_this_mac() {
        let machine = detect();

        assert!(!machine.chip.is_empty());
        assert!(machine.memory_bytes >= 1024 * 1024 * 1024);
        assert_eq!(machine.apple_silicon, machine.chip.starts_with("Apple"));
    }
}
