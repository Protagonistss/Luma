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
        setLoading(false);
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
    <section className="settings-stage">
      <header className="settings-header">
        <p className="kicker">设置</p>
        <h2>导入播放列表</h2>
        <p className="settings-desc">
          导入你拥有合法使用权的 M3U / M3U8 文件或链接。Luma 仅提供播放能力，不提供节目源。
        </p>
      </header>

      {error ? <div className="import-feedback import-feedback--error">{error}</div> : null}
      {message ? <div className="import-feedback import-feedback--success">{message}</div> : null}

      <div className="import-form">
        <label className="import-url-group">
          <span className="import-label">播放列表地址</span>
          <div className="import-url-row">
            <input
              className="import-url-input"
              value={url}
              onChange={(event) => setUrl(event.target.value)}
              placeholder="https://example.com/playlist.m3u"
              disabled={loading}
              onKeyDown={(event) => {
                if (event.key === "Enter" && url.trim() && !loading) {
                  void importFromUrl();
                }
              }}
            />
            <button
              type="button"
              className="primary-button import-url-submit"
              disabled={loading || !url.trim()}
              onClick={importFromUrl}
            >
              {loading ? "导入中" : "导入"}
            </button>
          </div>
        </label>

        <div className="import-divider" aria-hidden />

        <div className="import-options">
          <button
            type="button"
            className="import-option"
            disabled={loading}
            onClick={importFromFile}
          >
            <span className="import-option__text">
              <strong>从本地文件导入</strong>
              <span>m3u · m3u8 · txt</span>
            </span>
            <span className="import-option__arrow" aria-hidden>
              →
            </span>
          </button>
          <button
            type="button"
            className="import-option"
            disabled={loading}
            onClick={refresh}
          >
            <span className="import-option__text">
              <strong>刷新当前列表</strong>
              <span>重新下载已保存的 URL</span>
            </span>
            <span className="import-option__arrow" aria-hidden>
              →
            </span>
          </button>
        </div>
      </div>
    </section>
  );
}
