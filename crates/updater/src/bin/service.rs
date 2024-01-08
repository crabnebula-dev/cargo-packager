use std::sync::{Mutex, MutexGuard, OnceLock};

use cargo_packager_updater::current_exe::current_exe;
use windows::{
    core::{w, PCWSTR, PWSTR},
    Win32::{
        Foundation::{NO_ERROR, PSID},
        System::{
            EventLog::{
                DeregisterEventSource, RegisterEventSourceW, ReportEventW, EVENTLOG_ERROR_TYPE,
            },
            Services::{
                CloseServiceHandle, ControlService, CreateServiceW, DeleteService, OpenSCManagerW,
                OpenServiceW, RegisterServiceCtrlHandlerW, SetServiceStatus,
                StartServiceCtrlDispatcherW, StartServiceW, SC_MANAGER_ALL_ACCESS,
                SERVICE_ACCEPT_STOP, SERVICE_ALL_ACCESS, SERVICE_CONTROL_STOP,
                SERVICE_DEMAND_START, SERVICE_ERROR_NORMAL, SERVICE_RUNNING, SERVICE_START,
                SERVICE_START_PENDING, SERVICE_STATUS, SERVICE_STATUS_CURRENT_STATE,
                SERVICE_STATUS_HANDLE, SERVICE_STOP, SERVICE_STOPPED, SERVICE_TABLE_ENTRYW,
                SERVICE_WIN32_OWN_PROCESS,
            },
        },
    },
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const SVC_NAME: PCWSTR = w!("windows-service-poc");
const SVC_NAME_DISPLAY: PCWSTR = w!("Windows Service POC");
const SVC_ERROR: u32 = 0xC203201;

static SVC_STATUS_HANDLE: OnceLock<SERVICE_STATUS_HANDLE> = OnceLock::new();

#[inline]
pub fn encode_wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    std::os::windows::ffi::OsStrExt::encode_wide(s.as_ref())
        .chain(std::iter::once(0))
        .collect()
}

pub fn report_event<S: AsRef<str>>(error: S) -> Result<()> {
    let handle = unsafe { RegisterEventSourceW(PCWSTR::null(), SVC_NAME) }?;
    let message = encode_wide(error.as_ref());
    unsafe {
        ReportEventW(
            handle,
            EVENTLOG_ERROR_TYPE,
            0,
            SVC_ERROR,
            PSID::default(),
            0,
            Some(&[SVC_NAME, PCWSTR::from_raw(message.as_ptr())]),
            None,
        )?;

        DeregisterEventSource(handle)?;
    }
    Ok(())
}

fn svc_status<'a>() -> MutexGuard<'a, SERVICE_STATUS> {
    static SVC_STATUS: OnceLock<Mutex<SERVICE_STATUS>> = OnceLock::new();
    SVC_STATUS
        .get_or_init(|| {
            let mut svc_status = SERVICE_STATUS::default();
            svc_status.dwServiceType = SERVICE_WIN32_OWN_PROCESS;
            svc_status.dwServiceSpecificExitCode = 0;
            Mutex::new(svc_status)
        })
        .lock()
        .unwrap()
}

fn svc_check_point<'a>() -> MutexGuard<'a, u32> {
    static SVC_STATUS: OnceLock<Mutex<u32>> = OnceLock::new();
    SVC_STATUS.get_or_init(Mutex::default).lock().unwrap()
}

pub fn report_svc_status(
    dw_current_state: SERVICE_STATUS_CURRENT_STATE,
    dw_win32_exit_cide: u32,
    dw_wait_hint: u32,
) -> Result<()> {
    let mut status = svc_status();
    status.dwCurrentState = dw_current_state;
    status.dwWin32ExitCode = dw_win32_exit_cide;
    status.dwWaitHint = dw_wait_hint;
    status.dwControlsAccepted = match dw_current_state {
        SERVICE_START_PENDING => 0,
        _ => SERVICE_ACCEPT_STOP,
    };
    status.dwCheckPoint = match dw_current_state {
        SERVICE_RUNNING | SERVICE_STOPPED => 0,
        _ => {
            let mut svc_check_point = svc_check_point();
            *svc_check_point += 1;
            *svc_check_point
        }
    };

    if let Some(handle) = SVC_STATUS_HANDLE.get() {
        unsafe { SetServiceStatus(*handle, &*status) }?;
    }

    Ok(())
}

