import type { ProbeProgress } from '@/app/useChannelProbe'
import { ProbeIcon, SearchIcon } from '@/shared/icons'

interface ChannelBrowseToolbarProps {
  title: string
  count: number
  clock?: string
  searchQuery: string
  onSearchChange: (query: string) => void
  hideUnavailable: boolean
  onToggleHideUnavailable: () => void
  probing: boolean
  probeProgress?: ProbeProgress | null
  onProbeVisible: () => void
  onProbeAll: () => void
  probeSummary: { playable: number; unreachable: number; invalid: number } | null
}

export function ChannelBrowseToolbar({
  title,
  count,
  clock,
  searchQuery,
  onSearchChange,
  hideUnavailable,
  onToggleHideUnavailable,
  probing,
  probeProgress,
  onProbeVisible,
  onProbeAll,
  probeSummary
}: ChannelBrowseToolbarProps) {
  return (
    <div className="channel-browse-toolbar">
      <div className="channel-browse-toolbar__row">
        <h2 className="channel-browse-toolbar__heading">
          {title}
          <span className="channel-browse-toolbar__count">{count}</span>
        </h2>

        <div className="channel-search-wrap">
          <SearchIcon className="channel-search__icon" size={16} />
          <input
            className="channel-search"
            type="search"
            placeholder="搜索频道..."
            value={searchQuery}
            onChange={(event) => onSearchChange(event.target.value)}
          />
        </div>

        <div className="channel-browse-toolbar__actions">
          <button
            type="button"
            className={`filter-chip ${hideUnavailable ? 'active' : ''}`}
            onClick={onToggleHideUnavailable}
          >
            隐藏不可用
          </button>
          <button
            type="button"
            className="ghost-button ghost-button--compact"
            disabled={probing}
            onClick={onProbeVisible}
          >
            <ProbeIcon size={14} />
            {probing
              ? probeProgress
                ? `检测中 ${probeProgress.done}/${probeProgress.total}`
                : '检测中'
              : '检测'}
          </button>
          <button
            type="button"
            className="ghost-button ghost-button--compact"
            disabled={probing}
            onClick={onProbeAll}
          >
            全部
          </button>
          {clock ? <time className="toolbar-clock">{clock}</time> : null}
        </div>
      </div>

      {probeSummary ? (
        <div className="probe-summary probe-summary--inline">
          <span className="probe-stat playable">可用 {probeSummary.playable}</span>
          <span className="probe-stat unreachable">离线 {probeSummary.unreachable}</span>
          <span className="probe-stat invalid">无效 {probeSummary.invalid}</span>
        </div>
      ) : null}
    </div>
  )
}
