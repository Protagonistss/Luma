import { invoke } from '@tauri-apps/api/core'

import type {
  Channel,
  ChannelGroup,
  CommandError,
  PlayChannelResponse,
  Playlist,
  ProbeReport,
  ProbeStatus,
  Subscription
} from './types'

function isCommandError(error: unknown): error is CommandError {
  return typeof error === 'object' && error !== null && 'code' in error && 'message' in error
}

/** User-facing copy per backend `AppError` code; raw English messages stay in the console. */
const ERROR_CODE_MESSAGES: Record<string, string> = {
  NETWORK: '网络请求失败，请检查网络后重试',
  INVALID_PLAYLIST: '播放列表无效或无法解析',
  FILE: '文件读取失败',
  NOT_FOUND: '数据不存在，可能播放列表已更新',
  STORAGE: '本地存储读写失败',
  PLAYBACK: '播放失败'
}

export function toUserMessage(error: unknown): string {
  if (isCommandError(error)) {
    console.warn(`[luma] command error ${error.code}: ${error.message}`)
    return ERROR_CODE_MESSAGES[error.code] ?? error.message
  }
  if (error instanceof Error) {
    return error.message
  }
  return '发生未知错误'
}

/** True when the failure just means "no playlist imported yet" (first run). */
export function isNoPlaylistError(error: unknown): boolean {
  return (
    isCommandError(error) && error.code === 'NOT_FOUND' && error.message.includes('no playlist')
  )
}

export const lumaApi = {
  addSubscriptionFromUrl(url: string, smartGrouping?: boolean) {
    return invoke<Playlist>('add_subscription_from_url', { url, smartGrouping })
  },
  addSubscriptionFromFile(
    path: string,
    displayName: string,
    content: string,
    smartGrouping?: boolean
  ) {
    return invoke<Playlist>('add_subscription_from_file', {
      path,
      displayName,
      content,
      smartGrouping
    })
  },
  listSubscriptions() {
    return invoke<Subscription[]>('list_subscriptions')
  },
  removeSubscription(id: string) {
    return invoke<Playlist>('remove_subscription', { id })
  },
  toggleSubscription(id: string, enabled: boolean) {
    return invoke<Playlist>('toggle_subscription', { id, enabled })
  },
  refreshPlaylist() {
    return invoke<Playlist>('refresh_playlist')
  },
  autoRefreshPlaylist(maxAgeSecs?: number) {
    return invoke<Playlist | null>('auto_refresh_playlist', { maxAgeSecs })
  },
  getProbeStatus() {
    return invoke<Record<string, ProbeStatus>>('get_probe_status')
  },
  getSmartGrouping() {
    return invoke<boolean>('get_smart_grouping')
  },
  setSmartGrouping(enabled: boolean) {
    return invoke<void>('set_smart_grouping', { enabled })
  },
  listChannels(group?: string) {
    return invoke<Channel[]>('list_channels', { group })
  },
  listGroups() {
    return invoke<ChannelGroup[]>('list_groups')
  },
  toggleFavorite(channelId: string) {
    return invoke<boolean>('toggle_favorite', { channelId })
  },
  listFavorites() {
    return invoke<Channel[]>('list_favorites')
  },
  listRecent() {
    return invoke<Channel[]>('list_recent')
  },
  playChannel(channelId: string) {
    return invoke<PlayChannelResponse>('play_channel', { channelId })
  },
  probeChannels(channelIds?: string[]) {
    return invoke<ProbeReport>('probe_channels', { channelIds })
  }
}
