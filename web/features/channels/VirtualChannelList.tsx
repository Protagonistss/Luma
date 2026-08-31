import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useMemo, useRef, useState } from "react";

import { ChannelCard } from "./ChannelCard";
import {
  buildChannelVirtualRows,
  CHANNEL_VIRTUALIZE_THRESHOLD,
  estimateChannelVirtualRowSize,
  getChannelGridColumnCount,
} from "./channelSelectors";
import type { Channel } from "@/shared/tauri/types";
import type { ProbeStatus } from "@/shared/tauri/types";
import { useScrollElement } from "@/shared/ui/ScrollArea";

interface VirtualChannelListProps {
  channels: Channel[];
  groupByShelf: boolean;
  favoriteIds: Set<string>;
  probeStatusById: Record<string, ProbeStatus>;
  probing: boolean;
  onPlay: (channelId: string) => void;
  onToggleFavorite: (channelId: string) => void;
}

export function VirtualChannelList({
  channels,
  groupByShelf,
  favoriteIds,
  probeStatusById,
  probing,
  onPlay,
  onToggleFavorite,
}: VirtualChannelListProps) {
  const scrollElementRef = useScrollElement();
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(0);

  useEffect(() => {
    const element = containerRef.current;
    if (!element) {
      return;
    }

    const updateWidth = () => {
      setContainerWidth(element.clientWidth);
    };

    updateWidth();
    const observer = new ResizeObserver(updateWidth);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const columnCount = useMemo(
    () => getChannelGridColumnCount(containerWidth || window.innerWidth),
    [containerWidth],
  );

  const rows = useMemo(
    () => buildChannelVirtualRows(channels, columnCount, groupByShelf),
    [channels, columnCount, groupByShelf],
  );

  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollElementRef?.current ?? null,
    estimateSize: (index) =>
      estimateChannelVirtualRowSize(rows[index]!, columnCount, containerWidth || window.innerWidth),
    overscan: 4,
  });

  useEffect(() => {
    rowVirtualizer.measure();
  }, [columnCount, rows.length, rowVirtualizer]);

  if (channels.length === 0) {
    return null;
  }

  return (
    <div ref={containerRef} className="virtual-channel-list">
      <div
        className="virtual-channel-list__spacer"
        style={{ height: `${rowVirtualizer.getTotalSize()}px` }}
      >
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const row = rows[virtualRow.index];
          if (!row) {
            return null;
          }

          return (
            <div
              key={row.key}
              ref={rowVirtualizer.measureElement}
              data-index={virtualRow.index}
              className="virtual-channel-row"
              style={{ transform: `translateY(${virtualRow.start}px)` }}
            >
              {row.kind === "shelf-header" ? (
                <header className="shelf-header">
                  <h3>{row.title}</h3>
                  <span>{row.count}</span>
                </header>
              ) : (
                <div
                  className="channel-grid channel-grid--virtual"
                  style={{
                    gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
                  }}
                >
                  {row.channels.map((channel) => (
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
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function shouldVirtualizeChannels(channelCount: number) {
  return channelCount >= CHANNEL_VIRTUALIZE_THRESHOLD;
}
