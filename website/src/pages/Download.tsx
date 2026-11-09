import { useEffect, useState, type ReactNode } from "react"
import { ArrowUpRight, Download, FileKey, Github, ShieldAlert, Terminal, TriangleAlert } from "lucide-react"
import data, { type ReleaseAsset } from "virtual:release"
import { AssetTable } from "@/components/download/AssetTable"
import { CodeBlock } from "@/components/download/CodeBlock"
import { useDetectedOs, type DetectedOs } from "@/components/download/useDetectedOs"
import { PageHeader, Section } from "@/components/site/Section"
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { blob, download, platforms, repo } from "@/content/site"
import { cn } from "@/lib/utils"
import { formatBytes, formatDate } from "@/lib/paths"

// Copy used only on this page (rules: header of src/content/site.ts).
const copy = {
  prerelease: "Pre-release",
  downloadInstaller: "Download installer",
  noInstaller: "This release has no Windows installer. Pick a file from the list on GitHub.",
  installerHash: "SHA-256 of the installer",
  caseNote: "PowerShell prints the value in upper case. Upper and lower case letters count as the same.",
  sidecars: "The release also has these checksum files:",
  updaterLink: "How the updater checks it",
  sigAsset: "Signature file",
  udevCaption: "Device permissions (udev), run in the cloned repository",
  cloneCaption: "Clone with git",
  viewOnGithub: "View on GitHub",
  downloadZip: "Download ZIP",
  filesTitle: "All files in this release",
  filesText: "Effect plugins, hardware modules and catalogs are downloaded by the app when you ask for them. You only need the installer.",
  showFiles: (n: number) => `Show all ${n} files`,
  detected: (os: string) => `Detected: ${os}`,
}

// Same key as docs/UPDATES.md and crates/freergb-modules/keys/release-keys.txt.
const RELEASE_PUBLIC_KEY = "23469f6bec07ca5df93240261eaf3015b2ca8dbe37226e201499d1a8c1823c4a"

// Copied from docs/INSTALL.md, "Device permissions (udev)".
const UDEV_COMMANDS = `sudo install -m 0644 packaging/linux/60-freergb.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger`

const CLONE_COMMAND = "git clone https://github.com/leotrax3d/freeRGB.git"

const release = data.release
const installer: ReleaseAsset | undefined = release?.assets.find((a) => /windows.*setup\.exe$/i.test(a.name))
const assetNamed = (name: string) => release?.assets.find((a) => a.name === name)
const sidecarSha = installer ? assetNamed(`${installer.name}.sha256`) : undefined
const sha256sums = assetNamed("SHA256SUMS")
const installerSig = installer ? assetNamed(`${installer.name}.sig`) : undefined
const installerName = installer?.name ?? "freeRGB-<version>-windows-x86_64-setup.exe"

function TextLink({ href, children, className }: { href: string; children: ReactNode; className?: string }) {
  return (
    <a
      href={href}
      className={cn(
        "inline-flex min-h-6 items-center gap-1 rounded-sm text-brand-foreground underline-offset-4 transition-colors duration-fast ease-out hover:underline",
        className,
      )}
    >
      {children}
      <ArrowUpRight aria-hidden className="size-4 shrink-0" />
    </a>
  )
}

/** Always rendered so it holds its space from the first paint; only visibility changes after mount. */
function DetectedBadge({ show, os }: { show: boolean; os: string }) {
  return (
    <Badge variant="brand" className={cn("shrink-0", !show && "invisible")} aria-hidden={!show}>
      {copy.detected(os)}
    </Badge>
  )
}

function BetaNotice() {
  return (
    <div className="container">
      <Card role="note" aria-labelledby="beta-title" className="flex gap-3 rounded-card px-4 py-3 shadow-none">
        <TriangleAlert aria-hidden className="mt-0.5 size-4 shrink-0 text-warning" />
        <p className="min-w-0 text-[13px] leading-5 text-muted-foreground">
          <strong id="beta-title" className="font-semibold text-foreground">
            {download.betaTitle}.
          </strong>{" "}
          {download.betaText}{" "}
          <TextLink href={blob("docs/COOLING_SAFETY.md")} className="text-[13px]">
            {download.betaLink}
          </TextLink>
        </p>
      </Card>
    </div>
  )
}

