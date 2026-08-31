import { useEffect, useState } from 'react'

import { ChevronRightIcon, FileIcon, ImportIcon, RefreshIcon, TrashIcon } from '@/shared/icons'
import { lumaApi, toUserMessage } from '@/shared/tauri/api'
import type { Subscription } from '@/shared/tauri/types'

interface ImportPlaylistPanelProps {
  onImported: () => void
}

/** Public playlist presets verified against real hosts. */
const PLAYLIST_PRESETS = [
  {
    id: 'vbsky',
    label: '国内频道',
    detail: '每日更新 · 带台标/EPG',
    url: 'https://raw.githubusercontent.com/vbskycn/iptv/master/tv/iptv4.m3u'
  },
  {
    id: 'suxuang',
    label: '卫视补全',
    detail: '补齐31省卫视 · 带EPG',
    url: 'https://raw.githubusercontent.com/suxuang/myIPTV/main/APTV手机专享.m3u'
  },
  {
    id: 'cn',
    label: '国内备选',
    detail: 'iptv-org · 国际社区维护',
    url: 'https://iptv-org.github.io/iptv/countries/cn.m3u'
  },
  {
    id: 'zho',
    label: '中文频道',
    detail: 'iptv-org · 含港澳台',
    url: 'https://iptv-org.github.io/iptv/languages/zho.m3u'
  }
] as const

function sourceTitle(subscription: Subscription) {
  return subscription.source.type === 'url'
    ? subscription.source.url
    : subscription.source.displayName
}

function sourceKind(subscription: Subscription) {
  return subscription.source.type === 'url' ? '网络' : '本地'
}

