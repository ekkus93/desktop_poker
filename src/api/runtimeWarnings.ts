import { invoke } from "@tauri-apps/api/core";

function hasTauriRuntime() {
  if (typeof window === "undefined") {
    return false;
  }

  const runtimeWindow = window as Window & {
    __TAURI__?: unknown;
    __TAURI_INTERNALS__?: unknown;
  };
  return Boolean(runtimeWindow.__TAURI__ || runtimeWindow.__TAURI_INTERNALS__);
}

export async function getRuntimeWarnings(): Promise<string[]> {
  if (!hasTauriRuntime()) {
    return [];
  }

  return invoke<string[]>("get_runtime_warnings");
}
