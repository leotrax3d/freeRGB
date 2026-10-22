import { create } from "zustand";
import type { ViewId } from "./views";

const VIEW_IDS: readonly ViewId[] = ["lighting", "devices", "layout", "profiles", "cooling", "macros", "monitoring", "settings"];

const KEY = "freergb.view";

function initial(): ViewId {
  // "#devices" opens a view directly (handy for screenshots and docs).
  const hash = location.hash.slice(1);
  if (VIEW_IDS.includes(hash as ViewId)) return hash as ViewId;
  try {
    return (localStorage.getItem(KEY) as ViewId | null) ?? "lighting";
  } catch {
    return "lighting";
  }
}

export const useNavigation = create<{ view: ViewId; go: (view: ViewId) => void }>((set) => ({
  view: initial(),
  go: (view) => {
    try {
      localStorage.setItem(KEY, view);
    } catch {
      // Remembering the view is a convenience only.
    }
    set({ view });
  },
}));
