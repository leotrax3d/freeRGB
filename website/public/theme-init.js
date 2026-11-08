// Runs before first paint so the page never flashes the wrong theme.
// The choice is stored only in this browser (localStorage key "freergb-theme").
try {
  var stored = localStorage.getItem("freergb-theme")
  var dark = stored ? stored === "dark" : !window.matchMedia("(prefers-color-scheme: light)").matches
  document.documentElement.classList.toggle("dark", dark)
} catch (e) {
  document.documentElement.classList.add("dark")
}
