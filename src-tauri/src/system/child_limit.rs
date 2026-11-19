#[cfg(unix)]
pub fn prepare(command: &mut tokio::process::Command) {
    unsafe {
        command.pre_exec(|| {
            let none = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::setrlimit(libc::RLIMIT_NPROC, &none) == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }
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
    const JOB_OBJECT_BASIC_LIMIT_INFORMATION: i32 = 2;

    #[repr(C)]
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

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> *mut c_void;
        fn SetInformationJobObject(
            job: *mut c_void,
            class: i32,
            information: *const c_void,
            length: u32,
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
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let job = Job(handle);
            let limits = BasicLimitInformation {
                per_process_user_time_limit: 0,
                per_job_user_time_limit: 0,
                limit_flags: JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
                minimum_working_set_size: 0,
                maximum_working_set_size: 0,
                active_process_limit: active_processes,
                affinity: 0,
                priority_class: 0,
                scheduling_class: 0,
            };
            if SetInformationJobObject(
                handle,
                JOB_OBJECT_BASIC_LIMIT_INFORMATION,
                (&raw const limits).cast(),
                size_of::<BasicLimitInformation>() as u32,
            ) == 0
                || AssignProcessToJobObject(handle, process) == 0
            {
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
