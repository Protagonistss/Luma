import { useCallback } from "react";

type Direction = "left" | "right" | "up" | "down";

function findNextChannelCard(
  cards: HTMLElement[],
  currentRect: DOMRect,
  direction: Direction,
) {
  const currentCenterX = currentRect.left + currentRect.width / 2;
  const currentCenterY = currentRect.top + currentRect.height / 2;
  let best: HTMLElement | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;

  for (const card of cards) {
    const rect = card.getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;
    const deltaX = centerX - currentCenterX;
    const deltaY = centerY - currentCenterY;

    const matchesDirection =
      (direction === "right" && deltaX > 24 && Math.abs(deltaY) < currentRect.height * 1.2) ||
      (direction === "left" && deltaX < -24 && Math.abs(deltaY) < currentRect.height * 1.2) ||
      (direction === "down" && deltaY > 24) ||
      (direction === "up" && deltaY < -24);

    if (!matchesDirection) {
      continue;
    }

    const distance = Math.hypot(deltaX, deltaY);
    if (distance < bestDistance) {
      bestDistance = distance;
      best = card;
    }
  }

  return best;
}

export function useChannelGridNavigation() {
  return useCallback((event: React.KeyboardEvent) => {
    const active = document.activeElement;
    if (!(active instanceof HTMLElement) || !active.classList.contains("channel-card")) {
      return false;
    }

    let direction: Direction | null = null;
    switch (event.key) {
      case "ArrowRight":
        direction = "right";
        break;
      case "ArrowLeft":
        direction = "left";
        break;
      case "ArrowDown":
        direction = "down";
        break;
      case "ArrowUp":
        direction = "up";
        break;
      default:
        return false;
    }

    const cards = Array.from(
      document.querySelectorAll<HTMLElement>(".channel-card"),
    );
    const next = findNextChannelCard(cards, active.getBoundingClientRect(), direction);
    if (!next) {
      return false;
    }

    event.preventDefault();
    next.focus({ preventScroll: false });
    next.scrollIntoView({ block: "nearest", inline: "nearest" });
    return true;
  }, []);
}
