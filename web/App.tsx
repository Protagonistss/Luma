import { useEffect, useState } from "react";

import { AppShell } from "@/app/AppShell";
import { SplashScreen } from "@/app/SplashScreen";
import { WindowTitleBar } from "@/app/WindowTitleBar";
import { isDesktopTauri } from "@/shared/platform";
const SPLASH_VISIBLE_MS = 1000;
const SPLASH_EXIT_MS = 400;

export function App() {
  const [splashPhase, setSplashPhase] = useState<"visible" | "exiting" | "done">(
    "visible",
  );

  useEffect(() => {
    const exitTimer = window.setTimeout(
      () => setSplashPhase("exiting"),
      SPLASH_VISIBLE_MS,
    );
    const doneTimer = window.setTimeout(
      () => setSplashPhase("done"),
      SPLASH_VISIBLE_MS + SPLASH_EXIT_MS,
    );

    return () => {
      window.clearTimeout(exitTimer);
      window.clearTimeout(doneTimer);
    };
  }, []);

  const showDesktopChrome = isDesktopTauri();

  return (
    <div className={showDesktopChrome ? "desktop-shell" : "app-root"}>
      {showDesktopChrome ? <WindowTitleBar /> : null}
      <AppShell />
      {splashPhase !== "done" ? (
        <SplashScreen exiting={splashPhase === "exiting"} />
      ) : null}
    </div>
  );
}