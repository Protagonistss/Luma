import { describe, expect, it } from 'vitest'

import {
  buildChannelVirtualRows,
  buildLineIndex,
  filterChannelsBySection,
  getChannelGridColumnCount,
  groupChannelsByShelf,
  mergeChannelLines,
  orderLinesByProbe
} from './channelSelectors'

const channels = [
  {
    id: '1',
    name: 'News',
    streamUrl: 'https://example.com/1.m3u8',
    group: 'News'
  },
  {
    id: '2',
    name: 'Sports',
    streamUrl: 'https://example.com/2.m3u8',
    group: 'Sports'
  }
]

describe('filterChannelsBySection', () => {
  it('returns all channels for all section', () => {
    expect(filterChannelsBySection('all', null, channels, [], [])).toHaveLength(2)
  })

  it('filters by group', () => {
    expect(filterChannelsBySection('group', 'News', channels, [], [])).toEqual([channels[0]])
  })

  it('groups channels into shelves by category', () => {
    expect(groupChannelsByShelf(channels)).toEqual([
      { title: 'News', channels: [channels[0]] },
      { title: 'Sports', channels: [channels[1]] }
    ])
  })

  it('calculates grid columns from container width', () => {
    expect(getChannelGridColumnCount(1200)).toBe(6)
    expect(getChannelGridColumnCount(400)).toBe(1)
  })

  it('builds virtual rows with shelf headers', () => {
    expect(buildChannelVirtualRows(channels, 1, true)).toEqual([
      { kind: 'shelf-header', title: 'News', count: 1, key: 'header:News' },
      { kind: 'channel-row', channels: [channels[0]], key: 'row:News:0' },
      { kind: 'shelf-header', title: 'Sports', count: 1, key: 'header:Sports' },
      { kind: 'channel-row', channels: [channels[1]], key: 'row:Sports:0' }
    ])
  })

  it('builds flat virtual rows without shelf headers', () => {
    expect(buildChannelVirtualRows(channels, 2, false)).toEqual([
      { kind: 'channel-row', channels, key: 'row:0' }
    ])
  })
})

const lineA = { id: 'a', name: 'CCTV-1', streamUrl: 'https://a.m3u8', group: '央视' }
const lineB = { id: 'b', name: 'CCTV-1', streamUrl: 'https://b.m3u8', group: '央视' }
const lineC = { id: 'c', name: 'CCTV-1', streamUrl: 'https://c.m3u8', group: '央视' }
const other = { id: 'd', name: 'CCTV-2', streamUrl: 'https://d.m3u8', group: '央视' }

describe('mergeChannelLines', () => {
  it('merges duplicate lines of the same channel into one entry', () => {
    const merged = mergeChannelLines([lineA, lineB, other])
    expect(merged).toHaveLength(2)
    expect(merged[0]!.lines).toEqual([lineA, lineB])
    expect(merged[0]!.id).toBe('a')
  })

  it('keeps same name in different groups separate', () => {
    const otherGroup = { ...lineA, id: 'x', group: '卫视' }
    const merged = mergeChannelLines([lineA, otherGroup])
    expect(merged).toHaveLength(2)
  })

  it('orders lines probe-aware: playable first, dead last', () => {
    const merged = mergeChannelLines([lineA, lineB, lineC], { a: 'unreachable', b: 'playable' })
    expect(merged[0]!.lines.map((line) => line.id)).toEqual(['b', 'c', 'a'])
    // Primary line is the best-known one.
    expect(merged[0]!.id).toBe('b')
  })
})

describe('orderLinesByProbe', () => {
  it('keeps list order among equal ranks', () => {
    const ordered = orderLinesByProbe([lineA, lineB], { a: 'playable', b: 'playable' })
    expect(ordered.map((line) => line.id)).toEqual(['a', 'b'])
  })
})

describe('buildLineIndex', () => {
  it('maps every line id to the same ordered list', () => {
    const merged = mergeChannelLines([lineA, lineB, other])
    const index = buildLineIndex(merged)
    expect(index.get('a')).toEqual([lineA, lineB])
    expect(index.get('b')).toEqual([lineA, lineB])
    expect(index.get('d')).toEqual([other])
  })
})
