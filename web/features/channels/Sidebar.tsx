import { buildSidebarItems } from "./channelSelectors";
import type { SidebarSection } from "./channelSelectors";

interface SidebarProps {
  activeSection: SidebarSection;
  onSelect: (section: SidebarSection) => void;
  onOpenSettings: () => void;
  settingsActive: boolean;
}

function HomeIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path
        fill="currentColor"
        d="M4.5 11.2 12 4.8l7.5 6.4v8.3a1.5 1.5 0 0 1-1.5 1.5h-4.2v-5.1h-3.6v5.1H6a1.5 1.5 0 0 1-1.5-1.5z"
      />
    </svg>
  );
}

function StarIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path
        fill="currentColor"
        d="m12 3.6 2.4 4.9 5.4.8-3.9 3.8.9 5.4L12 16l-4.8 2.5.9-5.4-3.9-3.8 5.4-.8z"
      />
    </svg>
  );
}

function ClockIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path
        fill="currentColor"
        d="M12 3.5a8.5 8.5 0 1 1 0 17 8.5 8.5 0 0 1 0-17m0 2a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13m.7 2.2v4.1l2.8 1.7-.7 1.2L11 12.3V7.7z"
      />
    </svg>
  );
}

function GearIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path
        fill="currentColor"
        d="M10.1 3.2h3.8l.4 2.3a7 7 0 0 1 1.8.8l2.2-1 1.9 3.3-1.8 1.5a7 7 0 0 1 0 1.8l1.8 1.5-1.9 3.3-2.2-1a7 7 0 0 1-1.8.8l-.4 2.3h-3.8l-.4-2.3a7 7 0 0 1-1.8-.8l-2.2 1-1.9-3.3 1.8-1.5a7 7 0 0 1 0-1.8L3.8 8.6l1.9-3.3 2.2 1a7 7 0 0 1 1.8-.8zm1.9 5.3A3.5 3.5 0 1 0 12 15.5 3.5 3.5 0 0 0 12 8.5"
      />
    </svg>
  );
}

const ICONS = {
  all: HomeIcon,
  favorites: StarIcon,
  recent: ClockIcon,
} as const;

export function Sidebar({
  activeSection,
  onSelect,
  onOpenSettings,
  settingsActive,
}: SidebarProps) {
  const items = buildSidebarItems();

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <span className="sidebar-mark">L</span>
        <strong>Luma</strong>
      </div>
      <nav className="sidebar-nav" aria-label="主导航">
        {items.map((item) => {
          const Icon = ICONS[item.id];
          const isActive = !settingsActive && item.id === activeSection;

          return (
            <button
              key={item.id}
              type="button"
              className={`sidebar-item ${isActive ? "active" : ""}`}
              onClick={() => onSelect(item.id)}
            >
              <Icon />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>
      <button
        type="button"
        className={`sidebar-item sidebar-settings ${settingsActive ? "active" : ""}`}
        onClick={onOpenSettings}
      >
        <GearIcon />
        <span>设置</span>
      </button>
    </aside>
  );
}
