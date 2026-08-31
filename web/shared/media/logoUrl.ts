export function isRenderableLogoUrl(logo?: string | null): logo is string {
  if (!logo) {
    return false
  }

  const value = logo.trim()
  return value.startsWith('http://') || value.startsWith('https://')
}
