import { StrictMode, useEffect, type ComponentType } from "react"
import { destroyMotion, initMotion } from "@/lib/motion"
import { createRoot, hydrateRoot } from "react-dom/client"
import { Layout, type PageId } from "@/components/site/Layout"
import "@/index.css"

/** Hydrates the prerendered page (production) or renders it fresh (dev server). */
function Motion() {
  useEffect(() => {
    initMotion()
    return destroyMotion
  }, [])
  return null
}

export function mount(page: PageId, Page: ComponentType) {
  const root = document.getElementById("root")!
  const app = (
    <StrictMode>
      <Motion />
      <Layout page={page}>
        <Page />
      </Layout>
    </StrictMode>
  )
  if (root.firstElementChild) hydrateRoot(root, app)
  else createRoot(root).render(app)
}
