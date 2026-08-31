import { LUMA_ARC_PATH, LUMA_PLAY_PATH } from './brandPaths'
import { Icon, type IconProps } from './Icon'

export function LumaLogoIcon(props: IconProps) {
  return (
    <Icon {...props} strokeWidth={1.85}>
      <path d={LUMA_ARC_PATH} />
      <path d={LUMA_PLAY_PATH} fill="currentColor" stroke="none" />
    </Icon>
  )
}

export function LumaTitlebarIcon(props: IconProps) {
  return (
    <Icon {...props} stroke="#f3f5f8" strokeWidth={2.1}>
      <path d={LUMA_ARC_PATH} />
      <path d={LUMA_PLAY_PATH} fill="#f3f5f8" stroke="none" />
    </Icon>
  )
}

export function LumaLogoMonochromeIcon(props: IconProps) {
  return <LumaLogoIcon {...props} />
}

export function HomeIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4.5 10.5 12 5l7.5 5.5" />
      <path d="M6.5 11v7.5h4v-4.5h3v4.5h4V11" />
    </Icon>
  )
}

export function StarIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 4.5 14.1 9l4.9.7-3.5 3.4.8 4.9L12 15.8l-3.3 1.7.8-4.9-3.5-3.4 4.9-.7z" />
    </Icon>
  )
}

export function StarFilledIcon(props: IconProps) {
  return (
    <Icon {...props} fill="currentColor" stroke="none">
      <path d="M12 4.5 14.1 9l4.9.7-3.5 3.4.8 4.9L12 15.8l-3.3 1.7.8-4.9-3.5-3.4 4.9-.7z" />
    </Icon>
  )
}

export function ClockIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="12" cy="12" r="7.5" />
      <path d="M12 8.5V12l2.5 1.5" />
    </Icon>
  )
}

export function SettingsIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="12" cy="12" r="2.75" />
      <path d="M12 3.5v2M12 18.5v2M4.6 4.6l1.4 1.4M18 18l1.4 1.4M3.5 12h2M18.5 12h2M4.6 19.4l1.4-1.4M18 6l1.4-1.4" />
    </Icon>
  )
}

export function SearchIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="11" cy="11" r="5.5" />
      <path d="m16 16 4.5 4.5" />
    </Icon>
  )
}

export function ChevronRightIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="m9.5 6.5 5 5.5-5 5.5" />
    </Icon>
  )
}

export function LiveIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="12" cy="12" r="2.25" fill="currentColor" stroke="none" />
      <path d="M5.5 8.5a7 7 0 0 0 0 7M18.5 8.5a7 7 0 0 1 0 7" />
    </Icon>
  )
}

export function GridIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <rect x="4.5" y="4.5" width="6" height="6" rx="1.5" />
      <rect x="13.5" y="4.5" width="6" height="6" rx="1.5" />
      <rect x="4.5" y="13.5" width="6" height="6" rx="1.5" />
      <rect x="13.5" y="13.5" width="6" height="6" rx="1.5" />
    </Icon>
  )
}

export function ProbeIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M7.5 12.5 10.5 15.5 16.5 8.5" />
      <circle cx="12" cy="12" r="7.5" />
    </Icon>
  )
}

export function ImportIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 4.5v9" />
      <path d="M8.5 11 12 14.5 15.5 11" />
      <path d="M5.5 17.5h13" />
    </Icon>
  )
}

export function FileIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M8.5 4.5h5l3 3v12h-8z" />
      <path d="M13.5 4.5V8h3" />
    </Icon>
  )
}

export function TrashIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M5.5 6.5h13" />
      <path d="M9.5 6.5V4.75a1.25 1.25 0 0 1 1.25-1.25h2.5a1.25 1.25 0 0 1 1.25 1.25V6.5" />
      <path d="M7.5 6.5 8.4 18.6a1.5 1.5 0 0 0 1.5 1.4h4.2a1.5 1.5 0 0 0 1.5-1.4L16.5 6.5" />
      <path d="M10.5 10v6" />
      <path d="M13.5 10v6" />
    </Icon>
  )
}

export function RefreshIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M18.5 8.5A6.5 6.5 0 0 0 7.2 7.2L5.5 5.5" />
      <path d="M5.5 9.5V5.5h4" />
      <path d="M5.5 15.5A6.5 6.5 0 0 0 16.8 16.8l1.7 1.7" />
      <path d="M18.5 14.5v4h-4" />
    </Icon>
  )
}
