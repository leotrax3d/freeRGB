import { useEffect, useState } from "react"
import { ArrowRight } from "lucide-react"
import ring from "../../../../assets/brand/ring-dark.svg"
import { Button } from "@/components/ui/button"
import { comingSoon } from "@/content/site"

// public/theme-init.js adds `gate` to <html> before paint when this device has not
// clicked through yet, so the site never flashes behind the overlay. Without JS there
// is no class and the overlay stays hidden: crawlers and no-JS visitors see the site.
const SEEN = "freergb-entered"
const DEVICE = "freergb-device"

function deviceId() {
  try {
    let id = localStorage.getItem(DEVICE)
    if (!id) localStorage.setItem(DEVICE, (id = crypto.randomUUID()))
    return id
  } catch {
    return null
  }
}

/** Counts the click once per device (see supabase/migrations/*_coming_soon_clicks.sql). */
function count() {
  const url = import.meta.env.VITE_SUPABASE_URL as string | undefined
  const key = import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY as string | undefined
  const id = deviceId()
  if (!url || !key || !id) return
  fetch(`${url.replace(/\/$/, "")}/rest/v1/rpc/register_coming_soon_click`, {
    method: "POST",
    keepalive: true,
    headers: { apikey: key, Authorization: `Bearer ${key}`, "Content-Type": "application/json" },
    body: JSON.stringify({ p_device: id }),
  }).catch(() => {
    /* offline or blocked: the visitor still gets in */
  })
}

export function ComingSoon() {
  const [leaving, setLeaving] = useState(false)
  useEffect(() => {
    if (document.documentElement.classList.contains("gate")) document.querySelector<HTMLElement>("[data-gate] a, [data-gate] button")?.focus()
  }, [])
  const enter = () => {
    let first = true
    try {
      first = !localStorage.getItem(SEEN)
      localStorage.setItem(SEEN, "1")
    } catch {
      /* storage blocked: count anyway, the server still de-duplicates */
    }
    if (first) count()
    setLeaving(true)
    window.setTimeout(() => document.documentElement.classList.remove("gate"), 450)
  }
  return (
    <div data-gate="" data-leaving={leaving || undefined} role="dialog" aria-modal="true" aria-labelledby="gate-title" className="fixed inset-0 z-[90] flex-col items-center justify-center gap-8 bg-background px-6 text-center">
      <img src={ring} alt="" width={56} height={56} className="gate-ring size-14 rounded-[14px]" />
      <h1 id="gate-title" className="text-[clamp(44px,10vw,112px)] font-semibold leading-none tracking-[-0.05em]">
        {comingSoon.title}
      </h1>
      <Button size="lg" onClick={enter} className="motion-btn">
        {comingSoon.enter}
        <ArrowRight aria-hidden />
      </Button>
    </div>
  )
}
