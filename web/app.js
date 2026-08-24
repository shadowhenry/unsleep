const toggleButton = document.querySelector("#wakeToggle");
const buttonText = document.querySelector("#buttonText");
const statusText = document.querySelector("#statusText");
const supportText = document.querySelector("#supportText");
const elapsedTime = document.querySelector("#elapsedTime");

let wakeLock = null;
let isRequested = false;
let startedAt = null;
let timerId = null;

const nativeInvoke = window.__TAURI__?.core?.invoke;
const nativeListen = window.__TAURI__?.event?.listen;
const isNativeApp = Boolean(nativeInvoke);
const hasWakeLock = "wakeLock" in navigator;
const isSecure = window.isSecureContext;

function formatElapsed(milliseconds) {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000));
  const hours = String(Math.floor(totalSeconds / 3600)).padStart(2, "0");
  const minutes = String(Math.floor((totalSeconds % 3600) / 60)).padStart(2, "0");
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  return `${hours}:${minutes}:${seconds}`;
}

function updateTimer() {
  elapsedTime.textContent = startedAt ? formatElapsed(Date.now() - startedAt) : "00:00:00";
}

function startTimer() {
  if (!startedAt) {
    startedAt = Date.now();
  }

  updateTimer();
  if (!timerId) {
    timerId = window.setInterval(updateTimer, 1000);
  }
}

function stopTimer() {
  startedAt = null;
  if (timerId) {
    window.clearInterval(timerId);
    timerId = null;
  }
  updateTimer();
}

function setUi(active, message) {
  toggleButton.classList.toggle("is-on", active);
  toggleButton.setAttribute("aria-pressed", String(active));
  buttonText.textContent = active ? "关闭防睡眠" : "打开防睡眠";
  statusText.textContent = message || (active ? "防睡眠运行中" : "防睡眠已关闭");

  if (active) {
    startTimer();
  } else {
    stopTimer();
  }
}

function setSupportMessage(message) {
  supportText.textContent = message;
}

async function requestWakeLock() {
  if (isNativeApp) {
    try {
      await nativeInvoke("enable_prevent_sleep");
      isRequested = true;
      setUi(true, "防睡眠运行中");
      setSupportMessage("已启用系统级防睡眠，切到其他软件时仍会尽量保持生效。");
    } catch (error) {
      isRequested = false;
      setUi(false, "防睡眠开启失败");
      setSupportMessage(String(error));
    }
    return;
  }

  if (!hasWakeLock) {
    setSupportMessage("当前浏览器不支持 Screen Wake Lock API，请换用新版 Chrome、Edge 或 Android 浏览器。");
    return;
  }

  if (!isSecure) {
    setSupportMessage("防睡眠需要 HTTPS 或 localhost 环境。请用本地服务器打开，不要直接双击 HTML 文件。");
    return;
  }

  try {
    wakeLock = await navigator.wakeLock.request("screen");
    isRequested = true;
    setUi(true, "防睡眠运行中");
    setSupportMessage("保持此页面打开，屏幕将尽量保持常亮。");

    wakeLock.addEventListener("release", () => {
      wakeLock = null;
      if (isRequested && document.visibilityState === "visible") {
        setUi(false, "防睡眠被系统暂停，正在等待恢复");
      } else {
        setUi(false);
      }
    });
  } catch (error) {
    isRequested = false;
    setUi(false, "防睡眠开启失败");
    setSupportMessage(error.message || "系统拒绝了防睡眠请求。");
  }
}

async function releaseWakeLock() {
  isRequested = false;

  if (isNativeApp) {
    try {
      await nativeInvoke("disable_prevent_sleep");
      setUi(false);
      setSupportMessage("已关闭系统级防睡眠。");
    } catch (error) {
      setSupportMessage(String(error));
    }
    return;
  }

  if (wakeLock) {
    await wakeLock.release();
    wakeLock = null;
  }
  setUi(false);
  setSupportMessage("已关闭防睡眠。");
}

toggleButton.addEventListener("click", async () => {
  if (wakeLock) {
    await releaseWakeLock();
    return;
  }

  if (isNativeApp && isRequested) {
    await releaseWakeLock();
    return;
  }

  await requestWakeLock();
});

document.addEventListener("visibilitychange", async () => {
  if (isRequested && document.visibilityState === "visible" && !wakeLock) {
    await requestWakeLock();
  }
});

if (isNativeApp) {
  setSupportMessage("桌面模式准备就绪。");

  nativeListen?.("prevent-sleep-changed", (event) => {
    const active = Boolean(event.payload);
    isRequested = active;
    setUi(active, active ? "防睡眠运行中" : "防睡眠已关闭");
    setSupportMessage(active ? "已从菜单栏启用系统级防睡眠。" : "已从菜单栏关闭系统级防睡眠。");
  });

  nativeListen?.("prevent-sleep-error", (event) => {
    setSupportMessage(String(event.payload));
  });
} else if (!hasWakeLock) {
  setSupportMessage("提示：此浏览器可能不支持防睡眠 API。");
} else if (!isSecure) {
  setSupportMessage("提示：请通过 HTTPS 或 localhost 使用。");
} else {
  setSupportMessage("准备就绪。");
}
