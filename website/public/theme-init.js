// Runs before first paint so the page never flashes the wrong theme.
// The choice is stored only in this browser (localStorage key "freergb-theme").
try {
  var stored = localStorage.getItem("freergb-theme")
  var dark = stored ? stored === "dark" : !window.matchMedia("(prefers-color-scheme: light)").matches
  document.documentElement.classList.toggle("dark", dark)
  // Scroll motion: hide reveal targets until src/lib/motion.ts takes over (JS on only).
  if (!window.matchMedia("(prefers-reduced-motion: reduce)").matches) document.documentElement.classList.add("motion-pending")
} catch (e) {
  document.documentElement.classList.add("dark")
}