function WindowsCard({ os }: { os: DetectedOs }) {
  return (
    <Card id="windows" aria-labelledby="windows-title" className="min-w-0 scroll-mt-24 rounded-panel border-border-strong p-6 md:p-8">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 id="windows-title" className="text-h3">
          {download.windowsTitle}
        </h2>
        <DetectedBadge show={os === "windows"} os="Windows" />
      </div>

      {!release ? (
        <p className="mt-4 text-body text-muted-foreground">{download.noRelease}</p>
      ) : (
        <>
          <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-2 text-small text-muted-foreground">
            <span className="font-medium text-foreground">{release.name}</span>
            {release.prerelease && <Badge variant="warning">{copy.prerelease}</Badge>}
            <time dateTime={release.publishedAt}>{formatDate(release.publishedAt)}</time>
          </div>

          {installer ? (
            <>
              <div className="mt-6 flex flex-col gap-4 sm:flex-row sm:items-center">
                <Button size="lg" className="h-12 w-full px-6 text-body sm:w-auto" asChild>
                  <a href={installer.url}>
                    <Download aria-hidden />
                    {copy.downloadInstaller}
                  </a>
                </Button>
                <p className="min-w-0 text-small text-muted-foreground">
                  <span className="block break-all font-mono text-[13px] text-foreground">{installer.name}</span>
                  {formatBytes(installer.size)}
                </p>
              </div>
              {installer.sha256 && (
                <CodeBlock
                  className="mt-6"
                  caption={copy.installerHash}
                  code={installer.sha256}
                  copyLabel="Copy SHA-256 of the installer"
                  wrap
                />
              )}
            </>
          ) : (
            <p className="mt-6 text-body text-muted-foreground">{copy.noInstaller}</p>
          )}

        </>
      )}
    </Card>
  )
}

function LinuxCard({ os }: { os: DetectedOs }) {
  const detected = os === "linux"
  return (
    <Card
      id="linux"
      aria-labelledby="linux-title"
      className={cn(
        "flex min-w-0 scroll-mt-24 flex-col rounded-panel p-6 shadow-none transition-colors duration-fast ease-out md:p-8",
        detected && "border-border-strong",
      )}
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 id="linux-title" className="text-h3">
          {download.linuxTitle}
        </h2>
        <DetectedBadge show={detected} os="Linux" />
      </div>
      <p className="mt-3 text-small text-muted-foreground">{download.linuxText}</p>
      <CodeBlock wrap className="mt-5" caption={copy.udevCaption} code={UDEV_COMMANDS} copyLabel="Copy udev commands" />
      <div className="mt-6 lg:mt-auto lg:pt-6">
        <Button variant="outline" className="h-11 w-full sm:w-auto" asChild>
          <a href={`${blob("docs/INSTALL.md")}#linux`}>
            <Terminal aria-hidden />
            {download.linuxCta}
          </a>
        </Button>
      </div>
    </Card>
  )
}

const tabTrigger = "min-h-10 whitespace-normal px-2 py-2 text-center text-small sm:px-4"

