import { useCallback } from 'react'

import { SpatialNavigator, type FocusDirection } from './focusNav'

const KEY_DIRECTIONS: Record<string, FocusDirection> = {
  ArrowLeft: 'left',
  ArrowRight: 'right',
  ArrowUp: 'up',
  ArrowDown: 'down'
}

/** Arrow keys inside form fields must keep moving the caret, not the focus. */
const TEXT_INPUT_SELECTOR = 'input, textarea, select'

const spatialNavigator = new SpatialNavigator()

/**
 * TV remote spatial navigation for the whole app shell: one engine covers
 * sidebar, category panel, toolbar and channel cards, moving focus
 * geometrically (up/down/left/right) instead of by DOM order.
 */
export function useTvNavigation() {
  return useCallback((event: React.KeyboardEvent) => {
    const direction = KEY_DIRECTIONS[event.key]
    if (!direction) {
      return
    }

    const active = document.activeElement
    if (active instanceof HTMLElement && active.matches(TEXT_INPUT_SELECTOR)) {
      return
    }

    const root = event.currentTarget as HTMLElement
    const next = spatialNavigator.findNext(root, active, direction)
    if (next) {
      event.preventDefault()
      next.focus()
      next.scrollIntoView({ block: 'nearest', inline: 'nearest' })
    }
  }, [])
}
