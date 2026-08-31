import { useState } from 'react'

import { StarFilledIcon, StarIcon } from '@/shared/icons'
import { isRenderableLogoUrl } from '@/shared/media/logoUrl'
import type { Channel, ProbeStatus } from '@/shared/tauri/types'

import type { MergedChannel } from './channelSelectors'

interface ChannelCardProps {
  channel: Channel
  isFavorite: boolean
  probeStatus?: ProbeStatus
  probing?: boolean
  onPlay: (channelId: string) => void
  onToggleFavorite: (channelId: string) => void
}

function probeLabel(status?: ProbeStatus, probing?: boolean) {
  if (probing) {
    return '检测中'
  }
  switch (status) {
    case 'playable':
      return 'LIVE'
    case 'unreachable':
      return '离线'
    case 'invalidBody':
      return '无效'
    default:
      return null
  }
}

function posterTone(name: string) {
  let hash = 0
  for (const char of name) {
    hash = (hash * 31 + char.charCodeAt(0)) >>> 0
  }
  const palettes = [
    'linear-gradient(160deg, #1a2a3d 0%, #0c1018 100%)',
    'linear-gradient(160deg, #221c38 0%, #0d0f16 100%)',
    'linear-gradient(160deg, #152a28 0%, #0a0f0e 100%)',
    'linear-gradient(160deg, #2e1e28 0%, #110c10 100%)',
    'linear-gradient(160deg, #1a2434 0%, #0b0e14 100%)'
  ]
  return palettes[hash % palettes.length]
}

export function ChannelCard({
  channel,
  isFavorite,
  probeStatus,
  probing,
  onPlay,
  onToggleFavorite
}: ChannelCardProps) {
  const [logoFailed, setLogoFailed] = useState(false)
  const label = probeLabel(probeStatus, probing)
  const logoUrl = isRenderableLogoUrl(channel.logo) ? channel.logo : null
  const lineCount = (channel as MergedChannel).lines?.length ?? 1

  return (
    <div
      className="channel-card"
      data-channel-id={channel.id}
      role="button"
      tabIndex={0}
      onClick={() => onPlay(channel.id)}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault()
          onPlay(channel.id)
        }
        if (event.key.toLowerCase() === 'm') {
          event.preventDefault()
          event.stopPropagation()
          onToggleFavorite(channel.id)
        }
      }}
    >
      <div className="channel-poster" style={{ background: posterTone(channel.name) }}>
        {label ? <span className={`probe-badge ${probeStatus ?? 'checking'}`}>{label}</span> : null}
        {lineCount > 1 ? (
          <span className="channel-lines-badge" title={`${lineCount} 路直播源`}>
            {lineCount}路
          </span>
        ) : null}
        <button
          type="button"
          className={`favorite-button ${isFavorite ? 'active' : ''}`}
          aria-label={isFavorite ? '取消收藏' : '收藏'}
          tabIndex={-1}
          onClick={(event) => {
            event.stopPropagation()
            onToggleFavorite(channel.id)
          }}
        >
          {isFavorite ? <StarFilledIcon size={16} /> : <StarIcon size={16} />}
        </button>
        {logoUrl && !logoFailed ? (
          <img
            className="channel-logo"
            src={logoUrl}
            alt=""
            loading="lazy"
            referrerPolicy="no-referrer"
            onError={() => setLogoFailed(true)}
          />
        ) : (
          <div className="channel-logo placeholder" aria-hidden>
            {channel.name.slice(0, 1).toUpperCase()}
          </div>
        )}
      </div>
      <div className="channel-caption">
        <strong>{channel.name}</strong>
        <span>{channel.group}</span>
      </div>
    </div>
  )
}