function VerifySection({ os }: { os: DetectedOs }) {
  const [tab, setTab] = useState("powershell")
  const [touched, setTouched] = useState(false)
  // The section is below the fold, so picking the visitor's tab after mount does not shift the first view.
  useEffect(() => {
    if (!touched && (os === "linux" || os === "mac")) setTab("unix")
  }, [os, touched])

  return (
    <Section id="verify" labelledBy="verify-title" className="scroll-mt-16">
      <div className="grid gap-10 lg:grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)] lg:gap-12">
        <div className="min-w-0">
          <h2 id="verify-title" className="text-h2">
            {download.verifyTitle}
          </h2>
          <p className="mt-4 max-w-2xl text-body text-muted-foreground">{download.verifyText}</p>

          <Tabs
            value={tab}
            onValueChange={(v) => {
              setTab(v)
              setTouched(true)
            }}
            className="mt-6"
          >
            <TabsList aria-label="Shell" className="grid h-auto w-full grid-cols-3 sm:inline-grid sm:w-auto">
              <TabsTrigger value="powershell" className={tabTrigger}>
                PowerShell
              </TabsTrigger>
              <TabsTrigger value="cmd" className={tabTrigger}>
                Command Prompt
              </TabsTrigger>
              <TabsTrigger value="unix" className={tabTrigger}>
                Linux or macOS
              </TabsTrigger>
            </TabsList>
            <TabsContent value="powershell" className="mt-4">
              <CodeBlock wrap code={`Get-FileHash .\\${installerName} -Algorithm SHA256`} copyLabel="Copy PowerShell command" />
            </TabsContent>
            <TabsContent value="cmd" className="mt-4">
              <CodeBlock wrap code={`certutil -hashfile ${installerName} SHA256`} copyLabel="Copy Command Prompt command" />
            </TabsContent>
            <TabsContent value="unix" className="mt-4 space-y-4">
              <CodeBlock wrap caption="Linux" code={`sha256sum ${installerName}`} copyLabel="Copy Linux command" />
              <CodeBlock wrap caption="macOS" code={`shasum -a 256 ${installerName}`} copyLabel="Copy macOS command" />
            </TabsContent>
          </Tabs>
          <p className="mt-4 max-w-2xl text-small text-subtle-foreground">{copy.caseNote}</p>

          {(sidecarSha || sha256sums) && (
            <div className="mt-6 text-small text-muted-foreground">
              <p>{copy.sidecars}</p>
              <ul className="mt-2 flex flex-col gap-2">
                {[sidecarSha, sha256sums].map(
                  (asset) =>
                    asset && (
                      <li key={asset.name}>
                        <a
                          href={asset.url}
                          className="break-all rounded-sm font-mono text-[13px] text-brand-foreground underline-offset-4 hover:underline"
                        >
                          {asset.name}
                        </a>
                      </li>
                    ),
                )}
              </ul>
            </div>
          )}
        </div>

        <div className="flex min-w-0 flex-col gap-6">
          <Card className="rounded-card p-6 shadow-none">
            <div className="flex items-start gap-3">
              <ShieldAlert aria-hidden className="mt-1 size-5 shrink-0 text-muted-foreground" />
              <h3 className="text-h3">{download.smartscreenTitle}</h3>
            </div>
            <p className="mt-3 text-small text-muted-foreground">{download.smartscreenText}</p>
          </Card>

          <Card className="rounded-card p-6 shadow-none">
            <div className="flex items-start gap-3">
              <FileKey aria-hidden className="mt-1 size-5 shrink-0 text-muted-foreground" />
              <h3 className="text-h3">{download.sigTitle}</h3>
            </div>
            <p className="mt-3 text-small text-muted-foreground">{download.sigText}</p>
            <CodeBlock
              className="mt-5"
              caption={download.sigKeyLabel}
              code={RELEASE_PUBLIC_KEY}
              copyLabel="Copy release public key"
              wrap
            />
            <div className="mt-5 flex flex-col items-start gap-3 text-small">
              <TextLink href={blob("docs/UPDATES.md")}>{copy.updaterLink}</TextLink>
              {installerSig && (
                <a
                  href={installerSig.url}
                  className="break-all rounded-sm text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
                >
                  {copy.sigAsset}: <span className="font-mono text-[13px]">{installerSig.name}</span>
                </a>
              )}
            </div>
          </Card>
        </div>
      </div>
    </Section>
  )
}

function SourceSection() {
  return (
    <Section id="source" labelledBy="source-title" className="scroll-mt-16 border-t">
      <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] lg:items-end lg:gap-12">
        <div className="min-w-0">
          <h2 id="source-title" className="text-h2">
            {download.sourceTitle}
          </h2>
          <p className="mt-4 max-w-xl text-body text-muted-foreground">{download.sourceText}</p>
          <div className="mt-6 flex flex-col gap-3 sm:flex-row">
            <Button variant="outline" className="h-11" asChild>
              <a href={repo}>
                <Github aria-hidden />
                {copy.viewOnGithub}
              </a>
            </Button>
            <Button variant="outline" className="h-11" asChild>
              <a href={`${repo}/archive/refs/heads/main.zip`}>
                <Download aria-hidden />
                {copy.downloadZip}
              </a>
            </Button>
          </div>
        </div>
        <CodeBlock wrap caption={copy.cloneCaption} code={CLONE_COMMAND} copyLabel="Copy git clone command" />
      </div>
    </Section>
  )
}

function FilesSection() {
  if (!release || release.assets.length === 0) return null
  return (
    <Section id="files" labelledBy="files-title" className="scroll-mt-16 border-t">
      <h2 id="files-title" className="text-h2">
        {copy.filesTitle}
      </h2>
      <p className="mt-4 max-w-2xl text-body text-muted-foreground">{copy.filesText}</p>
      <Accordion type="single" collapsible className="mt-6 rounded-card border bg-card px-4 md:px-6">
        <AccordionItem value="files" className="border-b-0">
          <AccordionTrigger className="min-h-12 text-small font-medium text-foreground hover:no-underline">
            {copy.showFiles(release.assets.length)}
          </AccordionTrigger>
          <AccordionContent className="border-t pb-2">
            <AssetTable assets={release.assets} />
          </AccordionContent>
        </AccordionItem>
      </Accordion>
    </Section>
  )
}

export function DownloadPage() {
  const os = useDetectedOs()
  return (
    <>
      <PageHeader title={download.title} lead={download.lead} />
      <BetaNotice />
      <div className="container mt-8 grid gap-6 lg:grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)]">
        <WindowsCard os={os} />
        <LinuxCard os={os} />
      </div>
      <VerifySection os={os} />
      <SourceSection />
      <FilesSection />
    </>
  )
}
