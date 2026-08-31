import { LUMA_BRAND_ARC_PATH, LUMA_BRAND_PLAY_PATH } from './brandPaths'

interface LumaBrandMarkProps {
  size?: number
  animated?: boolean
  className?: string
}

export function LumaBrandMark({ size = 64, animated = false, className = '' }: LumaBrandMarkProps) {
  return (
    <svg
      className={`luma-brand-mark ${animated ? 'luma-brand-mark--animated' : ''} ${className}`.trim()}
      width={size}
      height={size}
      viewBox="0 0 64 64"
      fill="none"
      aria-hidden
    >
      <path
        className="luma-brand-mark__arc"
        d={LUMA_BRAND_ARC_PATH}
        stroke="currentColor"
        strokeWidth="3"
        strokeLinecap="round"
      />
      <path className="luma-brand-mark__play" d={LUMA_BRAND_PLAY_PATH} fill="currentColor" />
    </svg>
  )
}
