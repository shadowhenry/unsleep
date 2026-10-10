use std::sync::Mutex;

#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::{Child, Command, Stdio};

#[cfg(desktop)]
use tauri::{
  menu::{CheckMenuItem, Menu, MenuItem},
  tray::TrayIconBuilder,
  Listener, Manager,
};
use tauri::{AppHandle, Emitter};

/// 电源模式决定“防睡眠”到底拦住哪些东西。
///
/// 长时间亮屏时最主要的发热来源就是屏幕本身（背光 + 面板），
/// 以及为了维持持唤醒而持续运行的显示和磁盘断言，因此提供两档策略：
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PowerMode {
  /// 屏幕常亮：阻止显示器与系统空闲睡眠（默认，适合需要一直盯着屏幕的场景）
  Screen,
  /// 节能挂机：只阻止系统睡眠，允许屏幕按系统设置变暗/熄灭（长时间挂机推荐，显著降耗）
  Eco,
}

impl Default for PowerMode {
  fn default() -> Self {
    Self::Screen
  }
}

impl PowerMode {
  fn parse(value: &str) -> Result<Self, String> {
    match value {
      "screen" => Ok(Self::Screen),
      "eco" => Ok(Self::Eco),
      other => Err(format!("未知的电源模式：{other}")),
    }
  }

  fn as_str(self) -> &'static str {
    match self {
      Self::Screen => "screen",
      Self::Eco => "eco",
    }
  }
}

#[derive(Default)]
struct PreventSleepState {
  #[cfg(any(target_os = "macos", target_os = "linux"))]
  child: Option<Child>,
  enabled: bool,
  mode: PowerMode,
}

type SharedState = Mutex<PreventSleepState>;

#[derive(serde::Serialize)]
struct PreventSleepStatus {
  enabled: bool,
  mode: &'static str,
}

impl PreventSleepState {
  fn status(&self) -> PreventSleepStatus {
    PreventSleepStatus {
      enabled: self.enabled,
      mode: self.mode.as_str(),
    }
  }
}

#[tauri::command]
fn enable_prevent_sleep(state: tauri::State<'_, SharedState>) -> Result<(), String> {
  set_prevent_sleep(&state, true).map(|_| ())
}

#[tauri::command]
fn disable_prevent_sleep(state: tauri::State<'_, SharedState>) -> Result<(), String> {
  set_prevent_sleep(&state, false).map(|_| ())
}

#[tauri::command]
fn get_prevent_sleep_status(
  state: tauri::State<'_, SharedState>,
) -> Result<PreventSleepStatus, String> {
  state
    .lock()
    .map(|guard| guard.status())
    .map_err(|_| "无法访问防睡眠状态".to_string())
}

/// 切换电源模式；如果防睡眠正在运行，会立刻用新的策略重启平台助手。
#[tauri::command]
fn set_power_mode(
  app: AppHandle,
  state: tauri::State<'_, SharedState>,
  mode: String,
) -> Result<PreventSleepStatus, String> {
  let parsed = PowerMode::parse(&mode)?;
  set_power_mode_internal(&app, &state, parsed)
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
fn preferences_path(app: &AppHandle) -> Option<std::path::PathBuf> {
  app
    .path()
    .app_config_dir()
    .ok()
    .map(|dir| dir.join("preferences.json"))
}

#[cfg(desktop)]
fn load_saved_power_mode(app: &AppHandle) -> PowerMode {
  preferences_path(app)
    .and_then(|path| std::fs::read_to_string(path).ok())
    .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
    .and_then(|value| {
      value
        .get("mode")
        .and_then(|mode| mode.as_str())
        .and_then(|mode| PowerMode::parse(mode).ok())
    })
    .unwrap_or_default()
}

#[cfg(desktop)]
fn save_power_mode(app: &AppHandle, mode: PowerMode) {
  let Some(path) = preferences_path(app) else {
    return;
  };

  if let Some(parent) = path.parent() {
    let _ = std::fs::create_dir_all(parent);
  }

  let _ = std::fs::write(path, format!("{{\"mode\":\"{}\"}}", mode.as_str()));
}

#[cfg(desktop)]
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
  let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
  let toggle_item = MenuItem::with_id(app, "toggle_sleep", "打开防睡眠", true, None::<&str>)?;
  let eco_check = load_saved_power_mode(app) == PowerMode::Eco;
  let eco_item = CheckMenuItem::with_id(
    app,
    "power_mode_eco",
    "节能挂机（允许屏幕变暗）",
    true,
    eco_check,
    None::<&str>,
  )?;
  let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
  let menu = Menu::with_items(app, &[&show_item, &toggle_item, &eco_item, &quit_item])?;
  let toggle_item_for_event = toggle_item.clone();
  let eco_item_for_listener = eco_item.clone();

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
        notify_window_active(app, true);
        if let Some(window) = app.get_webview_window("main") {
          let _ = window.show();
          let _ = window.set_focus();
        }
      }
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
      "power_mode_eco" => {
        // 以应用状态为准（勾选态随后由 power-mode-changed 事件同步，避免状态漂移）
        let state = app.state::<SharedState>();
        let next = if current_power_mode(&state) == PowerMode::Eco {
          PowerMode::Screen
        } else {
          PowerMode::Eco
        };
        let _ = set_power_mode_internal(app, &state, next);
      }
      "quit" => {
        let state = app.state::<SharedState>();
        let _ = set_prevent_sleep(&state, false);
        app.exit(0);
      }
      _ => {}
    })
    .build(app)?;

  // 无论模式是从托盘还是从界面切换，都保持勾选状态一致
  app.listen("power-mode-changed", move |event| {
    let eco = event.payload().contains("eco");
    let _ = eco_item_for_listener.set_checked(eco);
  });

  Ok(())
}

