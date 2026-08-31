export interface Channel {
  id: string
  name: string
  streamUrl: string
  group: string
  logo?: string | null
  tvgId?: string | null
}

export interface ChannelGroup {
  name: string
  channelCount: number
}

export interface Playlist {
  channels: Channel[]
  importedAt: string
}

export type PlaylistSource =
  | { type: 'Url'; url: string; displayUrl: string }
  | { type: 'File'; path: string; displayName: string }

export interface PlayChannelResponse {
  channelId: string
  name: string
  streamUrl: string
}

export interface CommandError {
  code: string
  message: string
}

export type ProbeStatus = 'playable' | 'unreachable' | 'invalidBody'

export interface ChannelProbeResult {
  channelId: string
  status: ProbeStatus
  latencyMs?: number | null
  message?: string | null
}

export interface ProbeReport {
  total: number
  playable: number
  unreachable: number
  invalid: number
  results: ChannelProbeResult[]
}
