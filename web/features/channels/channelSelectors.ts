import type { Channel } from '@/shared/tauri/types'

export type SidebarSection = 'all' | 'favorites' | 'recent' | 'group'

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
