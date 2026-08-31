export function isTauriRuntime(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  return "__TAURI_INTERNALS__" in window || "__TAURI__" in window;
}

export function isAndroidTauri(): boolean {
  return import.meta.env.TAURI_ENV_PLATFORM === "android";
}

export function isDesktopTauri(): boolean {
  return isTauriRuntime() && !isAndroidTauri();
}
