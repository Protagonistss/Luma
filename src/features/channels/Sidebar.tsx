import { useMemo } from "react";

import { buildSidebarItems } from "./channelSelectors";
import type { SidebarSection } from "./channelSelectors";
import type { ChannelGroup } from "@/shared/tauri/types";

interface SidebarProps {
  activeSection: SidebarSection;
  activeGroup: string | null;
  groups: ChannelGroup[];
  onSelect: (section: SidebarSection, groupName?: string | null) => void;
  onOpenSettings: () => void;
}

export function Sidebar({
  activeSection,
  activeGroup,
  groups,
  onSelect,
  onOpenSettings,
}: SidebarProps) {
  const items = useMemo(() => buildSidebarItems(groups), [groups]);

  return (
    <aside className="sidebar">
      <h1>Luma</h1>
      {items.map((item) => {
        const isActive =
          item.id === activeSection &&
          (item.id !== "group" || item.groupName === activeGroup);

        return (
          <button
            key={`${item.id}-${"groupName" in item ? item.groupName : item.label}`}
            type="button"
            className={`sidebar-item ${isActive ? "active" : ""}`}
            onClick={() =>
              onSelect(item.id, "groupName" in item ? item.groupName : null)
            }
          >
            {item.label}
            {"count" in item ? ` (${item.count})` : ""}
          </button>
        );
      })}
      <button type="button" className="sidebar-item" onClick={onOpenSettings}>
        设置 / 导入
      </button>
    </aside>
  );
}
