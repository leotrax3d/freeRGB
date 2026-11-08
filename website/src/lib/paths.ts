/** Site-relative link that respects BASE_PATH (GitHub Pages serves the site under /<repo>/). */
export function href(path: string) {
  return `${import.meta.env.BASE_URL}${path}`
}

export function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

export function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString("en-GB", { year: "numeric", month: "long", day: "numeric", timeZone: "UTC" })
}
