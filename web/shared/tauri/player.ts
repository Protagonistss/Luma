import { invoke } from '@tauri-apps/api/core'

import { isAndroidTauri, isTauriRuntime } from '@/shared/platform'

import type { PlayChannelResponse } from './types'

export function shouldUseDesktopPlayer(): boolean {
  return !isAndroidTauri()
}

export async function resolveDesktopStreamUrl(streamUrl: string): Promise<string> {
  if (!isTauriRuntime()) {
    return streamUrl
  }

  try {
    return await invoke<string>('get_proxied_stream_url', { streamUrl })
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    throw new Error(`无法启动本地流代理：${message}`, { cause: err })
  }
}

export async function openNativePlayer(payload: PlayChannelResponse): Promise<void> {
  if (shouldUseDesktopPlayer()) {
    return
  }

  await invoke('plugin:player|open_player', {
    payload: {
      channelId: payload.channelId,
      name: payload.name,
      streamUrl: payload.streamUrl
    }
  })
}
