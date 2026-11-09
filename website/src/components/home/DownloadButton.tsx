import { useEffect, useState } from "react"
import { Download, Terminal } from "lucide-react"
import { Button } from "@/components/ui/button"
import { hero } from "@/content/site"
import { href } from "@/lib/paths"
import { cn } from "@/lib/utils"

/**
 * OS-aware primary download button. The server render always shows the Windows label;
 * after mount a Linux user agent swaps label and target. min-w keeps the width stable.
 */
export function DownloadButton({ className }: { className?: string }) {
  const [linux, setLinux] = useState(false)
  useEffect(() => {
    const ua = navigator.userAgent
    setLinux(/Linux|X11/i.test(ua) && !/Android|CrOS/i.test(ua))
  }, [])
  return (
    <Button size="lg" className={cn("motion-btn min-w-[15rem]", className)} asChild>
      <a href={href(linux ? "download/#linux" : "download/")}>
        {linux ? <Terminal aria-hidden /> : <Download aria-hidden />}
        {linux ? hero.primaryLinux : hero.primaryWindows}
      </a>
    </Button>
  )
}
