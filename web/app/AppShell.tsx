import { useCallback, useEffect, useMemo, useState } from "react";

import { ChannelGrid } from "@/features/channels/ChannelGrid";
import { CategoryPanel } from "@/features/channels/CategoryPanel";
import { Sidebar } from "@/features/channels/Sidebar";
import { ScrollArea } from "@/shared/ui/ScrollArea";
import {
  filterChannelsBySection,
  type SidebarSection,
} from "@/features/channels/channelSelectors";
import { ImportPlaylistPanel } from "@/features/import-playlist/ImportPlaylistPanel";
import { DesktopPlayer } from "@/features/player/DesktopPlayer";
import { useTvNavigation } from "@/shared/focus/useTvNavigation";
import { lumaApi, toUserMessage } from "@/shared/tauri/api";
import { openNativePlayer, shouldUseDesktopPlayer } from "@/shared/tauri/player";
import type {
  Channel,
  ChannelGroup,
  PlayChannelResponse,
  ProbeStatus,
} from "@/shared/tauri/types";

type View = "home" | "settings";

function useClock() {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), 30_000);
    return () => window.clearInterval(timer);
  }, []);

  return now.toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
  });
}

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
  const [playingChannel, setPlayingChannel] = useState<PlayChannelResponse | null>(
    null,
  );
  const [probing, setProbing] = useState(false);
  const [probeStatusById, setProbeStatusById] = useState<Record<string, ProbeStatus>>(
    {},
  );
  const [probeSummary, setProbeSummary] = useState<{
    playable: number;
    unreachable: number;
    invalid: number;
  } | null>(null);
  const [showPlayableOnly, setShowPlayableOnly] = useState(false);
  const { onKeyDown } = useTvNavigation();
  const clock = useClock();

  const loadData = useCallback(async (options?: { silent?: boolean }) => {
    const silent = options?.silent ?? false;
    if (!silent) {
      setLoading(true);
    }
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
      if (!silent) {
        setChannels([]);
        setGroups([]);
        setFavorites([]);
        setRecent([]);
        setFavoriteIds(new Set());
      }
    } finally {
      if (!silent) {
        setLoading(false);
      }
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

  const applyProbeReport = (
    report: Awaited<ReturnType<typeof lumaApi.probeChannels>>,
  ) => {
    setProbeStatusById((previous) => {
      const next = { ...previous };
      for (const result of report.results) {
        next[result.channelId] = result.status;
      }
      return next;
    });
    setProbeSummary({
      playable: report.playable,
      unreachable: report.unreachable,
      invalid: report.invalid,
    });
  };

  const runProbe = async (channelIds?: string[]) => {
    setProbing(true);
    try {
      const report = await lumaApi.probeChannels(channelIds);
      applyProbeReport(report);
      setToast(
        `检测完成：可用 ${report.playable}，不可达 ${report.unreachable}，无效 ${report.invalid}`,
      );
    } catch (err) {
      setToast(toUserMessage(err));
    } finally {
      setProbing(false);
    }
  };

  const handlePlay = async (channelId: string) => {
    setLastFocusedChannelId(channelId);
    try {
      const payload = await lumaApi.playChannel(channelId);
      if (shouldUseDesktopPlayer()) {
        setPlayingChannel(payload);
      } else {
        await openNativePlayer(payload);
      }
      await loadData({ silent: true });
    } catch (err) {
      setToast(toUserMessage(err));
    }
  };

  const handleToggleFavorite = async (channelId: string) => {
    const wasFavorite = favoriteIds.has(channelId);
    const previousIds = favoriteIds;
    const previousFavorites = favorites;

    setFavoriteIds((current) => {
      const next = new Set(current);
      if (wasFavorite) {
        next.delete(channelId);
      } else {
        next.add(channelId);
      }
      return next;
    });

    setFavorites((current) => {
      if (wasFavorite) {
        return current.filter((channel) => channel.id !== channelId);
      }
      const channel =
        channels.find((item) => item.id === channelId) ??
        current.find((item) => item.id === channelId);
      return channel ? [...current, channel] : current;
    });

    try {
      await lumaApi.toggleFavorite(channelId);
    } catch (err) {
      setFavoriteIds(previousIds);
      setFavorites(previousFavorites);
      setToast(toUserMessage(err));
    }
  };

  return (
    <div
      className={`app-shell ${
        view === "home" && (section === "all" || section === "group") && groups.length > 0
          ? "with-categories"
          : ""
      }`}
      onKeyDown={onKeyDown}
    >
      <Sidebar
        activeSection={section === "group" ? "all" : section}
        settingsActive={view === "settings"}
        onSelect={(nextSection) => {
          setSection(nextSection);
          setActiveGroup(null);
          setView("home");
        }}
        onOpenSettings={() => setView("settings")}
      />
      <CategoryPanel
        groups={groups}
        activeGroup={activeGroup}
        visible={view === "home" && (section === "all" || section === "group")}
        onSelectGroup={(name) => {
          if (!name) {
            setSection("all");
            setActiveGroup(null);
          } else {
            setSection("group");
            setActiveGroup(name);
          }
        }}
      />
      <main className="content">
        {error ? <div className="error-banner">{error}</div> : null}
        {view === "settings" ? (
          <ScrollArea className="content-scroll" hideScrollbar>
            <ImportPlaylistPanel
              onImported={() => {
                setView("home");
                void loadData();
              }}
            />
          </ScrollArea>
        ) : (
          <ChannelGrid
            section={section}
            groupName={activeGroup}
            channels={visibleChannels}
            featuredChannel={recent[0] ?? channels[0] ?? null}
            favoriteIds={favoriteIds}
            loading={loading}
            probing={probing}
            probeStatusById={probeStatusById}
            probeSummary={probeSummary}
            showPlayableOnly={showPlayableOnly}
            clock={clock}
            onPlay={handlePlay}
            onToggleFavorite={handleToggleFavorite}
            onProbeVisible={() =>
              void runProbe(visibleChannels.map((channel) => channel.id))
            }
            onProbeAll={() => void runProbe()}
            onTogglePlayableOnly={() => setShowPlayableOnly((value) => !value)}
            onOpenSettings={() => setView("settings")}
          />
        )}
      </main>
      {toast ? <div className="toast">{toast}</div> : null}
      {playingChannel ? (
        <DesktopPlayer
          channel={playingChannel}
          onClose={() => setPlayingChannel(null)}
        />
      ) : null}
    </div>
  );
}
