/**
 * Spatial (directional) focus navigation for TV remote control.
 *
 * The engine measures every focusable element inside a root container once,
 * caches the geometry, and answers "which element is next when pressing
 * left/right/up/down" with nearest-neighbor math instead of re-querying the
 * DOM on every keypress.
 *
 * Coordinates are stored in page space (client rect + window scroll offset)
 * so the cache survives window scrolling. Inner scroll containers, resizes
 * and DOM changes (virtualized rows re-rendering) invalidate the snapshot
 * automatically via capture-phase listeners.
 */

export type FocusDirection = 'left' | 'right' | 'up' | 'down'

export interface FocusableGeometry {
  centerX: number
  centerY: number
  width: number
  height: number
}

export interface NavSnapshot {
  elements: readonly HTMLElement[]
  geometry: readonly FocusableGeometry[]
}

/** Minimum center-to-center distance (px) before a neighbor counts as "in that direction". */
const DIRECTION_THRESHOLD = 24

export const FOCUSABLE_SELECTOR =
  'button,[href],input,select,textarea,[tabindex]:not([tabindex="-1"])'

/**
 * Pick the best candidate for `direction` from a snapshot.
 *
 * Horizontal moves require the candidate to stay within the current row
 * (tolerance based on card height) so focus never jumps diagonally at row
 * ends. Vertical moves prefer the nearest candidate below/above, which
 * naturally lands in the same column for grid layouts.
 *
 * Pure function: fully unit-testable without a DOM.
 */
export function findNextIndex(
  snapshot: NavSnapshot,
  currentIndex: number,
  direction: FocusDirection
): number {
  const current = snapshot.geometry[currentIndex]
  if (!current) {
    return -1
  }

  const rowTolerance = Math.max(current.height, 8) * 1.2
  let bestIndex = -1
  let bestDistance = Number.POSITIVE_INFINITY

  for (let index = 0; index < snapshot.geometry.length; index += 1) {
    if (index === currentIndex) {
      continue
    }
    const candidate = snapshot.geometry[index]!
    const dx = candidate.centerX - current.centerX
    const dy = candidate.centerY - current.centerY

    const matchesDirection =
      (direction === 'left' && dx < -DIRECTION_THRESHOLD && Math.abs(dy) < rowTolerance) ||
      (direction === 'right' && dx > DIRECTION_THRESHOLD && Math.abs(dy) < rowTolerance) ||
      (direction === 'up' && dy < -DIRECTION_THRESHOLD) ||
      (direction === 'down' && dy > DIRECTION_THRESHOLD)

    if (!matchesDirection) {
      continue
    }

    const distance = Math.hypot(dx, dy)
    if (distance < bestDistance) {
      bestDistance = distance
      bestIndex = index
    }
  }

  return bestIndex
}

/**
 * Measure elements into a snapshot, dropping disabled or invisible items
 * (zero-size rects come from hidden views and collapsed containers).
 */
export function measureSnapshot(elements: readonly HTMLElement[]): NavSnapshot {
  const scrollX = window.scrollX
  const scrollY = window.scrollY
  const kept: HTMLElement[] = []
  const geometry: FocusableGeometry[] = []

  for (const element of elements) {
    if (element.hasAttribute('disabled')) {
      continue
    }
    const rect = element.getBoundingClientRect()
    if (rect.width < 1 || rect.height < 1) {
      continue
    }
    kept.push(element)
    geometry.push({
      centerX: rect.left + rect.width / 2 + scrollX,
      centerY: rect.top + rect.height / 2 + scrollY,
      width: rect.width,
      height: rect.height
    })
  }

  return { elements: kept, geometry }
}

/**
 * Caching navigator bound to one root container. Create one instance per
 * app shell; `findNext` falls back to a rebuild when the snapshot went
 * stale (detected by listeners below) or the focused element is missing.
 */
export class SpatialNavigator {
  private snapshot: NavSnapshot | null = null
  private snapshotRoot: HTMLElement | null = null

  constructor(private readonly selector: string = FOCUSABLE_SELECTOR) {
    registerInvalidationListener(this)
  }

  invalidate(): void {
    this.snapshot = null
  }

  findNext(
    root: HTMLElement,
    current: Element | null,
    direction: FocusDirection
  ): HTMLElement | null {
    if (!current) {
      return null
    }

    let snapshot = this.snapshot
    if (!snapshot || this.snapshotRoot !== root) {
      snapshot = this.rebuild(root)
    }

    let index = snapshot.elements.indexOf(current as HTMLElement)
    if (index < 0) {
      // The focused element is not in the cached set (render raced the
      // cache): rebuild once and retry.
      snapshot = this.rebuild(root)
      index = snapshot.elements.indexOf(current as HTMLElement)
      if (index < 0) {
        return null
      }
    }

    const next = findNextIndex(snapshot, index, direction)
    return next >= 0 ? snapshot.elements[next]! : null
  }

  private rebuild(root: HTMLElement): NavSnapshot {
    const elements = Array.from(root.querySelectorAll<HTMLElement>(this.selector))
    const snapshot = measureSnapshot(elements)
    this.snapshot = snapshot
    this.snapshotRoot = root
    return snapshot
  }
}

const navigators = new Set<SpatialNavigator>()
let invalidationStarted = false

function registerInvalidationListener(navigator: SpatialNavigator): void {
  navigators.add(navigator)
  if (invalidationStarted || typeof window === 'undefined') {
    return
  }
  invalidationStarted = true

  const invalidateAll = () => {
    for (const item of navigators) {
      item.invalidate()
    }
  }

  window.addEventListener('resize', invalidateAll)
  // Capture phase: scroll events do not bubble, but capturing on window
  // still observes every inner scroll container.
  window.addEventListener('scroll', invalidateAll, true)
  const observer = new MutationObserver(invalidateAll)
  observer.observe(document.body, { childList: true, subtree: true })
}