function importedAtLabel(subscription: Subscription) {
  if (!subscription.importedAt) {
    return '尚未更新'
  }
  const date = new Date(subscription.importedAt * 1000)
  return `更新于 ${date.toLocaleDateString()} ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
}

export function ImportPlaylistPanel({ onImported }: ImportPlaylistPanelProps) {
  const [url, setUrl] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [smartGrouping, setSmartGrouping] = useState(true)
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([])

  const reloadSubscriptions = () => {
    lumaApi
      .listSubscriptions()
      .then(setSubscriptions)
      .catch(() => undefined)
  }

  // Mirror the persisted backend state so the UI survives restarts.
  useEffect(() => {
    lumaApi
      .getSmartGrouping()
      .then(setSmartGrouping)
      .catch(() => undefined)
    reloadSubscriptions()
  }, [])

  const toggleSmartGrouping = (next: boolean) => {
    setSmartGrouping(next)
    void lumaApi.setSmartGrouping(next).catch(() => undefined)
  }

  const addFromUrl = async (targetUrl?: string) => {
    const effectiveUrl = (targetUrl ?? url).trim()
    if (!effectiveUrl) {
      return
    }
    setLoading(true)
    setError(null)
    setMessage(null)
    try {
      const playlist = await lumaApi.addSubscriptionFromUrl(effectiveUrl, smartGrouping)
      setMessage(`已订阅，当前共 ${playlist.channels.length} 个频道`)
      reloadSubscriptions()
      onImported()
    } catch (err) {
      setError(toUserMessage(err))
    } finally {
      setLoading(false)
    }
  }

  const addFromFile = async () => {
    setLoading(true)
    setError(null)
    setMessage(null)
    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const { readTextFile } = await import('@tauri-apps/plugin-fs')
      const selected = await open({
        multiple: false,
        filters: [{ name: 'M3U Playlist', extensions: ['m3u', 'm3u8', 'txt'] }]
      })

      if (!selected || Array.isArray(selected)) {
        setLoading(false)
        return
      }

      const content = await readTextFile(selected)
      const displayName = selected.split(/[\\/]/).pop() ?? 'playlist.m3u'
      const playlist = await lumaApi.addSubscriptionFromFile(
        selected,
        displayName,
        content,
        smartGrouping
      )
      setMessage(`已订阅，当前共 ${playlist.channels.length} 个频道`)
      reloadSubscriptions()
      onImported()
    } catch (err) {
      setError(toUserMessage(err))
    } finally {
      setLoading(false)
    }
  }

  const refreshAll = async () => {
    setLoading(true)
    setError(null)
    setMessage(null)
    try {
      const playlist = await lumaApi.refreshPlaylist()
      setMessage(`已刷新，当前共 ${playlist.channels.length} 个频道`)
      reloadSubscriptions()
      onImported()
    } catch (err) {
      setError(toUserMessage(err))
    } finally {
      setLoading(false)
    }
  }

  const remove = async (id: string) => {
    setLoading(true)
    setError(null)
    try {
      await lumaApi.removeSubscription(id)
      reloadSubscriptions()
      onImported()
    } catch (err) {
      setError(toUserMessage(err))
    } finally {
      setLoading(false)
    }
  }

  const toggle = async (id: string, enabled: boolean) => {
    // Optimistic flip; reload to resync with backend state.
    setSubscriptions((current) =>
      current.map((item) => (item.id === id ? { ...item, enabled } : item))
    )
    try {
      await lumaApi.toggleSubscription(id, enabled)
      reloadSubscriptions()
      onImported()
    } catch (err) {
      setError(toUserMessage(err))
      reloadSubscriptions()
    }
  }

  return (
    <section className="settings-stage">
      <header className="settings-header">
        <p className="kicker">设置</p>
        <h2>订阅管理</h2>
        <p className="settings-desc">
          可同时订阅多个播放列表，同频道源会自动合并为多线路。仅导入你拥有合法使用权的列表；Luma
          不提供节目源。
        </p>
      </header>

      {error ? <div className="import-feedback import-feedback--error">{error}</div> : null}
      {message ? <div className="import-feedback import-feedback--success">{message}</div> : null}

      <div className="import-form">
        {subscriptions.length > 0 ? (
          <div className="subscription-list">
            <span className="import-label">我的订阅（{subscriptions.length}）</span>
            {subscriptions.map((subscription) => (
              <div key={subscription.id} className="subscription-row">
                <label className="subscription-toggle">
                  <input
                    type="checkbox"
                    checked={subscription.enabled}
                    disabled={loading}
                    onChange={(event) => void toggle(subscription.id, event.target.checked)}
                  />
                </label>
                <span className="subscription-row__text">
                  <strong>{sourceTitle(subscription)}</strong>
                  <span>
                    {sourceKind(subscription)} · {importedAtLabel(subscription)}
                  </span>
                </span>
                <button
                  type="button"
                  className="subscription-remove"
                  aria-label="取消订阅"
                  disabled={loading}
                  onClick={() => void remove(subscription.id)}
                >
                  <TrashIcon size={16} />
                </button>
              </div>
            ))}
          </div>
        ) : null}

        <div className="import-presets">
          <span className="import-presets__label">推荐源</span>
          <div className="import-presets__list">
            {PLAYLIST_PRESETS.map((preset) => {
              const subscribed = subscriptions.some(
                (item) => item.source.type === 'url' && item.source.url === preset.url
              )
              return (
                <button
                  key={preset.id}
                  type="button"
                  className="preset-chip"
                  disabled={loading}
                  onClick={() => {
                    if (subscribed) {
                      setMessage('该源已在订阅中')
                      return
                    }
                    setUrl(preset.url)
                    void addFromUrl(preset.url)
                  }}
                >
                  <strong>
                    {preset.label}
                    {subscribed ? ' ✓' : ''}
                  </strong>
                  <span>{preset.detail}</span>
                </button>
              )
            })}
          </div>
        </div>

        <label className="import-url-group">
          <span className="import-label">添加订阅地址</span>
          <div className="import-url-row">
            <input
              className="import-url-input"
              value={url}
              onChange={(event) => setUrl(event.target.value)}
              placeholder="https://example.com/playlist.m3u"
              disabled={loading}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && url.trim() && !loading) {
                  void addFromUrl()
                }
              }}
            />
            <button
              type="button"
              className="primary-button import-url-submit"
              disabled={loading || !url.trim()}
              onClick={() => void addFromUrl()}
            >
              <ImportIcon size={16} />
              {loading ? '处理中' : '订阅'}
            </button>
          </div>
        </label>

        <label className="import-toggle">
          <input
            type="checkbox"
            checked={smartGrouping}
            disabled={loading}
            onChange={(event) => toggleSmartGrouping(event.target.checked)}
          />
          <span className="import-toggle__text">
            <strong>智能分组</strong>
            <span>
              清洗频道名（去画质/失效标记、繁转简）并按 央视 / 卫视 / 港澳台
              归组排序，适合国内频道列表
            </span>
          </span>
        </label>

        <div className="import-divider" aria-hidden />

        <div className="import-options">
          <button type="button" className="import-option" disabled={loading} onClick={addFromFile}>
            <span className="import-option__leading">
              <FileIcon size={18} />
            </span>
            <span className="import-option__text">
              <strong>订阅本地文件</strong>
              <span>m3u · m3u8 · txt</span>
            </span>
            <ChevronRightIcon className="import-option__arrow" size={18} />
          </button>
          <button type="button" className="import-option" disabled={loading} onClick={refreshAll}>
            <span className="import-option__leading">
              <RefreshIcon size={18} />
            </span>
            <span className="import-option__text">
              <strong>立即刷新全部订阅</strong>
              <span>重新下载所有已订阅列表</span>
            </span>
            <ChevronRightIcon className="import-option__arrow" size={18} />
          </button>
        </div>
      </div>
    </section>
  )
}
