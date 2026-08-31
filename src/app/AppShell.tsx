import { useCallback, useEffect, useMemo, useState } from "react";

import { ChannelGrid } from "@/features/channels/ChannelGrid";
import { Sidebar } from "@/features/channels/Sidebar";
import {
  filterChannelsBySection,
  type SidebarSection,
} from "@/features/channels/channelSelectors";
import { ImportPlaylistPanel } from "@/features/import-playlist/ImportPlaylistPanel";
import { useTvNavigation } from "@/shared/focus/useTvNavigation";
import { lumaApi, toUserMessage } from "@/shared/tauri/api";
import { openNativePlayer } from "@/shared/tauri/player";
import type { Channel, ChannelGroup } from "@/shared/tauri/types";

type View = "home" | "settings";

export function AppShell() {
  const [view, setView] = useState<View>("home");
  const [section, setSection] = useState<SidebarSection>("all");
  const [activeGroup, setActiveGroup] = useState<string | null>(null);
  const [channels, setChannels] = useState<Channel[]>([]);
  const [groups, setGroups] = useState<ChannelGroup[]>([]);
  const [favorites, setFavorites] = useState<Channel[]>([]);
  const [recent, setRecent] = useState<Channel[]>([]);
  const [favoriteIds, setFavoriteIds] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [lastFocusedChannelId, setLastFocusedChannelId] = useState<string | null>(
    null,
  );
  const { onKeyDown } = useTvNavigation();

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [allChannels, allGroups, favoriteChannels, recentChannels] =
        await Promise.all([
          lumaApi.listChannels(),
          lumaApi.listGroups(),
          lumaApi.listFavorites(),
          lumaApi.listRecent(),
        ]);
      setChannels(allChannels);
      setGroups(allGroups);
      setFavorites(favoriteChannels);
      setRecent(recentChannels);
      setFavoriteIds(new Set(favoriteChannels.map((channel) => channel.id)));
    } catch (err) {
      const message = toUserMessage(err);
      if (!message.includes("no playlist imported")) {
        setError(message);
      }
      setChannels([]);
      setGroups([]);
      setFavorites([]);
      setRecent([]);
      setFavoriteIds(new Set());
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  useEffect(() => {
    if (!toast) {
      return;
    }
    const timer = window.setTimeout(() => setToast(null), 3000);
    return () => window.clearTimeout(timer);
  }, [toast]);

  useEffect(() => {
    if (!lastFocusedChannelId) {
      return;
    }
    const element = document.querySelector<HTMLElement>(
      `[data-channel-id="${lastFocusedChannelId}"]`,
    );
    element?.focus();
  }, [channels, lastFocusedChannelId, view, section, activeGroup]);

  const visibleChannels = useMemo(
    () =>
      filterChannelsBySection(section, activeGroup, channels, favorites, recent),
    [section, activeGroup, channels, favorites, recent],
  );

  const handlePlay = async (channelId: string) => {
    setLastFocusedChannelId(channelId);
    try {
      const payload = await lumaApi.playChannel(channelId);
      await openNativePlayer(payload);
      await loadData();
    } catch (err) {
      setToast(toUserMessage(err));
    }
  };

  const handleToggleFavorite = async (channelId: string) => {
    const previous = new Set(favoriteIds);
    const optimistic = new Set(favoriteIds);
    if (optimistic.has(channelId)) {
      optimistic.delete(channelId);
    } else {
      optimistic.add(channelId);
    }
    setFavoriteIds(optimistic);

    try {
      await lumaApi.toggleFavorite(channelId);
      await loadData();
    } catch (err) {
      setFavoriteIds(previous);
      setToast(toUserMessage(err));
    }
  };

  return (
    <div className="app-shell" onKeyDown={onKeyDown}>
      <Sidebar
        activeSection={section}
        activeGroup={activeGroup}
        groups={groups}
        onSelect={(nextSection, groupName) => {
          setSection(nextSection);
          setActiveGroup(groupName ?? null);
          setView("home");
        }}
        onOpenSettings={() => setView("settings")}
      />
      <main className="content">
        {error ? <div className="error-banner">{error}</div> : null}
        {view === "settings" ? (
          <ImportPlaylistPanel
            onImported={() => {
              setView("home");
              void loadData();
            }}
          />
        ) : (
          <ChannelGrid
            section={section}
            groupName={activeGroup}
            channels={visibleChannels}
            favoriteIds={favoriteIds}
            loading={loading}
            onPlay={handlePlay}
            onToggleFavorite={handleToggleFavorite}
          />
        )}
      </main>
      {toast ? <div className="toast">{toast}</div> : null}
    </div>
  );
}
