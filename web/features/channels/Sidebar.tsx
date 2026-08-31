import { ClockIcon, HomeIcon, LumaLogoIcon, SettingsIcon, StarIcon } from '@/shared/icons'
import { isDesktopTauri } from '@/shared/platform'

import { buildSidebarItems } from './channelSelectors'
import type { SidebarSection } from './channelSelectors'

interface SidebarProps {
  activeSection: SidebarSection
  onSelect: (section: SidebarSection) => void
  onOpenSettings: () => void
  settingsActive: boolean
}

const ICONS = {
  all: HomeIcon,
  favorites: StarIcon,
  recent: ClockIcon
} as const

export function Sidebar({ activeSection, onSelect, onOpenSettings, settingsActive }: SidebarProps) {
  const items = buildSidebarItems()
  const showBrand = !isDesktopTauri()
  const isDesktop = isDesktopTauri()

  return (
    <aside
      className={[
        'sidebar',
        showBrand ? '' : 'sidebar--no-brand',
        isDesktop ? 'sidebar--desktop' : ''
      ]
        .filter(Boolean)
        .join(' ')}
    >
      {showBrand ? (
        <div className="sidebar-brand">
          <span className="sidebar-mark">
            <LumaLogoIcon size={22} />
          </span>
          <strong>Luma</strong>
        </div>
      ) : null}
      <nav className="sidebar-nav" aria-label="主导航">
        {items.map((item) => {
          const Icon = ICONS[item.id]
          const isActive = !settingsActive && item.id === activeSection

          return (
            <button
              key={item.id}
              type="button"
              className={`sidebar-item ${isActive ? 'active' : ''}`}
              aria-label={item.label}
              title={isDesktop ? item.label : undefined}
              onClick={() => onSelect(item.id)}
            >
              <span className="sidebar-item__icon" aria-hidden>
                <Icon size={20} />
              </span>
              <span className="sidebar-item__label">{item.label}</span>
            </button>
          )
        })}
      </nav>
      <button
        type="button"
        className={`sidebar-item sidebar-settings ${settingsActive ? 'active' : ''}`}
        aria-label="设置"
        title={isDesktop ? '设置' : undefined}
        onClick={onOpenSettings}
      >
        <span className="sidebar-item__icon" aria-hidden>
          <SettingsIcon size={20} />
        </span>
        <span className="sidebar-item__label">设置</span>
      </button>
    </aside>
  )
}
