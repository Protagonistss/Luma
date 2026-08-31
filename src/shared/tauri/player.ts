import { invoke } from "@tauri-apps/api/core";

import type { PlayChannelResponse } from "./types";

export async function openNativePlayer(
  payload: PlayChannelResponse,
): Promise<void> {
  try {
    await invoke("plugin:player|open_player", {
      payload: {
        channelId: payload.channelId,
        name: payload.name,
        streamUrl: payload.streamUrl,
      },
    });
  } catch {
    if (import.meta.env.DEV) {
      window.open(payload.streamUrl, "_blank", "noopener,noreferrer");
      return;
    }
    throw new Error("当前平台不支持原生播放器");
  }
}
