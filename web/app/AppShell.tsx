import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from 'react'

import { CategoryPanel } from '@/features/channels/CategoryPanel'
import { ChannelGrid } from '@/features/channels/ChannelGrid'
import {
  buildLineIndex,
  filterChannelsBySection,
  mergeChannelLines
} from '@/features/channels/channelSelectors'
import type { SidebarSection } from '@/features/channels/channelSelectors'
import { Sidebar } from '@/features/channels/Sidebar'
import { ImportPlaylistPanel } from '@/features/import-playlist/ImportPlaylistPanel'
import { useTvNavigation } from '@/shared/focus/useTvNavigation'
import { lumaApi, toUserMessage } from '@/shared/tauri/api'
import { openNativePlayer, shouldUseDesktopPlayer } from '@/shared/tauri/player'
import type { PlayChannelResponse } from '@/shared/tauri/types'
import { ScrollArea } from '@/shared/ui/ScrollArea'

import { useChannelProbe } from './useChannelProbe'
import { useClock } from './useClock'
import { usePlaylistData } from './usePlaylistData'

// hls.js weighs several hundred KB and is only needed for desktop playback,
// so keep it out of the initial bundle.
const DesktopPlayer = lazy(() =>
  import('@/features/player/DesktopPlayer').then((module) => ({
    default: module.DesktopPlayer
  }))
)

type View = 'home' | 'settings'

/** One full-screen playback: the ordered line list of the station being
 * watched plus the currently active line index. */
interface PlaySession {
  lines: PlayChannelResponse[]
  index: number
}

