import { useEffect, useState } from "react"

export type DetectedOs = "windows" | "linux" | "mac" | "other" | null

/** Reads the visitor's OS after mount. Always null during prerender and first render. */
export function useDetectedOs(): DetectedOs {
  const [os, setOs] = useState<DetectedOs>(null)
  useEffect(() => {
    const nav = navigator as Navigator & { userAgentData?: { platform?: string } }
    const source = `${nav.userAgentData?.platform ?? ""} ${nav.userAgent}`.toLowerCase()
    if (/android|iphone|ipad|cros/.test(source)) setOs("other")
    else if (source.includes("win")) setOs("windows")
    else if (/mac/.test(source)) setOs("mac")
    else if (/linux|x11/.test(source)) setOs("linux")
    else setOs("other")
  }, [])
  return os
}
