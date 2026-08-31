import { ChannelCard } from "./ChannelCard";
import type { Channel } from "@/shared/tauri/types";
import type { SidebarSection } from "./channelSelectors";

interface ChannelGridProps {
  section: SidebarSection;
  groupName: string | null;
  channels: Channel[];
  favoriteIds: Set<string>;
  loading: boolean;
  onPlay: (channelId: string) => void;
  onToggleFavorite: (channelId: string) => void;
}

function sectionTitle(section: SidebarSection, groupName: string | null) {
  switch (section) {
    case "all":
      return "全部频道";
    case "favorites":
      return "收藏";
    case "recent":
      return "最近观看";
    case "group":
      return groupName ?? "分类";
    default:
      return "频道";
  }
}

export function ChannelGrid({
  section,
  groupName,
  channels,
  favoriteIds,
  loading,
  onPlay,
  onToggleFavorite,
}: ChannelGridProps) {
  if (loading) {
    return <div className="loading-state">正在加载频道...</div>;
  }

  if (channels.length === 0) {
    return (
      <div className="empty-state">
        <h2>{sectionTitle(section, groupName)}</h2>
        <p>当前没有可显示的频道，请先导入 M3U 播放列表。</p>
      </div>
    );
  }

  return (
    <section>
      <h2>{sectionTitle(section, groupName)}</h2>
      <div className="channel-grid">
        {channels.map((channel) => (
          <ChannelCard
            key={channel.id}
            channel={channel}
            isFavorite={favoriteIds.has(channel.id)}
            onPlay={onPlay}
            onToggleFavorite={onToggleFavorite}
          />
        ))}
      </div>
    </section>
  );
}
