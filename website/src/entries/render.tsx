// Server entry used only by build/prerender.mjs to write static HTML for each page.
import { StrictMode, type ComponentType } from "react"
import { renderToString } from "react-dom/server"
import { Layout, type PageId } from "@/components/site/Layout"
import { HomePage } from "@/pages/Home"
import { DownloadPage } from "@/pages/Download"
import { DevicesPage } from "@/pages/Devices"
import { DocsPage } from "@/pages/Docs"
import { PrivacyPage } from "@/pages/Privacy"

const pages: Record<PageId, ComponentType> = {
  home: HomePage,
  download: DownloadPage,
  devices: DevicesPage,
  docs: DocsPage,
  privacy: PrivacyPage,
}

export function render(page: PageId) {
  const Page = pages[page]
  return renderToString(
    <StrictMode>
      <Layout page={page}>
        <Page />
      </Layout>
    </StrictMode>,
  )
}