fn try_svc_ctrl_handler(dwctrl: u32) -> Result<()> {
    match dwctrl {
        SERVICE_CONTROL_STOP => {
            // TODO
            // report_svc_status(SERVICE_STOPPED, NO_ERROR.0, 0)?;
            // signal any working thread to stop
            report_svc_status(SERVICE_STOPPED, NO_ERROR.0, 0)?;
        }
        _ => {}
    }

    Ok(())
}

unsafe extern "system" fn svc_ctrl_handler(dwctrl: u32) {
    if let Err(e) = try_svc_ctrl_handler(dwctrl) {
        let _ = report_event(e.to_string());
    }
}

fn try_svc_main(_argc: u32, _argv: *mut PWSTR) -> Result<()> {
    let handle = unsafe { RegisterServiceCtrlHandlerW(SVC_NAME, Some(svc_ctrl_handler)) }?;
    SVC_STATUS_HANDLE
        .set(handle)
        .map_err(|_| "failed to initialize SVC_STATUS_HANDLE")?;

    // TODO
    // report_svc_status(SERVICE_START_PENDING, NO_ERROR.0, 0)?;
    // init_service background thread? or communciate through what?

    report_svc_status(SERVICE_RUNNING, NO_ERROR.0, 0)?;

    Ok(())
}

unsafe extern "system" fn svc_main(argc: u32, argv: *mut PWSTR) {
    if let Err(e) = try_svc_main(argc, argv) {
        let _ = report_event(e.to_string());
    }
}

fn start_svc_main() -> Result<()> {
    let service = SERVICE_TABLE_ENTRYW {
        lpServiceName: PWSTR::from_raw(SVC_NAME.as_ptr() as _),
        lpServiceProc: Some(svc_main),
    };

    if let Err(e) = unsafe { StartServiceCtrlDispatcherW(&service) } {
        report_event(e.to_string())?;
        return Err(e.into());
    }

    Ok(())
}

fn install_svc() -> Result<()> {
    let scm = unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS) }?;

    let current_exe = current_exe()?;
    let current_exe = dunce::simplified(&current_exe);
    let current_exe = encode_wide(current_exe);

    unsafe {
        let service = CreateServiceW(
            scm,
            SVC_NAME,
            SVC_NAME_DISPLAY,
            SERVICE_ALL_ACCESS,
            SERVICE_WIN32_OWN_PROCESS,
            SERVICE_DEMAND_START,
            SERVICE_ERROR_NORMAL,
            PCWSTR::from_raw(current_exe.as_ptr()),
            PCWSTR::null(),
            None,
            PCWSTR::null(),
            PCWSTR::null(),
            PCWSTR::null(),
        )?;

        CloseServiceHandle(service)?;
        CloseServiceHandle(scm)?;
    }

    Ok(())
}

fn uninstall_svc() -> Result<()> {
    unsafe {
        let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)?;
        const DELETE: u32 = 0x10000;
        let service = OpenServiceW(scm, SVC_NAME, DELETE)?;
        DeleteService(service)?;
        CloseServiceHandle(service)?;
        CloseServiceHandle(scm)?;
    }

    Ok(())
}

fn start_svc() -> Result<()> {
    unsafe {
        let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)?;
        let service = OpenServiceW(scm, SVC_NAME, SERVICE_START)?;
        StartServiceW(service, None)?;
        CloseServiceHandle(service)?;
        CloseServiceHandle(scm)?;
    }

    Ok(())
}
fn stop_svc() -> Result<()> {
    unsafe {
        let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)?;
        let service = OpenServiceW(scm, SVC_NAME, SERVICE_STOP)?;
        let mut ret = SERVICE_STATUS::default();
        ControlService(service, SERVICE_CONTROL_STOP, &mut ret)?;
        CloseServiceHandle(service)?;
        CloseServiceHandle(scm)?;
    }

    Ok(())
}

fn handle_cli_args() -> Result<()> {
    let command = std::env::args_os()
        .skip(1)
        .next()
        .map(|a| a.to_string_lossy().to_string());
    match command.as_deref() {
        Some("install") => {
            install_svc()?;
        }
        Some("uninstall") => {
            uninstall_svc()?;
        }
        Some("start") => {
            start_svc()?;
        }
        Some("stop") => {
            stop_svc()?;
        }
        _ => return Ok(()),
    }

    std::process::exit(0);
}

fn main() -> Result<()> {
    handle_cli_args()?;
    start_svc_main()
}
