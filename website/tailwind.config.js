/** @type {import('tailwindcss').Config} */
import animate from "tailwindcss-animate"
// Tokens: docs/website/DESIGN_TOKENS.md (graphite). Values live in src/index.css.
export default {
  darkMode: ["class"],
  content: ["./index.html", "./*/index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    container: { center: true, padding: { DEFAULT: "1rem", sm: "1.5rem", lg: "2rem" }, screens: { "2xl": "1200px" } },
    extend: {
      fontFamily: {
        sans: ['"Inter Variable"', '"Inter Fallback"', "ui-sans-serif", "system-ui", "sans-serif"],
        mono: ["ui-monospace", '"Cascadia Code"', '"SF Mono"', "Consolas", "monospace"],
      },
      fontSize: {
        caption: ["12px", { lineHeight: "16px", letterSpacing: "0.01em", fontWeight: "500" }],
        small: ["14px", "22px"],
        body: ["16px", "26px"],
        lead: ["18px", "30px"],
        h3: ["20px", { lineHeight: "28px", letterSpacing: "-0.015em", fontWeight: "600" }],
        h2: ["clamp(26px, 3vw, 36px)", { lineHeight: "1.15", letterSpacing: "-0.03em", fontWeight: "600" }],
        h1: ["clamp(32px, 4vw, 48px)", { lineHeight: "1.1", letterSpacing: "-0.035em", fontWeight: "600" }],
        display: ["clamp(40px, 6vw, 68px)", { lineHeight: "1.04", letterSpacing: "-0.045em", fontWeight: "600" }],
      },
      colors: {
        border: "hsl(var(--border))",
        "border-strong": "hsl(var(--border-strong))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: { DEFAULT: "hsl(var(--primary))", foreground: "hsl(var(--primary-foreground))" },
        secondary: { DEFAULT: "hsl(var(--secondary))", foreground: "hsl(var(--secondary-foreground))" },
        muted: { DEFAULT: "hsl(var(--muted))", foreground: "hsl(var(--muted-foreground))" },
        subtle: { foreground: "hsl(var(--subtle-foreground))" },
        accent: { DEFAULT: "hsl(var(--accent))", foreground: "hsl(var(--accent-foreground))" },
        destructive: { DEFAULT: "hsl(var(--destructive))", foreground: "hsl(var(--destructive-foreground))" },
        warning: { DEFAULT: "hsl(var(--warning))" },
        brand: { DEFAULT: "hsl(var(--brand))", foreground: "hsl(var(--brand-foreground))" },
        card: { DEFAULT: "hsl(var(--card))", foreground: "hsl(var(--card-foreground))" },
        popover: { DEFAULT: "hsl(var(--popover))", foreground: "hsl(var(--popover-foreground))" },
      },
      borderRadius: {
        panel: "20px",
        card: "14px",
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "6px",
      },
      boxShadow: {
        popover: "0 8px 24px rgb(0 0 0 / .5), 0 0 0 1px rgb(255 255 255 / .08)",
        frame: "0 40px 80px -24px rgb(0 0 0 / var(--frame-shadow))",
      },
      transitionTimingFunction: { out: "cubic-bezier(0.2, 0, 0, 1)" },
      transitionDuration: { fast: "120ms", base: "180ms", slow: "240ms" },
      keyframes: {
        "accordion-down": { from: { height: "0" }, to: { height: "var(--radix-accordion-content-height)" } },
        "accordion-up": { from: { height: "var(--radix-accordion-content-height)" }, to: { height: "0" } },
        "glow-hue": { to: { filter: "blur(var(--glow-blur)) hue-rotate(360deg)" } },
      },
      animation: {
        "accordion-down": "accordion-down 180ms cubic-bezier(0.2, 0, 0, 1)",
        "accordion-up": "accordion-up 180ms cubic-bezier(0.2, 0, 0, 1)",
        "glow-hue": "glow-hue 40s linear infinite",
      },
    },
  },
  plugins: [animate],
}
