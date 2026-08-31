import { useCallback, useEffect, useState } from 'react'

import { isNoPlaylistError, lumaApi, toUserMessage } from '@/shared/tauri/api'
import type { Channel, ChannelGroup } from '@/shared/tauri/types'

/**
 * Owns all playlist-derived data (channels, groups, favorites, recent) and
 * keeps the favorite toggle optimistic: the UI flips immediately and rolls
 * back only when the backend call fails.
 */
export function usePlaylistData() {
  const [channels, setChannels] = useState<Channel[]>([])
  const [groups, setGroups] = useState<ChannelGroup[]>([])
  const [favorites, setFavorites] = useState<Channel[]>([])
  const [recent, setRecent] = useState<Channel[]>([])
  const [favoriteIds, setFavoriteIds] = useState<Set<string>>(new Set())
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const applySnapshot = useCallback(
    (next: {
      channels: Channel[]
      groups: ChannelGroup[]
      favorites: Channel[]
      recent: Channel[]
    }) => {
      setChannels(next.channels)
      setGroups(next.groups)
      setFavorites(next.favorites)
      setRecent(next.recent)
      setFavoriteIds(new Set(next.favorites.map((channel) => channel.id)))
    },
    []
  )

  const load = useCallback(
    async (options?: { silent?: boolean }) => {
      const silent = options?.silent ?? false
      if (!silent) {
        setLoading(true)
      }
      setError(null)
      try {
        const [allChannels, allGroups, favoriteChannels, recentChannels] = await Promise.all([
          lumaApi.listChannels(),
          lumaApi.listGroups(),
          lumaApi.listFavorites(),
          lumaApi.listRecent()
        ])
        applySnapshot({
          channels: allChannels,
          groups: allGroups,
          favorites: favoriteChannels,
          recent: recentChannels
        })
      } catch (err) {
        // First run without an imported playlist is an empty state, not an error.
        if (!isNoPlaylistError(err)) {
          setError(toUserMessage(err))
        }
        if (!silent) {
          applySnapshot({ channels: [], groups: [], favorites: [], recent: [] })
        }
      } finally {
        if (!silent) {
          setLoading(false)
        }
      }
    },
    [applySnapshot]
  )

  useEffect(() => {
    void load()
  }, [load])

  /** Refresh only the recently-watched list after a playback starts. */
  const refreshRecent = useCallback(async () => {
    try {
      const recentChannels = await lumaApi.listRecent()
      setRecent(recentChannels)
    } catch {
      // Recent-list refresh is best-effort; failures surface on the next full load.
    }
  }, [])

  /**
   * Optimistically toggle a favorite. Returns an error message to show as a
   * toast when the backend rejects the change, or null on success.
   */
  const toggleFavorite = useCallback(
    async (channelId: string): Promise<string | null> => {
      const wasFavorite = favoriteIds.has(channelId)
      const previousIds = favoriteIds
      const previousFavorites = favorites

      setFavoriteIds((current) => {
        const next = new Set(current)
        if (wasFavorite) {
          next.delete(channelId)
        } else {
          next.add(channelId)
        }
        return next
      })

      setFavorites((current) => {
        if (wasFavorite) {
          return current.filter((channel) => channel.id !== channelId)
        }
        const channel =
          channels.find((item) => item.id === channelId) ??
          current.find((item) => item.id === channelId)
        return channel ? [...current, channel] : current
      })

      try {
        await lumaApi.toggleFavorite(channelId)
        return null
      } catch (err) {
        setFavoriteIds(previousIds)
        setFavorites(previousFavorites)
        return toUserMessage(err)
      }
    },
    [channels, favoriteIds, favorites]
  )

  return {
    channels,
    groups,
    favorites,
    recent,
    favoriteIds,
    loading,
    error,
    load,
    refreshRecent,
    toggleFavorite
  }
}
