use std::sync::Mutex;

#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::{Child, Command, Stdio};

#[cfg(desktop)]
use tauri::{
  menu::{Menu, MenuItem},
  tray::TrayIconBuilder,
  Manager,
};
use tauri::{AppHandle, Emitter};

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
  let toggle_item = MenuItem::with_id(app, "toggle_sleep", "打开防睡眠", true, None::<&str>)?;
  let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
  let menu = Menu::with_items(app, &[&toggle_item, &quit_item])?;
  let toggle_item_for_event = toggle_item.clone();

  // 菜单栏图标：用 1024px 源图，系统会按 18pt 高度缩放，Retina 下依然清晰
  let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
    .expect("valid tray icon");

  TrayIconBuilder::with_id("main")
    .icon(tray_icon)
    .tooltip("UnSleep")
    .menu(&menu)
    .show_menu_on_left_click(true)
    .on_menu_event(move |app, event| match event.id().as_ref() {
      "toggle_sleep" => {
        let state = app.state::<SharedState>();
        match is_prevent_sleep_enabled(&state).and_then(|enabled| set_prevent_sleep(&state, !enabled))
        {
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
  // -w <pid>：本应用退出（含异常退出）时 caffeinate 随之结束，避免残留进程把机器锁在唤醒状态。
  // 只拦显示器(-d)、系统空闲睡眠(-i)与用户活跃断言(-u)。
  // 不用 -m，避免磁盘被强制保持转动而持续耗电发热。
  let self_pid = std::process::id().to_string();
  let args = ["-w", self_pid.as_str(), "-d", "-i", "-u"];

  let child = Command::new("caffeinate")
    .args(args)
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
  let args = vec![
    "--what=idle:sleep".to_string(),
    "--why=UnSleep is keeping the computer awake".to_string(),
    "--mode=block".to_string(),
    "sleep".to_string(),
    "infinity".to_string(),
  ];

  let child = Command::new("systemd-inhibit")
    .args(&args)
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
      // 纯菜单栏应用：不创建任何窗口，也不出现在 Dock / Cmd+Tab
      #[cfg(target_os = "macos")]
      app.set_activation_policy(tauri::ActivationPolicy::Accessory);
      setup_tray(app.handle())?;
      Ok(())
    });

  builder
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
