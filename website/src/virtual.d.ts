declare module "virtual:devices" {
  /** [name, family, transport, ids, status (s|b|n), module] */
  const data: { rows: [string, string, string, string, "s" | "b" | "n", string][] }
  export default data
}

declare module "virtual:fans" {
  const data: {
    lhmVersion: string
    chips: { chip: string; name: string; vendor: string; boards: string[]; fans: number; controls: number; status: string }[]
  }
  export default data
}

declare module "virtual:guide" {
  const data: { html: string; toc: { id: string; text: string; level: number }[] }
  export default data
}

declare module "virtual:release" {
  export type ReleaseAsset = { name: string; size: number; url: string; sha256: string | null }
  export type Release = { tag: string; name: string; url: string; publishedAt: string; prerelease: boolean; assets: ReleaseAsset[] }
  const data: { release: Release | null; stars: number | null; fetchedAt: string | null }
  export default data
}
