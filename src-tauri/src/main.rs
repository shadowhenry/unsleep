use std::sync::Mutex;

#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::{Child, Command, Stdio};

#[cfg(desktop)]
use tauri::{
  menu::{Menu, MenuItem},
  tray::TrayIconBuilder,
  AppHandle, Emitter, Manager,
};

#[derive(Default)]
struct PreventSleepState {
  #[cfg(any(target_os = "macos", target_os = "linux"))]
  child: Option<Child>,
  enabled: bool,
}

type SharedState = Mutex<PreventSleepState>;

#[tauri::command]
fn enable_prevent_sleep(state: tauri::State<'_, SharedState>) -> Result<(), String> {
  set_prevent_sleep(&state, true).map(|_| ())
}

#[tauri::command]
fn disable_prevent_sleep(state: tauri::State<'_, SharedState>) -> Result<(), String> {
  set_prevent_sleep(&state, false).map(|_| ())
}

fn set_prevent_sleep(state: &SharedState, enabled: bool) -> Result<bool, String> {
  let mut guard = state.lock().map_err(|_| "无法访问防睡眠状态".to_string())?;

  if guard.enabled == enabled {
    return Ok(guard.enabled);
  }

  if enabled {
    enable_platform_prevent_sleep(&mut guard)?;
  } else {
    disable_platform_prevent_sleep(&mut guard)?;
  }

  guard.enabled = enabled;
  Ok(guard.enabled)
}

fn is_prevent_sleep_enabled(state: &SharedState) -> Result<bool, String> {
  state
    .lock()
    .map(|guard| guard.enabled)
    .map_err(|_| "无法访问防睡眠状态".to_string())
}

#[cfg(desktop)]
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
  let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
  let toggle_item = MenuItem::with_id(app, "toggle_sleep", "打开防睡眠", true, None::<&str>)?;
  let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
  let menu = Menu::with_items(app, &[&show_item, &toggle_item, &quit_item])?;
  let toggle_item_for_event = toggle_item.clone();

  TrayIconBuilder::with_id("main")
    .icon(app.default_window_icon().cloned().unwrap_or_else(|| {
      tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
        .expect("valid tray icon")
    }))
    .tooltip("UnSleep")
    .menu(&menu)
    .show_menu_on_left_click(true)
    .on_menu_event(move |app, event| match event.id().as_ref() {
      "show" => {
        if let Some(window) = app.get_webview_window("main") {
          let _ = window.show();
          let _ = window.set_focus();
        }
      }
      "toggle_sleep" => {
        let state = app.state::<SharedState>();
        match is_prevent_sleep_enabled(&state).and_then(|enabled| {
          set_prevent_sleep(&state, !enabled)
        }) {
          Ok(enabled) => {
            let _ = toggle_item_for_event.set_text(if enabled {
              "关闭防睡眠"
            } else {
              "打开防睡眠"
            });
            let _ = app.emit("prevent-sleep-changed", enabled);
          }
          Err(error) => {
            let _ = app.emit("prevent-sleep-error", error);
          }
        }
      }
      "quit" => {
        let state = app.state::<SharedState>();
        let _ = set_prevent_sleep(&state, false);
        app.exit(0);
      }
      _ => {}
    })
    .build(app)?;

  Ok(())
}

#[cfg(target_os = "macos")]
fn enable_platform_prevent_sleep(state: &mut PreventSleepState) -> Result<(), String> {
  let child = Command::new("caffeinate")
    .args(["-dimsu"])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .map_err(|error| format!("启动 caffeinate 失败：{error}"))?;

  state.child = Some(child);
  Ok(())
}

#[cfg(target_os = "linux")]
fn enable_platform_prevent_sleep(state: &mut PreventSleepState) -> Result<(), String> {
  let child = Command::new("systemd-inhibit")
    .args([
      "--what=idle:sleep",
      "--why=UnSleep is keeping the computer awake",
      "--mode=block",
      "sleep",
      "infinity",
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .map_err(|error| format!("启动 systemd-inhibit 失败：{error}"))?;

  state.child = Some(child);
  Ok(())
}

#[cfg(windows)]
fn enable_platform_prevent_sleep(_state: &mut PreventSleepState) -> Result<(), String> {
  use windows_sys::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
  };

  let result = unsafe {
    SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED)
  };

  if result == 0 {
    return Err("调用 Windows 防睡眠接口失败".to_string());
  }

  Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn disable_platform_prevent_sleep(state: &mut PreventSleepState) -> Result<(), String> {
  if let Some(mut child) = state.child.take() {
    child
      .kill()
      .map_err(|error| format!("关闭防睡眠进程失败：{error}"))?;
    let _ = child.wait();
  }

  Ok(())
}

#[cfg(windows)]
fn disable_platform_prevent_sleep(_state: &mut PreventSleepState) -> Result<(), String> {
  use windows_sys::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS};

  let result = unsafe { SetThreadExecutionState(ES_CONTINUOUS) };

  if result == 0 {
    return Err("关闭 Windows 防睡眠接口失败".to_string());
  }

  Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn enable_platform_prevent_sleep(_state: &mut PreventSleepState) -> Result<(), String> {
  Err("当前系统暂不支持原生防睡眠".to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn disable_platform_prevent_sleep(_state: &mut PreventSleepState) -> Result<(), String> {
  Ok(())
}

pub fn run() {
  let builder = tauri::Builder::default()
    .manage(Mutex::new(PreventSleepState::default()));

  #[cfg(desktop)]
  let builder = builder
    .setup(|app| {
      // macOS 下以纯菜单栏应用运行（不出现在 Dock / Cmd+Tab，仅保留菜单栏图标）
      #[cfg(target_os = "macos")]
      app.set_activation_policy(tauri::ActivationPolicy::Accessory);
      setup_tray(app.handle())?;
      Ok(())
    });

  builder
    .on_window_event(|window, event| {
      // 关闭按钮（X）只隐藏主界面，不退出应用；退出只能通过菜单栏/托盘的“退出”
      if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
      }
    })
    .invoke_handler(tauri::generate_handler![
      enable_prevent_sleep,
      disable_prevent_sleep
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

fn main() {
  run();
}