#[cfg(desktop)]
fn current_power_mode(state: &tauri::State<'_, SharedState>) -> PowerMode {
  state.lock().map(|guard| guard.mode).unwrap_or_default()
}

fn notify_window_active(app: &AppHandle, active: bool) {
  let _ = app.emit("window-active", active);
}

/// 托盘与命令共用的模式切换逻辑。
#[cfg(desktop)]
fn set_power_mode_internal(
  app: &AppHandle,
  state: &tauri::State<'_, SharedState>,
  parsed: PowerMode,
) -> Result<PreventSleepStatus, String> {
  let status = {
    let mut guard = state
      .lock()
      .map_err(|_| "无法访问防睡眠状态".to_string())?;

    if guard.mode != parsed {
      let was_enabled = guard.enabled;

      if was_enabled {
        disable_platform_prevent_sleep(&mut guard)?;
      }

      guard.mode = parsed;

      if was_enabled {
        if let Err(error) = enable_platform_prevent_sleep(&mut guard) {
          guard.enabled = false;
          let _ = app.emit("prevent-sleep-error", error.clone());
          return Err(error);
        }
      }
    }

    guard.status()
  };

  save_power_mode(app, parsed);
  let _ = app.emit("power-mode-changed", parsed.as_str());

  Ok(status)
}

#[cfg(target_os = "macos")]
fn enable_platform_prevent_sleep(state: &mut PreventSleepState) -> Result<(), String> {
  // -w <pid>：本应用退出（含异常退出）时 caffeinate 会自动结束，避免残留进程把机器锁在唤醒状态。
  let self_pid = std::process::id().to_string();
  let mut args = vec!["-w", self_pid.as_str()];

  match state.mode {
    // 屏幕常亮：拦显示器睡眠(-d) + 系统空闲睡眠(-i) + 用户活跃断言(-u)。
    // 不再使用 -m，避免磁盘被强制保持转动而持续耗电发热。
    PowerMode::Screen => args.extend(["-d", "-i", "-u"]),
    // 节能挂机：只拦系统睡眠，屏幕可以正常变暗/熄灭（发热主要来自屏幕本身）。
    PowerMode::Eco => args.extend(["-i", "-s"]),
  }

  let child = Command::new("caffeinate")
    .args(&args)
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
  let inhibit = match state.mode {
    PowerMode::Screen => "idle:sleep",
    PowerMode::Eco => "sleep",
  };

  let args = vec![
    format!("--what={inhibit}"),
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
fn enable_platform_prevent_sleep(state: &mut PreventSleepState) -> Result<(), String> {
  use windows_sys::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
  };

  let flags = match state.mode {
    // 屏幕常亮：同时要求显示器与系统保持唤醒
    PowerMode::Screen => ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED,
    // 节能挂机：只要求系统保持唤醒，显示器可按系统设置关闭
    PowerMode::Eco => ES_CONTINUOUS | ES_SYSTEM_REQUIRED,
  };

  let result = unsafe { SetThreadExecutionState(flags) };

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

      // 恢复上次选择的电源模式
      let saved = load_saved_power_mode(app.handle());
      {
        let state = app.state::<SharedState>();
        if let Ok(mut guard) = state.lock() {
          guard.mode = saved;
        };
      }

      setup_tray(app.handle())?;
      Ok(())
    });

  builder
    .on_window_event(|window, event| {
      match event {
        // 关闭按钮（X）只隐藏主界面，不退出应用；退出只能通过菜单栏/托盘的“退出”
        tauri::WindowEvent::CloseRequested { api, .. } => {
          api.prevent_close();
          let _ = window.hide();
          notify_window_active(window.app_handle(), false);
        }
        // 窗口真正不可见时前端进入低功耗待机；仅是失焦（窗口还开着）则保持刷新
        tauri::WindowEvent::Focused(focused) => {
          let visible = window.is_visible().unwrap_or(true);

          if !focused && !visible {
            notify_window_active(window.app_handle(), false);
          } else if *focused {
            notify_window_active(window.app_handle(), true);
          }
        }
        _ => {}
      }
    })
    .invoke_handler(tauri::generate_handler![
      enable_prevent_sleep,
      disable_prevent_sleep,
      get_prevent_sleep_status,
      set_power_mode
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

fn main() {
  run();
}
