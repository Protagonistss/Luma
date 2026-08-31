import { describe, expect, it } from 'vitest'

import { findNextIndex, type FocusableGeometry, type NavSnapshot } from './focusNav'

const CARD_WIDTH = 168
const CARD_HEIGHT = 105
const GAP = 16

function geometryAt(column: number, row: number): FocusableGeometry {
  return {
    centerX: column * (CARD_WIDTH + GAP),
    centerY: row * (CARD_HEIGHT + GAP),
    width: CARD_WIDTH,
    height: CARD_HEIGHT
  }
}

/** Build a synthetic `columns × rows` grid snapshot (index = row * columns + column). */
function gridSnapshot(columns: number, rows: number): NavSnapshot {
  const geometry: FocusableGeometry[] = []
  for (let row = 0; row < rows; row += 1) {
    for (let column = 0; column < columns; column += 1) {
      geometry.push(geometryAt(column, row))
    }
  }
  return { elements: [], geometry }
}

describe('findNextIndex', () => {
  it('moves right within a row', () => {
    const snapshot = gridSnapshot(3, 3)
    // (row 1, col 0) -> (row 1, col 1)
    expect(findNextIndex(snapshot, 3, 'right')).toBe(4)
  })

  it('moves left within a row', () => {
    const snapshot = gridSnapshot(3, 3)
    expect(findNextIndex(snapshot, 5, 'left')).toBe(4)
  })

  it('does not wrap from row end to next row start', () => {
    const snapshot = gridSnapshot(3, 3)
    // (row 0, col 2) has nothing to its right within the row
    expect(findNextIndex(snapshot, 2, 'right')).toBe(-1)
  })

  it('does not wrap from row start to previous row end', () => {
    const snapshot = gridSnapshot(3, 3)
    expect(findNextIndex(snapshot, 3, 'left')).toBe(-1)
  })

  it('moves down to the element in the same column', () => {
    const snapshot = gridSnapshot(3, 3)
    // (row 0, col 1) -> (row 1, col 1)
    expect(findNextIndex(snapshot, 1, 'down')).toBe(4)
  })

  it('moves up to the element in the same column', () => {
    const snapshot = gridSnapshot(3, 3)
    // (row 2, col 2) -> (row 1, col 2)
    expect(findNextIndex(snapshot, 8, 'up')).toBe(5)
  })

  it('returns -1 above the top row and below the bottom row', () => {
    const snapshot = gridSnapshot(3, 3)
    expect(findNextIndex(snapshot, 0, 'up')).toBe(-1)
    expect(findNextIndex(snapshot, 6, 'down')).toBe(-1)
  })

  it('prefers the same column over a diagonally drifted candidate', () => {
    const snapshot: NavSnapshot = {
      elements: [],
      geometry: [
        geometryAt(1, 0), // current
        { centerX: 100, centerY: 130, width: 60, height: 30 }, // same column, small offset
        { centerX: 1, centerY: 121, width: 60, height: 30 } // far left, slightly closer in Y
      ]
    }
    expect(findNextIndex(snapshot, 0, 'down')).toBe(1)
  })

  it('ignores candidates outside the row tolerance for horizontal moves', () => {
    const snapshot: NavSnapshot = {
      elements: [],
      geometry: [
        geometryAt(0, 0), // current
        geometryAt(1, 0), // proper right neighbor
        geometryAt(3, 2) // far right but two rows down — must not match
      ]
    }
    expect(findNextIndex(snapshot, 0, 'right')).toBe(1)
  })

  it('falls back to a drifted candidate when the column is empty below', () => {
    // Shelf layout: last row of shelf A (partial), first full row of shelf B.
    const snapshot: NavSnapshot = {
      elements: [],
      geometry: [
        geometryAt(0, 0),
        geometryAt(1, 0),
        geometryAt(0, 1), // current: end of shelf A last row
        geometryAt(1, 2) // only candidate below is offset to the right
      ]
    }
    expect(findNextIndex(snapshot, 2, 'down')).toBe(3)
  })

  it('returns -1 for an unknown current index', () => {
    const snapshot = gridSnapshot(2, 2)
    expect(findNextIndex(snapshot, 99, 'right')).toBe(-1)
  })
})
