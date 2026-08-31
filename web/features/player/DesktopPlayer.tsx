import Hls from 'hls.js'
import { useEffect, useRef, useState } from 'react'

import { resolveDesktopStreamUrl } from '@/shared/tauri/player'
import type { PlayChannelResponse } from '@/shared/tauri/types'

interface DesktopPlayerProps {
  /** The line currently being played: `lines[lineIndex]`. */
  channel: PlayChannelResponse
  /** All sources of this station, best-known line first. */
  lines: PlayChannelResponse[]
  lineIndex: number
  onSwitchLine: (index: number) => void
  onClose: () => void
}

export function DesktopPlayer({
  channel,
  lines,
  lineIndex,
  onSwitchLine,
  onClose
}: DesktopPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const hlsRef = useRef<Hls | null>(null)
  const [status, setStatus] = useState('正在连接直播...')
  const [error, setError] = useState<string | null>(null)
  const [showChrome, setShowChrome] = useState(true)
  // Fatal errors arrive inside hls.js callbacks; keep the failover callback
  // in a ref so the playback effect stays keyed on the stream URL only.
  const switchLineRef = useRef(onSwitchLine)
  switchLineRef.current = onSwitchLine

  const failover = useRef(() => {})
  failover.current = () => {
    if (lineIndex + 1 < lines.length) {
      setStatus(`线路中断，切换到线路 ${lineIndex + 2}/${lines.length}...`)
      setError(null)
      switchLineRef.current(lineIndex + 1)
    } else {
      setError('播放失败：所有线路均不可用，请稍后重试或重新检测频道')
      setStatus('')
    }
  }

  useEffect(() => {
    const video = videoRef.current
    if (!video) {
      return
    }

    let cancelled = false
    let playbackUrl = channel.streamUrl
    setStatus('正在连接直播...')
    setError(null)

    const cleanup = () => {
      hlsRef.current?.destroy()
      hlsRef.current = null
      video.pause()
      video.removeAttribute('src')
      video.load()
    }

    const startPlayback = async () => {
      try {
        playbackUrl = await resolveDesktopStreamUrl(
          channel.streamUrl,
          channel.userAgent,
          channel.referrer
        )
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : '无法启动本地流代理')
          setStatus('')
        }
        return
      }

      if (cancelled) {
        return
      }

      if (!Hls.isSupported()) {
        if (video.canPlayType('application/vnd.apple.mpegurl')) {
          video.src = playbackUrl
          video.addEventListener(
            'loadedmetadata',
            () => {
              if (cancelled) {
                return
              }
              setStatus('')
              void video.play().catch(() => {
                setError('自动播放失败，请点击视频开始播放')
              })
            },
            { once: true }
          )
          video.addEventListener(
            'error',
            () => {
              if (!cancelled) {
                setError('播放失败，请检查流地址是否有效')
                setStatus('')
              }
            },
            { once: true }
          )
          return
        }

        setError('当前环境不支持 HLS 播放')
        setStatus('')
        return
      }

      const hls = new Hls({
        enableWorker: true,
        // Everything played here is a live stream, so opt into LL-HLS handling
        // whenever the source supports it.
        lowLatencyMode: true,
        xhrSetup(xhr) {
          xhr.withCredentials = false
        }
      })
      hlsRef.current = hls
      hls.attachMedia(video)
      hls.on(Hls.Events.MEDIA_ATTACHED, () => {
        hls.loadSource(playbackUrl)
      })
      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        if (cancelled) {
          return
        }
        setStatus('')
        void video.play().catch(() => {
          setError('自动播放失败，请点击视频开始播放')
        })
      })
      hls.on(Hls.Events.ERROR, (_event, data) => {
        if (cancelled || !data.fatal) {
          return
        }

        if (data.type === Hls.ErrorTypes.NETWORK_ERROR) {
          setStatus('网络异常，正在重试...')
          hls.startLoad()
          return
        }

        if (data.type === Hls.ErrorTypes.MEDIA_ERROR) {
          setStatus('媒体解码异常，正在恢复...')
          hls.recoverMediaError()
          return
        }

        // Unrecoverable: try the next source of this station before giving up.
        failover.current()
      })
    }

    void startPlayback()

    return () => {
      cancelled = true
      cleanup()
    }
  }, [channel.streamUrl, channel.name])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' || event.key === 'Backspace') {
        event.preventDefault()
        onClose()
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [onClose])

  useEffect(() => {
    let timer: number | undefined
    const resetTimer = () => {
      setShowChrome(true)
      if (timer) {
        window.clearTimeout(timer)
      }
      timer = window.setTimeout(() => setShowChrome(false), 4000)
    }

    resetTimer()
    window.addEventListener('mousemove', resetTimer)
    window.addEventListener('keydown', resetTimer)

    return () => {
      if (timer) {
        window.clearTimeout(timer)
      }
      window.removeEventListener('mousemove', resetTimer)
      window.removeEventListener('keydown', resetTimer)
    }
  }, [])

  return (
    <div className="desktop-player">
      <div className="desktop-player-stage">
        <video
          ref={videoRef}
          className="desktop-player-video"
          playsInline
          autoPlay
          crossOrigin="anonymous"
        />
      </div>

      <div className={`desktop-player-chrome ${showChrome ? 'visible' : ''}`}>
        <div className="desktop-player-chrome__left">
          <button type="button" className="player-back-button" onClick={onClose}>
            返回
          </button>
          <div className="player-info">
            <strong>{channel.name}</strong>
            {lines.length > 1 ? (
              <span className="player-lines">
                {lines.map((line, index) => (
                  <button
                    key={line.channelId}
                    type="button"
                    className={`player-line-pill ${index === lineIndex ? 'active' : ''}`}
                    onClick={() => onSwitchLine(index)}
                  >
                    线路{index + 1}
                  </button>
                ))}
              </span>
            ) : null}
          </div>
        </div>
      </div>

      {status ? <div className="desktop-player-overlay">{status}</div> : null}
      {error ? (
        <div className="desktop-player-overlay desktop-player-error">
          <p>{error}</p>
          <button type="button" className="ghost-button" onClick={onClose}>
            返回列表
          </button>
        </div>
      ) : null}
    </div>
  )
}
