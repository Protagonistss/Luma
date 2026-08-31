import { lazy, Suspense, useEffect, useMemo, useState } from 'react'

import { CategoryPanel } from '@/features/channels/CategoryPanel'
import { ChannelGrid } from '@/features/channels/ChannelGrid'
import { filterChannelsBySection, type SidebarSection } from '@/features/channels/channelSelectors'
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

export function AppShell() {
  const [view, setView] = useState<View>('home')
  const [section, setSection] = useState<SidebarSection>('all')
  const [activeGroup, setActiveGroup] = useState<string | null>(null)
  const [toast, setToast] = useState<string | null>(null)
  const [lastFocusedChannelId, setLastFocusedChannelId] = useState<string | null>(null)
  const [playingChannel, setPlayingChannel] = useState<PlayChannelResponse | null>(null)
  const [showPlayableOnly, setShowPlayableOnly] = useState(false)

  const playlist = usePlaylistData()
  const probe = useChannelProbe()
  const onKeyDown = useTvNavigation()
  const clock = useClock()

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

  const visibleChannels = useMemo(
    () =>
      filterChannelsBySection(
        section,
        activeGroup,
        playlist.channels,
        playlist.favorites,
        playlist.recent
      ),
    [section, activeGroup, playlist.channels, playlist.favorites, playlist.recent]
  )

  const handlePlay = async (channelId: string) => {
    setLastFocusedChannelId(channelId)
    try {
      const payload = await lumaApi.playChannel(channelId)
      if (shouldUseDesktopPlayer()) {
        setPlayingChannel(payload)
      } else {
        await openNativePlayer(payload)
      }
      await playlist.refreshRecent()
    } catch (err) {
      setToast(toUserMessage(err))
    }
  }

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
            <ImportPlaylistPanel
              onImported={() => {
                setView('home')
                void playlist.load()
              }}
            />
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
            showPlayableOnly={showPlayableOnly}
            clock={clock}
            onPlay={handlePlay}
            onToggleFavorite={handleToggleFavorite}
            onProbeVisible={() => void runProbe(visibleChannels.map((channel) => channel.id))}
            onProbeAll={() => void runProbe()}
            onTogglePlayableOnly={() => setShowPlayableOnly((value) => !value)}
            onOpenSettings={() => setView('settings')}
          />
        )}
      </main>
      {toast ? <div className="toast">{toast}</div> : null}
      {playingChannel ? (
        <Suspense fallback={null}>
          <DesktopPlayer channel={playingChannel} onClose={() => setPlayingChannel(null)} />
        </Suspense>
      ) : null}
    </div>
  )
}
