import type { ReactNode, SVGAttributes } from 'react'

export interface IconProps extends SVGAttributes<SVGSVGElement> {
  size?: number
}

export function Icon({
  size = 24,
  children,
  viewBox = '0 0 24 24',
  fill = 'none',
  stroke = 'currentColor',
  strokeWidth = 1.75,
  strokeLinecap = 'round',
  strokeLinejoin = 'round',
  ...props
}: IconProps & { children: ReactNode }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox={viewBox}
      fill={fill}
      stroke={stroke}
      strokeWidth={strokeWidth}
      strokeLinecap={strokeLinecap}
      strokeLinejoin={strokeLinejoin}
      aria-hidden={props['aria-hidden'] ?? true}
      {...props}
    >
      {children}
    </svg>
  )
}
