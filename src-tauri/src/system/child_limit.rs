#[cfg(unix)]
pub fn prepare(command: &mut tokio::process::Command) {
    unsafe {
        command.pre_exec(|| {
            let none = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::setrlimit(libc::RLIMIT_NPROC, &none) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            let mut applied = libc::rlimit {
                rlim_cur: 1,
                rlim_max: 1,
            };
            if libc::getrlimit(libc::RLIMIT_NPROC, &mut applied) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if applied.rlim_cur == 0 && applied.rlim_max == 0 {
                Ok(())
            } else {
                Err(std::io::Error::other("the process limit was not applied"))
            }
        });
    }
}

#[cfg(unix)]
pub fn ensure_effective() -> std::io::Result<()> {
    if unsafe { libc::geteuid() } == 0 || unsafe { libc::getuid() } == 0 {
        return Err(std::io::Error::other(
            "the process limit is not enforced for the superuser",
        ));
    }
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status")?;
        if capabilities_bypass_the_limit(&status) != Some(false) {
            return Err(std::io::Error::other(
                "the process limit is not enforced with CAP_SYS_RESOURCE or CAP_SYS_ADMIN",
            ));
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn capabilities_bypass_the_limit(status: &str) -> Option<bool> {
    const CAP_SYS_ADMIN: u32 = 21;
    const CAP_SYS_RESOURCE: u32 = 24;
    let effective = status
        .lines()
        .find_map(|line| line.strip_prefix("CapEff:"))?;
    let mask = u64::from_str_radix(effective.trim(), 16).ok()?;
    Some(mask & ((1 << CAP_SYS_ADMIN) | (1 << CAP_SYS_RESOURCE)) != 0)
}

#[cfg(windows)]
pub use windows::{Job, confine, prepare};

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use tokio::process::{Child, Command};
    use windows_sys::Win32::Foundation::CloseHandle;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_SUSPENDED: u32 = 0x0000_0004;
    const JOB_OBJECT_LIMIT_ACTIVE_PROCESS: u32 = 0x8;
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: i32 = 9;

    #[repr(C)]
    #[derive(Default)]
    struct BasicLimitInformation {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct ExtendedLimitInformation {
        basic: BasicLimitInformation,
        io_counters: [u64; 6],
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> *mut c_void;
        fn SetInformationJobObject(
            job: *mut c_void,
            class: i32,
            information: *const c_void,
            length: u32,
        ) -> i32;
        fn QueryInformationJobObject(
            job: *mut c_void,
            class: i32,
            information: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
        fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
    }

    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtResumeProcess(process: *mut c_void) -> i32;
    }

    pub struct Job(*mut c_void);

    unsafe impl Send for Job {}

    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    pub fn prepare(command: &mut Command) {
        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
    }

    pub fn confine(child: &Child, active_processes: u32) -> std::io::Result<Job> {
        let process = child
            .raw_handle()
            .ok_or_else(std::io::Error::last_os_error)? as *mut c_void;
        let flags = JOB_OBJECT_LIMIT_ACTIVE_PROCESS | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let job = Job(handle);
            let mut limits = ExtendedLimitInformation::default();
            limits.basic.limit_flags = flags;
            limits.basic.active_process_limit = active_processes;
            let length = size_of::<ExtendedLimitInformation>() as u32;
            if SetInformationJobObject(
                handle,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                (&raw const limits).cast(),
                length,
            ) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            let mut applied = ExtendedLimitInformation::default();
            if QueryInformationJobObject(
                handle,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                (&raw mut applied).cast(),
                length,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            if applied.basic.limit_flags & flags != flags
                || applied.basic.active_process_limit != active_processes
            {
                return Err(std::io::Error::other("the job limits were not applied"));
            }
            if AssignProcessToJobObject(handle, process) == 0 {
                return Err(std::io::Error::last_os_error());
            }
            if NtResumeProcess(process) < 0 {
                return Err(std::io::Error::other(
                    "the git process could not be resumed",
                ));
            }
            Ok(job)
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn capabilities_that_bypass_the_limit_are_detected() {
        let status = |mask: &str| format!("Name:\tx\nCapPrm:\t{mask}\nCapEff:\t{mask}\n");
        assert_eq!(
            capabilities_bypass_the_limit(&status("0000000000000000")),
            Some(false)
        );
        assert_eq!(
            capabilities_bypass_the_limit(&status("0000000001000000")),
            Some(true)
        );
        assert_eq!(
            capabilities_bypass_the_limit(&status("0000000000200000")),
            Some(true)
        );
        assert_eq!(
            capabilities_bypass_the_limit(&status("000001ffffffffff")),
            Some(true)
        );
        assert_eq!(capabilities_bypass_the_limit("Name:\tx\n"), None);
        assert_eq!(capabilities_bypass_the_limit("CapEff:\tzz\n"), None);
    }
}
