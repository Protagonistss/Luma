import { useEffect, useMemo, useState } from "react";
import { ChannelBrowseToolbar } from "./ChannelBrowseToolbar";
import { ChannelCard } from "./ChannelCard";
import { FeaturedHero } from "./FeaturedHero";
import { groupChannelsByShelf } from "./channelSelectors";
import type { SidebarSection } from "./channelSelectors";
import {
  shouldVirtualizeChannels,
  VirtualChannelList,
} from "./VirtualChannelList";
import { ScrollArea } from "@/shared/ui/ScrollArea";
import type { Channel, ProbeStatus } from "@/shared/tauri/types";

interface ChannelGridProps {
  section: SidebarSection;
  groupName: string | null;
  channels: Channel[];
  featuredChannel: Channel | null;
  favoriteIds: Set<string>;
  loading: boolean;
  probing: boolean;
  probeStatusById: Record<string, ProbeStatus>;
  probeSummary: { playable: number; unreachable: number; invalid: number } | null;
  showPlayableOnly: boolean;
  clock?: string;
  onPlay: (channelId: string) => void;
  onToggleFavorite: (channelId: string) => void;
  onProbeVisible: () => void;
  onProbeAll: () => void;
  onTogglePlayableOnly: () => void;
  onOpenSettings: () => void;
}

function sectionTitle(section: SidebarSection, groupName: string | null) {
  switch (section) {
    case "all":
      return "全部频道";
    case "favorites":
      return "我的收藏";
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
  featuredChannel,
  favoriteIds,
  loading,
  probing,
  probeStatusById,
  probeSummary,
  showPlayableOnly,
  clock,
  onPlay,
  onToggleFavorite,
  onProbeVisible,
  onProbeAll,
  onTogglePlayableOnly,
  onOpenSettings,
}: ChannelGridProps) {
  const [searchQuery, setSearchQuery] = useState("");

  useEffect(() => {
    setSearchQuery("");
  }, [section, groupName]);

  const displayChannels = useMemo(
    () =>
      showPlayableOnly
        ? channels.filter((channel) => probeStatusById[channel.id] === "playable")
        : channels,
    [channels, probeStatusById, showPlayableOnly],
  );

  const filteredChannels = useMemo(() => {
    const normalized = searchQuery.trim().toLowerCase();
    if (!normalized) {
      return displayChannels;
    }
    return displayChannels.filter(
      (channel) =>
        channel.name.toLowerCase().includes(normalized) ||
        channel.group.toLowerCase().includes(normalized),
    );
  }, [displayChannels, searchQuery]);

  const shelves = useMemo(
    () =>
      section === "all" && !groupName
        ? groupChannelsByShelf(filteredChannels)
        : [{ title: sectionTitle(section, groupName), channels: filteredChannels }],
    [filteredChannels, groupName, section],
  );

  const showShelfHeaders = section === "all" && !groupName && shelves.length > 1;
  const useVirtualList = shouldVirtualizeChannels(filteredChannels.length);
  const showHero =
    featuredChannel &&
    section === "all" &&
    !groupName &&
    !searchQuery.trim() &&
    !showPlayableOnly;

  if (loading) {
    return (
      <div className="loading-state">
        <span className="loading-spinner" aria-hidden />
        正在加载频道...
      </div>
    );
  }

  if (channels.length === 0) {
    return (
      <div className="empty-state">
        <p className="kicker">Luma</p>
        <h2>还没有频道</h2>
        <p>导入一份 M3U 播放列表，即可在电视上浏览和播放直播频道。</p>
        <button type="button" className="primary-button" onClick={onOpenSettings}>
          导入播放列表
        </button>
      </div>
    );
  }

  return (
    <section className="home-stage">
      <ChannelBrowseToolbar
        title={sectionTitle(section, groupName)}
        count={filteredChannels.length}
        clock={clock}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        showPlayableOnly={showPlayableOnly}
        onTogglePlayableOnly={onTogglePlayableOnly}
        probing={probing}
        onProbeVisible={onProbeVisible}
        onProbeAll={onProbeAll}
        probeSummary={probeSummary}
      />

      <ScrollArea className="channel-body-scroll" hideScrollbar>
        <div className="channel-body-inner">
          {showHero ? (
            <FeaturedHero
              channel={featuredChannel}
              onPlay={() => onPlay(featuredChannel.id)}
            />
          ) : null}

          {filteredChannels.length === 0 ? (
            <div className="empty-state compact">
              <p>
                {searchQuery.trim()
                  ? `没有匹配「${searchQuery.trim()}」的频道。`
                  : "当前筛选下没有可用频道，取消「仅可用」或重新检测。"}
              </p>
            </div>
          ) : useVirtualList ? (
            <VirtualChannelList
              channels={filteredChannels}
              groupByShelf={showShelfHeaders}
              favoriteIds={favoriteIds}
              probeStatusById={probeStatusById}
              probing={probing}
              onPlay={onPlay}
              onToggleFavorite={onToggleFavorite}
            />
          ) : (
            shelves.map((shelf) => (
              <section key={shelf.title} className="channel-shelf">
                {showShelfHeaders ? (
                  <header className="shelf-header">
                    <h3>{shelf.title}</h3>
                    <span>{shelf.channels.length}</span>
                  </header>
                ) : null}
                <div className="channel-grid">
                  {shelf.channels.map((channel) => (
                    <ChannelCard
                      key={channel.id}
                      channel={channel}
                      isFavorite={favoriteIds.has(channel.id)}
                      probeStatus={probeStatusById[channel.id]}
                      probing={probing && !probeStatusById[channel.id]}
                      onPlay={onPlay}
                      onToggleFavorite={onToggleFavorite}
                    />
                  ))}
                </div>
              </section>
            ))
          )}
        </div>
      </ScrollArea>
    </section>
  );
}
