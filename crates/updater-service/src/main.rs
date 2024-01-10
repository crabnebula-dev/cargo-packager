use std::{
    path::Path,
    process::Command,
    sync::{Mutex, MutexGuard, OnceLock},
};

use cargo_packager_updater::{current_exe::current_exe, UpdaterBuilder};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{HANDLE, NO_ERROR, PSID},
        Globalization::lstrlenW,
        System::{
            EventLog::{
                DeregisterEventSource, RegisterEventSourceW, ReportEventW, EVENTLOG_ERROR_TYPE,
            },
            Services::{
                CloseServiceHandle, ControlService, CreateServiceW, DeleteService, OpenSCManagerW,
                OpenServiceW, RegisterServiceCtrlHandlerW, SetServiceStatus,
                StartServiceCtrlDispatcherW, StartServiceW, SC_MANAGER_ALL_ACCESS,
                SC_MANAGER_CONNECT, SERVICE_ACCEPT_STOP, SERVICE_ALL_ACCESS, SERVICE_CONTROL_STOP,
                SERVICE_DEMAND_START, SERVICE_ERROR_NORMAL, SERVICE_RUNNING, SERVICE_START,
                SERVICE_START_PENDING, SERVICE_STATUS, SERVICE_STATUS_CURRENT_STATE,
                SERVICE_STATUS_HANDLE, SERVICE_STOP, SERVICE_STOPPED, SERVICE_TABLE_ENTRYW,
                SERVICE_WIN32_OWN_PROCESS,
            },
            Threading::{CreateEventW, SetEvent, WaitForSingleObject, INFINITE},
        },
    },
};
use winreg::enums::HKEY_LOCAL_MACHINE;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const SVC_ERROR: u32 = 0xC203201;

static SVC_STATUS_HANDLE: OnceLock<SERVICE_STATUS_HANDLE> = OnceLock::new();
static SVC_NAME: OnceLock<String> = OnceLock::new();

#[inline]
pub fn encode_wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    std::os::windows::ffi::OsStrExt::encode_wide(s.as_ref())
        .chain(std::iter::once(0))
        .collect()
}

pub fn wchar_ptr_to_string(wchar: PWSTR) -> String {
    let len = unsafe { lstrlenW(PCWSTR::from_raw(wchar.0)) } as usize;
    let wchar_slice = unsafe { std::slice::from_raw_parts(wchar.0, len) };
    String::from_utf16_lossy(wchar_slice)
}

pub fn report_event_to_service<S: AsRef<str>>(svc_name: &str, error: S, level: u32) -> Result<()> {
    let svc_name = encode_wide(svc_name);
    let svc_name = PCWSTR::from_raw(svc_name.as_ptr());

    let handle = unsafe { RegisterEventSourceW(PCWSTR::null(), svc_name) }?;
    let message = encode_wide(error.as_ref());
    unsafe {
        ReportEventW(
            handle,
            EVENTLOG_ERROR_TYPE,
            0,
            level,
            PSID::default(),
            0,
            Some(&[svc_name, PCWSTR::from_raw(message.as_ptr())]),
            None,
        )?;

        DeregisterEventSource(handle)?;
    }
    Ok(())
}

pub fn report_event<S: AsRef<str>>(error: S, level: u32) -> Result<()> {
    report_event_to_service(
        SVC_NAME.get().map(|s| s.as_str()).unwrap_or_default(),
        error,
        level,
    )
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

fn svc_stop_event<'a>() -> &'a HANDLE {
    static SVC_STATUS: OnceLock<HANDLE> = OnceLock::new();
    SVC_STATUS.get_or_init(|| unsafe { CreateEventW(None, true, false, None).unwrap_or_default() })
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
            report_svc_status(SERVICE_STOPPED, NO_ERROR.0, 0)?;
        }
        _ => {}
    }

    Ok(())
}

