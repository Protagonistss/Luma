import { useState } from "react";

import type { Channel } from "@/shared/tauri/types";

interface ChannelCardProps {
  channel: Channel;
  isFavorite: boolean;
  onPlay: (channelId: string) => void;
  onToggleFavorite: (channelId: string) => void;
}

export function ChannelCard({
  channel,
  isFavorite,
  onPlay,
  onToggleFavorite,
}: ChannelCardProps) {
  const [logoFailed, setLogoFailed] = useState(false);

  return (
    <div
      className="channel-card"
      data-channel-id={channel.id}
      role="button"
      tabIndex={0}
      onClick={() => onPlay(channel.id)}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onPlay(channel.id);
        }
      }}
    >
      {channel.logo && !logoFailed ? (
        <img
          className="channel-logo"
          src={channel.logo}
          alt={channel.name}
          onError={() => setLogoFailed(true)}
        />
      ) : (
        <div className="channel-logo placeholder" aria-hidden>
          {channel.name.slice(0, 1).toUpperCase()}
        </div>
      )}
      <div>
        <strong>{channel.name}</strong>
        <div>{channel.group}</div>
      </div>
      <div className="channel-meta">
        <span>播放</span>
        <button
          type="button"
          className={`favorite-button ${isFavorite ? "active" : ""}`}
          onClick={(event) => {
            event.stopPropagation();
            onToggleFavorite(channel.id);
          }}
        >
          {isFavorite ? "已收藏" : "收藏"}
        </button>
      </div>
    </div>
  );
}
