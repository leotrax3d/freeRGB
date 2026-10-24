//! Transient toasts: short confirmations for explicit user actions.
//! Decorative feedback only — errors that matter also stay in the status bar
//! and on their own page.

use eframe::egui;
use std::time::{Duration, Instant};

const LIFETIME: Duration = Duration::from_millis(2600);
const MAX: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A completed action (saved, applied, deleted).
    Success,
    /// A failed action that the daemon could not execute.
    ///
    /// Kept next to [`Toasts::error`]; the Classic GUI surfaces daemon
    /// failures in the status strip, so no caller needs this yet.
    #[allow(dead_code)]
    Error,
}

#[derive(Clone)]
struct Toast {
    kind: Kind,
    text: String,
    born: Instant,
}

#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    pub fn success(&mut self, text: impl Into<String>) {
        self.push(Toast {
            kind: Kind::Success,
            text: text.into(),
            born: Instant::now(),
        });
    }

    #[allow(dead_code)]
    pub fn error(&mut self, text: impl Into<String>) {
        self.push(Toast {
            kind: Kind::Error,
            text: text.into(),
            born: Instant::now(),
        });
    }

    fn push(&mut self, toast: Toast) {
        self.items.push(toast);
        while self.items.len() > MAX {
            self.items.remove(0);
        }
    }

    /// Drops expired toasts and draws the remaining stack at bottom-right.
    pub fn show(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.items
            .retain(|toast| now.duration_since(toast.born) < LIFETIME);
        let expired = self.items.clone();
        egui::Area::new(egui::Id::new("toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -18.0])
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                for toast in expired {
                    let (fill, accent, mark) = match toast.kind {
                        Kind::Success => {
                            (egui::Color32::from_rgb(26, 42, 32), crate::theme::GOOD, "✓")
                        }
                        Kind::Error => {
                            (egui::Color32::from_rgb(46, 30, 30), crate::theme::BAD, "✕")
                        }
                    };
                    egui::Frame::none()
                        .fill(fill)
                        .stroke(egui::Stroke::new(1.0_f32, accent.gamma_multiply(0.8)))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(accent, mark);
                                ui.label(&toast.text);
                            });
                        });
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_newest_toasts_are_kept() {
        let mut toasts = Toasts::default();
        for index in 0..8 {
            toasts.success(format!("number {index}"));
        }
        assert_eq!(toasts.items.len(), MAX);
        assert_eq!(toasts.items[0].text, "number 4");
    }
}
