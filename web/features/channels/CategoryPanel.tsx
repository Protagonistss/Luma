import { useMemo, useState } from "react";

import { ScrollArea } from "@/shared/ui/ScrollArea";
import type { ChannelGroup } from "@/shared/tauri/types";

interface CategoryPanelProps {
  groups: ChannelGroup[];
  activeGroup: string | null;
  onSelectGroup: (groupName: string | null) => void;
  visible: boolean;
}

export function CategoryPanel({
  groups,
  activeGroup,
  onSelectGroup,
  visible,
}: CategoryPanelProps) {
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) {
      return groups;
    }
    return groups.filter((group) =>
      group.name.toLowerCase().includes(normalized),
    );
  }, [groups, query]);

  if (!visible || groups.length === 0) {
    return null;
  }

  return (
    <aside className="category-panel" aria-label="频道分类">
      <div className="category-panel-header">
        <h3>分类</h3>
        <span>{groups.length}</span>
      </div>
      {groups.length > 12 ? (
        <input
          className="category-search"
          type="search"
          placeholder="搜索分类..."
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      ) : null}
      <ScrollArea className="category-scroll">
        <div className="category-list">
          <button
            type="button"
            className={`category-item ${activeGroup === null ? "active" : ""}`}
            onClick={() => onSelectGroup(null)}
          >
            <span>全部频道</span>
            <span className="category-item-count">
              {groups.reduce((sum, group) => sum + group.channelCount, 0)}
            </span>
          </button>
          {filtered.map((group) => (
            <button
              key={group.name}
              type="button"
              className={`category-item ${activeGroup === group.name ? "active" : ""}`}
              onClick={() => onSelectGroup(group.name)}
            >
              <span>{group.name}</span>
              <span className="category-item-count">{group.channelCount}</span>
            </button>
          ))}
          {filtered.length === 0 ? (
            <p className="category-empty">没有匹配的分类</p>
          ) : null}
        </div>
      </ScrollArea>
    </aside>
  );
}
