import type { Channel, ProbeStatus } from '@/shared/tauri/types'

export type SidebarSection = 'all' | 'favorites' | 'recent' | 'group'

/** A channel whose duplicate sources (same group + normalized name) have
 * been merged into one card. `lines[0]` is the primary line to play. */
export interface MergedChannel extends Channel {
  lines: Channel[]
}

export const CHANNEL_GRID_MIN_CARD_WIDTH = 168
export const CHANNEL_GRID_GAP = 16
export const CHANNEL_GRID_PADDING_X = 80
export const CHANNEL_VIRTUALIZE_THRESHOLD = 48

export type ChannelVirtualRow =
  | { kind: 'shelf-header'; title: string; count: number; key: string }
  | { kind: 'channel-row'; channels: Channel[]; key: string }

export function filterChannelsBySection(
  section: SidebarSection,
  groupName: string | null,
  allChannels: Channel[],
  favorites: Channel[],
  recent: Channel[]
): Channel[] {
  switch (section) {
    case 'favorites':
      return favorites
    case 'recent':
      return recent
    case 'group':
      return groupName ? allChannels.filter((channel) => channel.group === groupName) : allChannels
    case 'all':
    default:
      return allChannels
  }
}

/** Merge channels that point at the same station (same group + name after
 * import-time normalization) into a single card with all playable sources.
 * Lines are ordered probe-aware: confirmed playable first, unprobed next,
 * confirmed dead last, keeping list order otherwise. */
export function mergeChannelLines(
  channels: Channel[],
  probeStatusById: Record<string, ProbeStatus> = {}
): MergedChannel[] {
  const groups = new Map<string, Channel[]>()
  const order: string[] = []

  for (const channel of channels) {
    const key = `${channel.group}\u{0}${channel.name}`
    const existing = groups.get(key)
    if (existing) {
      existing.push(channel)
    } else {
      groups.set(key, [channel])
      order.push(key)
    }
  }

  return order.map((key) => {
    const lines = orderLinesByProbe(groups.get(key)!, probeStatusById)
    // Object.assign instead of object spread: the merged entry is the primary
    // line plus its line list, and lint flags spread-modify in maps.
    return Object.assign({}, lines[0], { lines })
  })
}

export function orderLinesByProbe(
  lines: Channel[],
  probeStatusById: Record<string, ProbeStatus>
): Channel[] {
  const rank = (channel: Channel) => {
    switch (probeStatusById[channel.id]) {
      case 'playable':
        return 0
      case 'unreachable':
        return 2
      case 'invalidBody':
        return 2
      default:
        return 1
    }
  }
  return lines
    .map((channel, index) => ({ channel, index, rank: rank(channel) }))
    .toSorted((left, right) => left.rank - right.rank || left.index - right.index)
    .map((entry) => entry.channel)
}

/** Map every channel id (including hidden duplicate lines) to its ordered
 * line list, so playback failover works no matter which card was clicked. */
export function buildLineIndex(merged: MergedChannel[]): Map<string, Channel[]> {
  const index = new Map<string, Channel[]>()
  for (const entry of merged) {
    for (const line of entry.lines) {
      index.set(line.id, entry.lines)
    }
  }
  return index
}

export function buildSidebarItems() {
  return [
    { id: 'all' as const, label: '首页' },
    { id: 'favorites' as const, label: '收藏' },
    { id: 'recent' as const, label: '最近' }
  ]
}

export function groupChannelsByShelf(channels: Channel[]) {
  const shelves = new Map<string, Channel[]>()

  for (const channel of channels) {
    const title = channel.group.trim() || '未分类'
    const existing = shelves.get(title)
    if (existing) {
      existing.push(channel)
    } else {
      shelves.set(title, [channel])
    }
  }

  return [...shelves.entries()].map(([title, items]) => ({
    title,
    channels: items
  }))
}

export function getChannelGridColumnCount(containerWidth: number) {
  const innerWidth = Math.max(containerWidth - CHANNEL_GRID_PADDING_X, CHANNEL_GRID_MIN_CARD_WIDTH)
  return Math.max(
    1,
    Math.floor((innerWidth + CHANNEL_GRID_GAP) / (CHANNEL_GRID_MIN_CARD_WIDTH + CHANNEL_GRID_GAP))
  )
}

export function buildChannelVirtualRows(
  channels: Channel[],
  columnCount: number,
  groupByShelf: boolean
): ChannelVirtualRow[] {
  if (columnCount < 1 || channels.length === 0) {
    return []
  }

  const rows: ChannelVirtualRow[] = []

  const appendChannelRows = (items: Channel[], keyPrefix: string) => {
    for (let index = 0; index < items.length; index += columnCount) {
      rows.push({
        kind: 'channel-row',
        channels: items.slice(index, index + columnCount),
        key: `${keyPrefix}:${index}`
      })
    }
  }

  if (groupByShelf) {
    for (const shelf of groupChannelsByShelf(channels)) {
      rows.push({
        kind: 'shelf-header',
        title: shelf.title,
        count: shelf.channels.length,
        key: `header:${shelf.title}`
      })
      appendChannelRows(shelf.channels, `row:${shelf.title}`)
    }
    return rows
  }

  appendChannelRows(channels, 'row')
  return rows
}

export function estimateChannelVirtualRowSize(
  row: ChannelVirtualRow,
  columnCount: number,
  containerWidth: number
) {
  if (row.kind === 'shelf-header') {
    return 46
  }

  const innerWidth = Math.max(containerWidth - CHANNEL_GRID_PADDING_X, CHANNEL_GRID_MIN_CARD_WIDTH)
  const cardWidth = (innerWidth - CHANNEL_GRID_GAP * Math.max(columnCount - 1, 0)) / columnCount
  const posterHeight = cardWidth * (10 / 16)
  return posterHeight + 10 + 42 + 20
}
