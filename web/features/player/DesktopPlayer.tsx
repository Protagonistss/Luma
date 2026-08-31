import { useEffect, useRef, useState } from "react";
import Hls from "hls.js";

import type { PlayChannelResponse } from "@/shared/tauri/types";
import { resolveDesktopStreamUrl } from "@/shared/tauri/player";

interface DesktopPlayerProps {
  channel: PlayChannelResponse;
  onClose: () => void;
}

function formatHlsError(data: { type: string; details: string; fatal: boolean }) {
  return `${data.type} / ${data.details}`;
}

export function DesktopPlayer({ channel, onClose }: DesktopPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const hlsRef = useRef<Hls | null>(null);
  const [status, setStatus] = useState("正在连接直播...");
  const [error, setError] = useState<string | null>(null);
  const [showChrome, setShowChrome] = useState(true);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) {
      return;
    }

    let cancelled = false;
    let playbackUrl = channel.streamUrl;
    setStatus("正在连接直播...");
    setError(null);

    const cleanup = () => {
      hlsRef.current?.destroy();
      hlsRef.current = null;
      video.pause();
      video.removeAttribute("src");
      video.load();
    };

    const startPlayback = async () => {
      try {
        playbackUrl = await resolveDesktopStreamUrl(channel.streamUrl);
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "无法启动本地流代理");
          setStatus("");
        }
        return;
      }

      if (cancelled) {
        return;
      }

      if (!Hls.isSupported()) {
        if (video.canPlayType("application/vnd.apple.mpegurl")) {
          video.src = playbackUrl;
          video.addEventListener(
            "loadedmetadata",
            () => {
              if (cancelled) {
                return;
              }
              setStatus("");
              void video.play().catch(() => {
                setError("自动播放失败，请点击视频开始播放");
              });
            },
            { once: true },
          );
          video.addEventListener(
            "error",
            () => {
              if (!cancelled) {
                setError("播放失败，请检查流地址是否有效");
                setStatus("");
              }
            },
            { once: true },
          );
          return;
        }

        setError("当前环境不支持 HLS 播放");
        setStatus("");
        return;
      }

      const hls = new Hls({
        enableWorker: true,
        lowLatencyMode:
          channel.streamUrl.includes("live") || channel.streamUrl.includes("mux.dev"),
        xhrSetup(xhr) {
          xhr.withCredentials = false;
        },
      });
      hlsRef.current = hls;
      hls.attachMedia(video);
      hls.on(Hls.Events.MEDIA_ATTACHED, () => {
        hls.loadSource(playbackUrl);
      });
      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        if (cancelled) {
          return;
        }
        setStatus("");
        void video.play().catch(() => {
          setError("自动播放失败，请点击视频开始播放");
        });
      });
      hls.on(Hls.Events.ERROR, (_event, data) => {
        if (cancelled || !data.fatal) {
          return;
        }

        if (data.type === Hls.ErrorTypes.NETWORK_ERROR) {
          setStatus("网络异常，正在重试...");
          hls.startLoad();
          return;
        }

        if (data.type === Hls.ErrorTypes.MEDIA_ERROR) {
          setStatus("媒体解码异常，正在恢复...");
          hls.recoverMediaError();
          return;
        }

        const details = formatHlsError(data);
        const hint = channel.streamUrl.includes(".m3u8")
          ? "请确认该频道源有效；桌面版需通过 Tauri 应用（pnpm dev:desktop）播放。"
          : "该地址可能不是 HLS 直播流（.m3u8），而是网页链接，无法直接播放。";
        setError(`播放失败：${details}\n${hint}`);
        setStatus("");
      });
    };

    void startPlayback();

    return () => {
      cancelled = true;
      cleanup();
    };
  }, [channel.streamUrl, channel.name]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" || event.key === "Backspace") {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  useEffect(() => {
    let timer: number | undefined;
    const resetTimer = () => {
      setShowChrome(true);
      if (timer) {
        window.clearTimeout(timer);
      }
      timer = window.setTimeout(() => setShowChrome(false), 4000);
    };

    resetTimer();
    window.addEventListener("mousemove", resetTimer);
    window.addEventListener("keydown", resetTimer);

    return () => {
      if (timer) {
        window.clearTimeout(timer);
      }
      window.removeEventListener("mousemove", resetTimer);
      window.removeEventListener("keydown", resetTimer);
    };
  }, []);

  return (
    <div className="desktop-player">
      <div className="desktop-player-stage">
        <video
          ref={videoRef}
          className="desktop-player-video"
          playsInline
          autoPlay
          crossOrigin="anonymous"
        />
      </div>

      <div className={`desktop-player-chrome ${showChrome ? "visible" : ""}`}>
        <div className="desktop-player-info">
          <span className="desktop-player-live">
            <span className="live-dot" aria-hidden />
            直播中
          </span>
          <strong>{channel.name}</strong>
        </div>
        <button type="button" className="player-back-button" onClick={onClose}>
          返回
        </button>
      </div>

      {status ? <div className="desktop-player-overlay">{status}</div> : null}
      {error ? (
        <div className="desktop-player-overlay desktop-player-error">
          <p>{error}</p>
          <button type="button" className="ghost-button" onClick={onClose}>
            返回列表
          </button>
        </div>
      ) : null}
    </div>
  );
}
