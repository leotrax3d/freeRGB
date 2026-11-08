import type { ReleaseAsset } from "virtual:release"
import { CopyButton } from "./CopyButton"
import { formatBytes } from "@/lib/paths"

const short = (hash: string) => `${hash.slice(0, 12)}…${hash.slice(-6)}`

/** Every file of the release with size and SHA-256. Stacks into cards below md. */
export function AssetTable({ assets }: { assets: ReleaseAsset[] }) {
  return (
    <table className="w-full text-left text-small max-md:block">
      <caption className="sr-only">Files in this release with size and SHA-256 checksum</caption>
      <thead className="max-md:sr-only">
        <tr className="border-b text-caption text-subtle-foreground">
          <th scope="col" className="py-3 pr-4 font-medium">
            File
          </th>
          <th scope="col" className="py-3 pr-4 text-right font-medium">
            Size
          </th>
          <th scope="col" className="py-3 font-medium">
            SHA-256
          </th>
        </tr>
      </thead>
      <tbody className="max-md:block">
        {assets.map((asset) => (
          <tr key={asset.name} className="border-b last:border-b-0 max-md:block max-md:py-4">
            <td className="py-3 pr-4 align-middle max-md:block max-md:py-0">
              <a
                href={asset.url}
                className="break-all rounded-sm text-foreground underline decoration-border-strong underline-offset-4 transition-colors duration-fast ease-out hover:decoration-foreground"
              >
                {asset.name}
              </a>
            </td>
            <td className="whitespace-nowrap py-3 pr-4 text-right align-middle tabular-nums text-muted-foreground max-md:block max-md:py-1 max-md:text-left">
              <span className="md:sr-only">Size: </span>
              {formatBytes(asset.size)}
            </td>
            <td className="py-3 align-middle max-md:block max-md:py-0">
              {asset.sha256 ? (
                <div className="flex items-center gap-3">
                  <code className="min-w-0 flex-1 font-mono text-[13px] leading-5 text-muted-foreground" title={asset.sha256}>
                    <span aria-hidden>{short(asset.sha256)}</span>
                    <span className="sr-only">SHA-256 {asset.sha256}</span>
                  </code>
                  <CopyButton text={asset.sha256} label={`Copy SHA-256 of ${asset.name}`} />
                </div>
              ) : (
                <span className="text-subtle-foreground">Not published</span>
              )}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}