export function AppShell() {
  const [view, setView] = useState<View>('home')
  const [section, setSection] = useState<SidebarSection>('all')
  const [activeGroup, setActiveGroup] = useState<string | null>(null)
  const [toast, setToast] = useState<string | null>(null)
  const [lastFocusedChannelId, setLastFocusedChannelId] = useState<string | null>(null)
  const [playSession, setPlaySession] = useState<PlaySession | null>(null)
  const [hideUnavailable, setHideUnavailable] = useState(false)

  const probe = useChannelProbe()
  const handleAutoRefreshed = useCallback(() => {
    void probe.runProbe()
  }, [probe.runProbe])
  const playlist = usePlaylistData(handleAutoRefreshed)
  const onKeyDown = useTvNavigation()
  const clock = useClock()

  // All channels merged by station: duplicate sources become one card, and
  // every id maps to its ordered line list for playback failover.
  const lineIndex = useMemo(
    () => buildLineIndex(mergeChannelLines(playlist.channels, probe.probeStatusById)),
    [playlist.channels, probe.probeStatusById]
  )

  const visibleChannels = useMemo(
    () =>
      mergeChannelLines(
        filterChannelsBySection(
          section,
          activeGroup,
          playlist.channels,
          playlist.favorites,
          playlist.recent
        ),
        probe.probeStatusById
      ),
    [
      section,
      activeGroup,
      playlist.channels,
      playlist.favorites,
      playlist.recent,
      probe.probeStatusById
    ]
  )

  useEffect(() => {
    if (!toast) {
      return
    }
    const timer = window.setTimeout(() => setToast(null), 3000)
    return () => window.clearTimeout(timer)
  }, [toast])

  // Restore focus to the last played channel card after views/sections swap.
  useEffect(() => {
    if (!lastFocusedChannelId) {
      return
    }
    const element = document.querySelector<HTMLElement>(
      `[data-channel-id="${lastFocusedChannelId}"]`
    )
    element?.focus()
  }, [playlist.channels, lastFocusedChannelId, view, section, activeGroup])

  const handlePlay = async (channelId: string) => {
    setLastFocusedChannelId(channelId)
    try {
      const payload = await lumaApi.playChannel(channelId)
      if (shouldUseDesktopPlayer()) {
        // Failover list: the clicked line first, then siblings ordered by
        // probe health (best line first).
        const siblings = (lineIndex.get(channelId) ?? [])
          .filter((line) => line.id !== payload.channelId)
          .map((line) => ({
            channelId: line.id,
            name: line.name,
            streamUrl: line.streamUrl,
            userAgent: line.userAgent ?? null,
            referrer: line.referrer ?? null
          }))
        setPlaySession({ lines: [payload, ...siblings], index: 0 })
      } else {
        await openNativePlayer(payload)
      }
      await playlist.refreshRecent()
    } catch (err) {
      setToast(toUserMessage(err))
    }
  }

  const playSessionRef = useRef<PlaySession | null>(null)
  playSessionRef.current = playSession

  const handleSwitchLine = useCallback(
    (index: number) => {
      const session = playSessionRef.current
      if (!session || index < 0 || index >= session.lines.length) {
        return
      }
      setPlaySession({ lines: session.lines, index })
      // Record the new line as recently watched (best-effort).
      void lumaApi.playChannel(session.lines[index]!.channelId).catch(() => undefined)
      void playlist.refreshRecent()
    },
    [playlist]
  )

  const handleToggleFavorite = async (channelId: string) => {
    const message = await playlist.toggleFavorite(channelId)
    if (message) {
      setToast(message)
    }
  }

  const runProbe = async (channelIds?: string[]) => {
    const message = await probe.runProbe(channelIds)
    setToast(message)
  }

  // Any import / subscribe / refresh changes the channel set, so a probe
  // follows automatically: dead lines get badges, 「隐藏不可用」 has data to
  // act on, and failover ordering picks the best-known line. Without this,
  // manually added subscriptions never get probed (auto-refresh only fires
  // for lists older than 24h).
  const handleImported = useCallback(() => {
    setView('home')
    void playlist.load()
    void runProbe()
  }, [playlist])

  return (
    <div
      className={`app-shell ${
        view === 'home' && (section === 'all' || section === 'group') && playlist.groups.length > 0
          ? 'with-categories'
          : ''
      }`}
      onKeyDown={onKeyDown}
    >
      <Sidebar
        activeSection={section === 'group' ? 'all' : section}
        settingsActive={view === 'settings'}
        onSelect={(nextSection) => {
          setSection(nextSection)
          setActiveGroup(null)
          setView('home')
        }}
        onOpenSettings={() => setView('settings')}
      />
      <CategoryPanel
        groups={playlist.groups}
        activeGroup={activeGroup}
        visible={view === 'home' && (section === 'all' || section === 'group')}
        onSelectGroup={(name) => {
          if (!name) {
            setSection('all')
            setActiveGroup(null)
          } else {
            setSection('group')
            setActiveGroup(name)
          }
        }}
      />
      <main className="content">
        {playlist.error ? <div className="error-banner">{playlist.error}</div> : null}
        {view === 'settings' ? (
          <ScrollArea className="content-scroll" hideScrollbar>
            <ImportPlaylistPanel onImported={handleImported} />
          </ScrollArea>
        ) : (
          <ChannelGrid
            section={section}
            groupName={activeGroup}
            channels={visibleChannels}
            featuredChannel={playlist.recent[0] ?? playlist.channels[0] ?? null}
            favoriteIds={playlist.favoriteIds}
            loading={playlist.loading}
            probing={probe.probing}
            probeStatusById={probe.probeStatusById}
            probeSummary={probe.probeSummary}
            hideUnavailable={hideUnavailable}
            clock={clock}
            onPlay={handlePlay}
            onToggleFavorite={handleToggleFavorite}
            onProbeVisible={() => void runProbe(visibleChannels.map((channel) => channel.id))}
            onProbeAll={() => void runProbe()}
            onToggleHideUnavailable={() => setHideUnavailable((value) => !value)}
            onOpenSettings={() => setView('settings')}
          />
        )}
      </main>
      {toast ? <div className="toast">{toast}</div> : null}
      {playSession ? (
        <Suspense fallback={null}>
          <DesktopPlayer
            channel={playSession.lines[playSession.index]!}
            lines={playSession.lines}
            lineIndex={playSession.index}
            onSwitchLine={handleSwitchLine}
            onClose={() => setPlaySession(null)}
          />
        </Suspense>
      ) : null}
    </div>
  )
}
