import { invoke } from "@tauri-apps/api/core";

import type {
  Channel,
  ChannelGroup,
  CommandError,
  PlayChannelResponse,
  Playlist,
  PlaylistSource,
  ProbeReport,
} from "./types";

function isCommandError(error: unknown): error is CommandError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error
  );
}

export function toUserMessage(error: unknown): string {
  if (isCommandError(error)) {
    return error.message;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return "发生未知错误";
}

export const lumaApi = {
  importPlaylistFromUrl(url: string) {
    return invoke<Playlist>("import_playlist_from_url", { url });
  },
  importPlaylistFromText(content: string, source: PlaylistSource) {
    return invoke<Playlist>("import_playlist_from_text", { content, source });
  },
  refreshPlaylist() {
    return invoke<Playlist>("refresh_playlist");
  },
  listChannels(group?: string) {
    return invoke<Channel[]>("list_channels", { group });
  },
  listGroups() {
    return invoke<ChannelGroup[]>("list_groups");
  },
  toggleFavorite(channelId: string) {
    return invoke<boolean>("toggle_favorite", { channelId });
  },
  listFavorites() {
    return invoke<Channel[]>("list_favorites");
  },
  listRecent() {
    return invoke<Channel[]>("list_recent");
  },
  getPlaylistSource() {
    return invoke<PlaylistSource | null>("get_playlist_source");
  },
  playChannel(channelId: string) {
    return invoke<PlayChannelResponse>("play_channel", { channelId });
  },
  probeChannels(channelIds?: string[]) {
    return invoke<ProbeReport>("probe_channels", { channelIds });
  },
};
