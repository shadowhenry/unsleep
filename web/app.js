const toggleButton = document.querySelector("#wakeToggle");
const statusText = document.querySelector("#statusText");
const statusBadge = document.querySelector("#statusBadge");
const supportText = document.querySelector("#supportText");
const elapsedTime = document.querySelector("#elapsedTime");
const switchTrack = document.querySelector("#switchTrack");

let wakeLock = null;
let isRequested = false;
let startedAt = null;
let timerId = null;
let idleId = null;
let isIdle = false;
let isSuspended = false;
let lastActivityAt = 0;

const IDLE_DELAY = 20000;

const nativeInvoke = window.__TAURI__?.core?.invoke;
const nativeListen = window.__TAURI__?.event?.listen;
const isNativeApp = Boolean(nativeInvoke);
const hasWakeLock = "wakeLock" in navigator;
const isSecure = window.isSecureContext;

switchTrack.dataset.onLabel = "开";
switchTrack.dataset.offLabel = "关";

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

  if (!timerId && !isSuspended) {
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

/* ------------------------------------------------------------------
 * 省电处理（不改变界面，只减少持续重绘带来的发热）：
 * 空闲 20 秒后停掉循环动画与实时背景模糊；
 * 窗口被隐藏（只留菜单栏图标）时整个前端停摆。
 * ---------------------------------------------------------------- */

function setIdle(next) {
  if (isIdle === next) {
    return;
  }

  isIdle = next;
  document.body.classList.toggle("is-idle", isIdle);
}

function noteActivity() {
  const now = Date.now();

  // 1 秒节流：鼠标移动很密集，没必要每帧重排定时器
  if (now - lastActivityAt < 1000) {
    return;
  }

  lastActivityAt = now;
  setIdle(false);

  if (idleId) {
    window.clearTimeout(idleId);
  }

  idleId = window.setTimeout(() => setIdle(true), IDLE_DELAY);
}

function setSuspended(next) {
  if (isSuspended === next) {
    return;
  }

  isSuspended = next;
  document.body.classList.toggle("is-suspended", isSuspended);

  if (isSuspended) {
    if (timerId) {
      window.clearInterval(timerId);
      timerId = null;
    }
    return;
  }

  // 回到前台时按真实时间戳补正，不会因暂停而走偏
  updateTimer();

  if (startedAt && !timerId) {
    timerId = window.setInterval(updateTimer, 1000);
  }
}

["pointermove", "pointerdown", "keydown", "wheel", "touchstart"].forEach((type) => {
  window.addEventListener(type, noteActivity, { passive: true });
});

function setBusy(busy) {
  toggleButton.classList.toggle("is-busy", busy);
  toggleButton.disabled = busy;
  toggleButton.setAttribute("aria-busy", String(busy));
}

function setUi(active, message) {
  toggleButton.classList.toggle("is-on", active);
  statusBadge.classList.toggle("is-on", active);
  toggleButton.setAttribute("aria-pressed", String(active));
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
      setBusy(true);
      toggleButton.classList.add("is-on");
      await nativeInvoke("enable_prevent_sleep");
      isRequested = true;
      setUi(true, "防睡眠运行中");
      setSupportMessage("已启用系统级防睡眠");
    } catch (error) {
      isRequested = false;
      setUi(false, "防睡眠开启失败");
      setSupportMessage(String(error));
    } finally {
      setBusy(false);
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
    setBusy(true);
    toggleButton.classList.add("is-on");
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
  } finally {
    setBusy(false);
  }
}

async function releaseWakeLock() {
  isRequested = false;

  if (isNativeApp) {
    try {
      setBusy(true);
      toggleButton.classList.remove("is-on");
      await nativeInvoke("disable_prevent_sleep");
      setUi(false);
      setSupportMessage("已关闭系统级防睡眠。");
    } catch (error) {
      setSupportMessage(String(error));
    } finally {
      setBusy(false);
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
  setSuspended(document.hidden);

  if (!document.hidden && isRequested && !isNativeApp && !wakeLock) {
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

  // 窗口被隐藏（仅留菜单栏图标）时彻底停掉前端刷新
  nativeListen?.("window-active", (event) => {
    setSuspended(!Boolean(event.payload));
  });
} else if (!hasWakeLock) {
  setSupportMessage("提示：此浏览器可能不支持防睡眠 API。");
} else if (!isSecure) {
  setSupportMessage("提示：请通过 HTTPS 或 localhost 使用。");
} else {
  setSupportMessage("准备就绪。");
}

noteActivity();
