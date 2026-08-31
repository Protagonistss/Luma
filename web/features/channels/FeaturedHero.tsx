import { useState } from 'react'

import { LiveIcon } from '@/shared/icons'
import { isRenderableLogoUrl } from '@/shared/media/logoUrl'
import type { Channel } from '@/shared/tauri/types'

interface FeaturedHeroProps {
  channel: Channel
  onPlay: () => void
}

function heroGradient(name: string) {
  let hash = 0
  for (const char of name) {
    hash = (hash * 31 + char.charCodeAt(0)) >>> 0
  }
  const palettes = [
    'linear-gradient(135deg, #1a2a3d 0%, #0c1018 55%, #101620 100%)',
    'linear-gradient(135deg, #221c38 0%, #0d0f16 55%, #121018 100%)',
    'linear-gradient(135deg, #152a28 0%, #0a0f0e 55%, #0e1412 100%)',
    'linear-gradient(135deg, #2e1e28 0%, #110c10 55%, #161012 100%)'
  ]
  return palettes[hash % palettes.length]
}

export function FeaturedHero({ channel, onPlay }: FeaturedHeroProps) {
  const [logoFailed, setLogoFailed] = useState(false)
  const logoUrl = isRenderableLogoUrl(channel.logo) ? channel.logo : null

  return (
    <button
      type="button"
      className="hero-card"
      style={{ background: heroGradient(channel.name) }}
      onClick={onPlay}
    >
      <div className="hero-card__mesh" aria-hidden />
      <div className="hero-card__content">
        <span className="hero-card__badge">
          <LiveIcon className="hero-card__live-icon" size={12} />
          继续观看
        </span>
        <h3>{channel.name}</h3>
        <p>{channel.group}</p>
        <span className="hero-card__cta">按 OK 播放</span>
      </div>
      <div className="hero-card__visual" aria-hidden>
        {logoUrl && !logoFailed ? (
          <img
            className="hero-card__logo"
            src={logoUrl}
            alt=""
            referrerPolicy="no-referrer"
            onError={() => setLogoFailed(true)}
          />
        ) : (
          <div className="hero-card__logo placeholder">
            {channel.name.slice(0, 1).toUpperCase()}
          </div>
        )}
      </div>
    </button>
  )
}
