import { useState } from "react";

import { lumaApi, toUserMessage } from "@/shared/tauri/api";

interface ImportPlaylistPanelProps {
  onImported: () => void;
}

export function ImportPlaylistPanel({ onImported }: ImportPlaylistPanelProps) {
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const importFromUrl = async () => {
    setLoading(true);
    setError(null);
    setMessage(null);
    try {
      const playlist = await lumaApi.importPlaylistFromUrl(url.trim());
      setMessage(`已导入 ${playlist.channels.length} 个频道`);
      onImported();
    } catch (err) {
      setError(toUserMessage(err));
    } finally {
      setLoading(false);
    }
  };

  const importFromFile = async () => {
    setLoading(true);
    setError(null);
    setMessage(null);
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const { readTextFile } = await import("@tauri-apps/plugin-fs");
      const selected = await open({
        multiple: false,
        filters: [{ name: "M3U Playlist", extensions: ["m3u", "m3u8", "txt"] }],
      });

      if (!selected || Array.isArray(selected)) {
        return;
      }

      const content = await readTextFile(selected);
      const displayName = selected.split(/[\\/]/).pop() ?? "playlist.m3u";
      const playlist = await lumaApi.importPlaylistFromText(content, {
        type: "File",
        path: selected,
        displayName,
      });
      setMessage(`已导入 ${playlist.channels.length} 个频道`);
      onImported();
    } catch (err) {
      setError(toUserMessage(err));
    } finally {
      setLoading(false);
    }
  };

  const refresh = async () => {
    setLoading(true);
    setError(null);
    setMessage(null);
    try {
      const playlist = await lumaApi.refreshPlaylist();
      setMessage(`已刷新 ${playlist.channels.length} 个频道`);
      onImported();
    } catch (err) {
      setError(toUserMessage(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <section className="import-panel">
      <h2>导入播放列表</h2>
      <p>请仅导入您拥有合法使用权的 M3U / M3U8 播放列表。</p>
      {error ? <div className="error-banner">{error}</div> : null}
      {message ? <div className="empty-state">{message}</div> : null}
      <label>
        网络地址
        <input
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          placeholder="https://example.com/playlist.m3u"
        />
      </label>
      <div>
        <button
          type="button"
          className="primary-button"
          disabled={loading || !url.trim()}
          onClick={importFromUrl}
        >
          从 URL 导入
        </button>
      </div>
      <div>
        <button
          type="button"
          className="secondary-button"
          disabled={loading}
          onClick={importFromFile}
        >
          从本地文件导入
        </button>
      </div>
      <div>
        <button
          type="button"
          className="secondary-button"
          disabled={loading}
          onClick={refresh}
        >
          手动刷新当前列表
        </button>
      </div>
    </section>
  );
}