unsafe extern "system" fn svc_ctrl_handler(dwctrl: u32) {
    if let Err(e) = try_svc_ctrl_handler(dwctrl).or_else(|e| {
        report_svc_status(SERVICE_STOPPED, NO_ERROR.0, 0)?;
        Err(e)
    }) {
        let _ = report_event(e.to_string(), SVC_ERROR);
    }
}

fn try_svc_main(argc: u32, argv: *mut PWSTR) -> Result<()> {
    let args: Vec<String> = (0..argc)
        .map(|i| {
            let array_element_ptr = unsafe { argv.offset(i as isize) };
            wchar_ptr_to_string(unsafe { *array_element_ptr })
        })
        .collect();

    let svc_name = args[0].clone();
    let svc_name_pwstr = encode_wide(&svc_name);
    let svc_name_pwstr = PCWSTR::from_raw(svc_name_pwstr.as_ptr());

    SVC_NAME
        .set(svc_name)
        .map_err(|_| "failed to initialize SVC_NAME")?;

    let handle = unsafe { RegisterServiceCtrlHandlerW(svc_name_pwstr, Some(svc_ctrl_handler)) }?;
    SVC_STATUS_HANDLE
        .set(handle)
        .map_err(|_| "failed to initialize SVC_STATUS_HANDLE")?;

    report_svc_status(SERVICE_START_PENDING, NO_ERROR.0, 0)?;

    if svc_stop_event().is_invalid() {
        report_svc_status(SERVICE_STOPPED, NO_ERROR.0, 0)?;
        return Ok(());
    }

    report_svc_status(SERVICE_RUNNING, NO_ERROR.0, 0)?;

    std::thread::spawn(move || {
        fn perform_update(args: &[String]) -> Result<()> {
            let manufacturer = &args[1];
            let product_name = &args[2];

            let path = format!(r"SOFTWARE\{manufacturer}\{product_name} Updater Service");
            let reg = winreg::RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path)?;
            let pub_key: String = reg.get_value("PubKey")?;
            let executable_path: String = reg.get_value("ExecutablePath")?;

            let version = &args[3];
            let config = serde_json::from_str(&args[4])?;

            std::env::set_current_dir(
                Path::new(&executable_path)
                    .parent()
                    .ok_or("Failed to get parent")?
                    .parent()
                    .ok_or("Failed to get parent")?,
            )?;

            let update = UpdaterBuilder::new(version.parse()?, config)
                .executable_path(executable_path)
                .pub_key(pub_key)
                .build()?
                .check()?;

            if let Some(update) = update {
                let bytes = update.download()?;
                // TODO: close executable
                update.install_no_exit(bytes)?;
            }

            Ok(())
        }

        if let Err(e) = perform_update(&args) {
            let _ = report_event(e.to_string(), SVC_ERROR);
        }

        unsafe { SetEvent(*svc_stop_event()).unwrap() };
    });

    loop {
        unsafe { WaitForSingleObject(*svc_stop_event(), INFINITE) };
        report_svc_status(SERVICE_STOPPED, NO_ERROR.0, 0)?;
        break;
    }

    Ok(())
}

unsafe extern "system" fn svc_main(argc: u32, argv: *mut PWSTR) {
    if let Err(e) = try_svc_main(argc, argv).or_else(|e| {
        report_svc_status(SERVICE_STOPPED, NO_ERROR.0, 0)?;
        Err(e)
    }) {
        let _ = report_event(e.to_string(), SVC_ERROR);
    }
}

fn start_svc_main(svc_name: &str) -> Result<()> {
    let svc_name_pwstr = encode_wide(svc_name);

    let service = SERVICE_TABLE_ENTRYW {
        lpServiceName: PWSTR::from_raw(svc_name_pwstr.as_ptr() as _),
        lpServiceProc: Some(svc_main),
    };

    if let Err(e) = unsafe { StartServiceCtrlDispatcherW(&service) } {
        report_event_to_service(svc_name, e.to_string(), SVC_ERROR)?;
        return Err(e.into());
    }

    Ok(())
}

