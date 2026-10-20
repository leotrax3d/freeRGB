import { create } from "zustand";
import type { ViewId } from "./views";

const KEY = "freergb.view";

function initial(): ViewId {
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
