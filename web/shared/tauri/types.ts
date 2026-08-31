export interface Channel {
  id: string
  name: string
  streamUrl: string
  group: string
  logo?: string | null
  tvgId?: string | null
  /** Per-channel request hints; UA-checked origins reject default clients. */
  userAgent?: string | null
  referrer?: string | null
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
  | { type: 'url'; url: string; displayUrl: string }
  | { type: 'file'; path: string; displayName: string }

/** One playlist subscription; the on-screen playlist merges every enabled
 * subscription, turning same-station channels into alternate lines. */
export interface Subscription {
  id: string
  source: PlaylistSource
  enabled: boolean
  importedAt: number
}

export interface PlayChannelResponse {
  channelId: string
  name: string
  streamUrl: string
  userAgent?: string | null
  referrer?: string | null
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