fn install_svc(svc_name: &str) -> Result<()> {
    let scm = unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS) }?;

    let current_exe = current_exe()?;
    let current_exe = dunce::simplified(&current_exe);
    let current_exe = encode_wide(format!("{} {svc_name}", current_exe.display()));

    let svc_name_pcwstr = encode_wide(svc_name);
    let svc_name_pcwstr = PCWSTR::from_raw(svc_name_pcwstr.as_ptr());

    unsafe {
        let service = CreateServiceW(
            scm,
            svc_name_pcwstr,
            svc_name_pcwstr,
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

        let sc_exe = std::env::var("SYSTEMROOT").map_or_else(
            |_| "sc.exe".to_string(),
            |p| format!("{p}\\System32\\sc.exe"),
        );
        let output = Command::new(&sc_exe).arg("sdshow").arg(svc_name).output()?;
        let existing_perm = String::from_utf8_lossy(&output.stdout);
        let existing_perm = existing_perm.trim();
        let (dacl, sacl) = existing_perm
            .split_once("S:")
            .unwrap_or((existing_perm, ""));
        Command::new(sc_exe)
            .arg("sdset")
            .arg(svc_name)
            .arg(format!("{dacl}(A;;RPWPRC;;;BU)S:{sacl}"))
            .output()?;
    }

    Ok(())
}

fn uninstall_svc(svc_name: &str) -> Result<()> {
    let svc_name = encode_wide(svc_name);
    let svc_name = PCWSTR::from_raw(svc_name.as_ptr());

    unsafe {
        let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)?;
        const DELETE: u32 = 0x10000;
        let service = OpenServiceW(scm, svc_name, DELETE)?;
        DeleteService(service)?;
        CloseServiceHandle(service)?;
        CloseServiceHandle(scm)?;
    }

    Ok(())
}

fn start_svc(svc_name: &str, args: Vec<String>) -> Result<()> {
    let svc_name = encode_wide(svc_name);
    let svc_name = PCWSTR::from_raw(svc_name.as_ptr());

    let args = args.iter().map(|a| encode_wide(a)).collect::<Vec<_>>();
    let args = args
        .iter()
        .map(|a| PCWSTR::from_raw(a.as_ptr()))
        .collect::<Vec<_>>();

    unsafe {
        let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT)?;
        let service = OpenServiceW(scm, svc_name, SERVICE_START)?;
        StartServiceW(service, Some(&args))?;
        CloseServiceHandle(service)?;
        CloseServiceHandle(scm)?;
    }

    Ok(())
}
fn stop_svc(svc_name: &str) -> Result<()> {
    let svc_name = encode_wide(svc_name);
    let svc_name = PCWSTR::from_raw(svc_name.as_ptr());

    unsafe {
        let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT)?;
        let service = OpenServiceW(scm, svc_name, SERVICE_STOP)?;
        let mut ret = SERVICE_STATUS::default();
        ControlService(service, SERVICE_CONTROL_STOP, &mut ret)?;
        CloseServiceHandle(service)?;
        CloseServiceHandle(scm)?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let command_or_svc_name = args.next().map(|a| a.to_string_lossy().to_string());
    let svc_name = args.next().map(|a| a.to_string_lossy().to_string());

    match command_or_svc_name.as_deref() {
        Some("install") => {
            install_svc(&svc_name.expect("Missing service name"))?;
        }
        Some("uninstall") => {
            uninstall_svc(&svc_name.expect("Missing service name"))?;
        }
        Some("start") => {
            start_svc(
                &svc_name.expect("Missing service name"),
                args.map(|a| a.to_string_lossy().to_string()).collect(),
            )?;
        }
        Some("stop") => {
            stop_svc(&svc_name.expect("Missing service name"))?;
        }
        _ => {
            start_svc_main(&command_or_svc_name.expect("Missing service_name"))?;
        }
    }

    Ok(())
}
