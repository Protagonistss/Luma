import { useEffect, useState, type MouseEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { LumaLogoIcon } from "@/shared/icons";

function MinimizeIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
      <path d="M1 5.5h8" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function MaximizeIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
      <rect
        x="1.5"
        y="1.5"
        width="7"
        height="7"
        rx="1"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.2"
      />
    </svg>
  );
}

function RestoreIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
      <path
        d="M3.5 2.5h4v4M2.5 3.5v4h4"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
      <path
        d="m2 2 6 6M8 2 2 8"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinecap="round"
      />
    </svg>
  );
}

export function WindowTitleBar() {
  const [maximized, setMaximized] = useState(false);
  const appWindow = getCurrentWindow();

  useEffect(() => {
    let disposed = false;

    void appWindow.isMaximized().then((value) => {
      if (!disposed) {
        setMaximized(value);
      }
    });

    const unlistenPromise = appWindow.onResized(() => {
      void appWindow.isMaximized().then((value) => {
        if (!disposed) {
          setMaximized(value);
        }
      });
    });

    return () => {
      disposed = true;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [appWindow]);

  const handleTitlebarMouseDown = (event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0) {
      return;
    }

    const target = event.target as HTMLElement;
    if (target.closest(".window-titlebar__controls")) {
      return;
    }

    if (event.detail === 2) {
      void appWindow.toggleMaximize();
      return;
    }

    void appWindow.startDragging();
  };

  return (
    <header
      className="window-titlebar"
      data-tauri-drag-region
      onMouseDown={handleTitlebarMouseDown}
    >
      <div className="window-titlebar__drag" data-tauri-drag-region>
        <span
          className="window-titlebar__brand"
          aria-label="Luma"
          data-tauri-drag-region
        >
          <LumaLogoIcon size={20} />
        </span>
      </div>

      <div className="window-titlebar__controls">
        <button
          type="button"
          className="window-titlebar__button"
          aria-label="最小化"
          onClick={() => void appWindow.minimize()}
        >
          <MinimizeIcon />
        </button>
        <button
          type="button"
          className="window-titlebar__button"
          aria-label={maximized ? "还原" : "最大化"}
          onClick={() => void appWindow.toggleMaximize()}
        >
          {maximized ? <RestoreIcon /> : <MaximizeIcon />}
        </button>
        <button
          type="button"
          className="window-titlebar__button window-titlebar__button--close"
          aria-label="关闭"
          onClick={() => void appWindow.close()}
        >
          <CloseIcon />
        </button>
      </div>
    </header>
  );
}
