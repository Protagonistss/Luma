import {
  createContext,
  useContext,
  useRef,
  type CSSProperties,
  type ReactNode,
  type RefObject
} from 'react'
import SimpleBar from 'simplebar-react'

import 'simplebar-react/dist/simplebar.min.css'

const ScrollElementContext = createContext<RefObject<HTMLElement | null> | null>(null)

export function useScrollElement() {
  return useContext(ScrollElementContext)
}

interface ScrollAreaProps {
  children: ReactNode
  className?: string
  style?: CSSProperties
  hideScrollbar?: boolean
}

export function ScrollArea({ children, className, style, hideScrollbar = false }: ScrollAreaProps) {
  const scrollRef = useRef<HTMLElement | null>(null)
  const classes = ['scroll-area', className].filter(Boolean).join(' ')

  if (hideScrollbar) {
    return (
      <ScrollElementContext.Provider value={scrollRef}>
        <div
          ref={scrollRef as RefObject<HTMLDivElement>}
          className={`${classes} scroll-area--native`}
          style={{ overflow: 'auto', ...style }}
        >
          {children}
        </div>
      </ScrollElementContext.Provider>
    )
  }

  return (
    <ScrollElementContext.Provider value={scrollRef}>
      <SimpleBar
        className={classes}
        style={style}
        autoHide
        scrollbarMinSize={48}
        scrollableNodeProps={{ ref: scrollRef }}
      >
        {children}
      </SimpleBar>
    </ScrollElementContext.Provider>
  )
}
