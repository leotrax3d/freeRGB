import { clsx, type ClassValue } from "clsx"
import { extendTailwindMerge } from "tailwind-merge"

// Teach tailwind-merge the custom font sizes from tailwind.config.js. Without this it
// reads "text-caption" as a colour and drops it when a colour class follows.
const twMerge = extendTailwindMerge({
  extend: {
    classGroups: {
      "font-size": [{ text: ["caption", "small", "body", "lead", "h3", "h2", "h1", "display"] }],
    },
  },
})

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
