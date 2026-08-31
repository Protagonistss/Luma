import { useCallback, useEffect, useRef } from "react";

import { useChannelGridNavigation } from "./useChannelGridNavigation";

const FOCUSABLE_SELECTOR =
  'button,[href],input,select,textarea,[tabindex]:not([tabindex="-1"])';

export function useFocusTrap(containerRef: React.RefObject<HTMLElement>) {
  const lastFocused = useRef<HTMLElement | null>(null);

  useEffect(() => {
    lastFocused.current = document.activeElement as HTMLElement | null;
    const container = containerRef.current;
    if (!container) {
      return;
    }

    const first = container.querySelector<HTMLElement>(FOCUSABLE_SELECTOR);
    first?.focus();

    return () => {
      lastFocused.current?.focus();
    };
  }, [containerRef]);
}

export function useTvNavigation() {
  const onChannelGridKeyDown = useChannelGridNavigation();

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (onChannelGridKeyDown(event)) {
        return;
      }

      const target = event.currentTarget as HTMLElement;
      const focusables = Array.from(
        target.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR),
      ).filter((el) => !el.hasAttribute("disabled"));

      const index = focusables.indexOf(document.activeElement as HTMLElement);
      if (index < 0) {
        return;
      }

      const moveFocus = (nextIndex: number) => {
        const next = focusables[nextIndex];
        if (next) {
          event.preventDefault();
          next.focus();
        }
      };

      switch (event.key) {
        case "ArrowRight":
        case "ArrowDown":
          moveFocus(Math.min(index + 1, focusables.length - 1));
          break;
        case "ArrowLeft":
        case "ArrowUp":
          moveFocus(Math.max(index - 1, 0));
          break;
        default:
          break;
      }
    },
    [onChannelGridKeyDown],
  );

  return { onKeyDown };
}
