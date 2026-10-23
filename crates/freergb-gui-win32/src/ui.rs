//! Immediate-mode widget layer: every widget draws and hit-tests in one call,
//! so page code reads like the egui Classic GUI. Layout works in logical
//! pixels (96 DPI base) and scales with the window DPI. The two skins from
//! [`crate::theme`] differ only in palette values and gradients.

use crate::ffi::*;
use crate::theme::{self, Font, Fonts, Palette};
use std::collections::HashMap;

/// Logical rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    pub fn with_margin(&self, m: f32) -> Self {
        Self::new(self.x + m, self.y + m, self.w - 2.0 * m, self.h - 2.0 * m)
    }
}

/// Raw input state, filled by the window procedure in logical pixels.
#[derive(Default)]
pub struct Input {
    /// Mouse position in logical pixels; `valid` is false before the first move.
    pub mouse: (f32, f32),
    pub valid: bool,
    pub down: bool,
    pub right_down: bool,
    pub just_down: bool,
    pub just_up: bool,
    /// Left double-click (requires `CS_DBLCLKS`).
    pub double_click: bool,
    /// Right button released this frame.
    pub right_released: bool,
    /// Accumulated wheel delta (positive = up).
    pub wheel: i32,
    /// UTF-16 units from `WM_CHAR` this frame.
    pub unicode: Vec<u16>,
    /// VK codes from `WM_KEYDOWN` this frame.
    pub keys: Vec<u32>,
    /// Ctrl+V pressed this frame.
    pub paste: bool,
}

impl Input {
    pub fn begin_frame(&mut self) {
        self.just_down = false;
        self.just_up = false;
        self.double_click = false;
        self.right_released = false;
        self.wheel = 0;
        self.unicode.clear();
        self.keys.clear();
        self.paste = false;
    }
}

/// Interaction state that persists between frames.
#[derive(Default)]
pub struct Widgets {
    /// Widget with keyboard focus (text inputs).
    pub focus: Option<u64>,
    /// Widget that owns the current mouse press.
    pub press: Option<u64>,
    /// Open combobox popup.
    pub popup: Option<Popup>,
    /// Selection made in the open popup: (popup id, item index).
    pub popup_choice: Option<(u64, usize)>,
    /// Text caret of the focused input in (line, column) chars.
    pub caret: (usize, usize),
    /// Vertical scroll of focused multiline inputs.
    pub text_scroll: HashMap<u64, f32>,
}

/// Combobox dropdown state.
pub struct Popup {
    pub id: u64,
    /// Anchor in logical coordinates (the combo face).
    pub anchor: Rect,
    /// Scroll offset in items.
    pub offset: i32,
    pub items: Vec<String>,
    /// Last drawn list rect for hit-testing (logical).
    pub rect: Rect,
}

/// Stable widget ids: FNV-1a over the page and label.
pub fn id(parts: &[&str]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// One frame of drawing + interaction.
pub struct Ui<'a> {
    pub dc: HDC,
    pub scale: f32,
    /// Client size in logical pixels.
    pub width: f32,
    pub height: f32,
    pub input: &'a Input,
    pub widgets: &'a mut Widgets,
    pub palette: Palette,
    pub fonts: Fonts,
    pub brushes: &'a mut theme::Brushes,
    /// Layout cursor in logical pixels.
    pub cursor: (f32, f32),
    /// Left/right bounds of the current layout column.
    pub bounds: (f32, f32),
}

fn lerp_color(a: COLORREF, b: COLORREF, t: f32) -> COLORREF {
    let channel = |shift: u32| {
        let (x, y) = ((a >> shift) & 0xFF, (b >> shift) & 0xFF);
        (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u32
    };
    channel(0) | channel(8) << 8 | channel(16) << 16
}

impl Ui<'_> {
    pub fn new(
        dc: HDC,
        scale: f32,
        width: f32,
        height: f32,
        palette: Palette,
        fonts: Fonts,
        input: &Input,
        widgets: &mut Widgets,
        brushes: &mut theme::Brushes,
    ) -> Ui<'_> {
        Ui {
            dc,
            scale,
            width,
            height,
            input,
            widgets,
            palette,
            fonts,
            brushes,
            cursor: (0.0, 0.0),
            bounds: (0.0, width),
        }
    }

    pub fn s(&self, v: f32) -> i32 {
        (v * self.scale).round() as i32
    }

    pub fn px(&self, rect: &Rect) -> RECT {
        RECT {
            left: self.s(rect.x),
            top: self.s(rect.y),
            right: self.s(rect.right()),
            bottom: self.s(rect.bottom()),
        }
    }

    // ---- layout ----

    /// Claims the next `w × h` slot at the cursor and advances vertically.
    pub fn item(&mut self, w: f32, h: f32) -> Rect {
        let w = w.clamp(0.0, (self.bounds.1 - self.cursor.0).max(0.0));
        let rect = Rect::new(self.cursor.0, self.cursor.1, w, h);
        self.cursor.1 += h + 8.0;
        rect
    }

    pub fn advance(&mut self, h: f32) {
        self.cursor.1 += h;
    }

    pub fn column_width(&self) -> f32 {
        (self.bounds.1 - self.bounds.0).max(0.0)
    }

    pub fn set_cursor(&mut self, x: f32, y: f32) {
        self.cursor = (x, y);
        self.bounds = (x, self.bounds.1.max(x + 1.0));
    }

    pub fn set_bounds(&mut self, left: f32, right: f32) {
        self.bounds = (left, right);
        self.cursor.0 = left;
    }

    // ---- interaction helpers ----

    pub fn contains(&self, rect: &Rect) -> bool {
        self.input.valid && rect.contains(self.input.mouse.0, self.input.mouse.1)
    }

    /// Claims the press for `id` when a press starts inside `rect`.
    pub fn claim_press(&mut self, id: u64, rect: &Rect) {
        if self.input.just_down && self.contains(rect) && self.widgets.press.is_none() {
            self.widgets.press = Some(id);
        }
    }

    pub fn hovered_or_pressed(&self, id: u64, rect: &Rect) -> (bool, bool) {
        let hovered = self.contains(rect);
        let pressed = self.widgets.press == Some(id);
        (hovered, pressed && hovered)
    }

    /// True when the press that `id` owns is released inside `rect`.
    pub fn click(&mut self, id: u64, rect: &Rect) -> bool {
        self.input.just_up
            && self.widgets.press == Some(id)
            && self.contains(rect)
    }

    // ---- primitives ----

    pub fn fill(&mut self, rect: &Rect, color: COLORREF) {
        if rect.w <= 0.0 || rect.h <= 0.0 {
            return;
        }
        let rect_px = self.px(rect);
        let brush = self.brushes.get(color);
        unsafe { FillRect(self.dc, &rect_px, brush) };
    }

    /// Vertical gradient drawn as interpolated 1-px bands (the classic GDI
    /// way); gradients are what gives the Aero skin its gloss.
    pub fn gradient_v(&mut self, rect: &Rect, top: COLORREF, bottom: COLORREF) {
        if rect.w <= 0.0 || rect.h <= 0.0 {
            return;
        }
        let rect_px = self.px(rect);
        let bands = (rect_px.bottom - rect_px.top).max(1);
        for line in rect_px.top..rect_px.bottom {
            let t = f32::from(line - rect_px.top) / f32::from(bands);
            let band = RECT {
                left: rect_px.left,
                top: line,
                right: rect_px.right,
                bottom: line + 1,
            };
            let brush = unsafe { CreateSolidBrush(lerp_color(top, bottom, t)) };
            unsafe { FillRect(self.dc, &band, brush) };
            unsafe { DeleteObject(brush as _) };
        }
    }

    /// Filled rounded box with a 1-px border. `fill` is a (top, bottom)
    /// gradient when `Some`.
    pub fn box_(
        &mut self,
        rect: &Rect,
        fill: Option<(COLORREF, COLORREF)>,
        border: Option<COLORREF>,
    ) {
        if rect.w <= 0.0 || rect.h <= 0.0 {
            return;
        }
        let radius = self.palette.radius;
        let Some((top, bottom)) = fill else {
            return;
        };
        if top == bottom {
            let rect_px = self.px(rect);
            let brush = self.brushes.get(top);
            let pen = unsafe { CreatePen(PS_SOLID, 1, border.unwrap_or(top)) };
            let old_pen = unsafe { SelectObject(self.dc, pen as _) };
            let old_brush = unsafe { SelectObject(self.dc, brush as _) };
            unsafe {
                RoundRect(
                    self.dc,
                    rect_px.left,
                    rect_px.top,
                    rect_px.right,
                    rect_px.bottom,
                    self.s(radius * 2.0),
                    self.s(radius * 2.0),
                )
            };
            unsafe { SelectObject(self.dc, old_brush) };
            unsafe { SelectObject(self.dc, old_pen) };
            unsafe { DeleteObject(pen as _) };
        } else {
            // Gradient first (square), then the border on top so corners
            // still read as rounded.
            self.gradient_v(rect, top, bottom);
            if let Some(color) = border {
                let rect_px = self.px(rect);
                let pen = unsafe { CreatePen(PS_SOLID, 1, color) };
                let old = unsafe { SelectObject(self.dc, pen as _) };
                let null_brush = unsafe { GetStockObject(NULL_BRUSH) };
                let old_brush = unsafe { SelectObject(self.dc, null_brush) };
                unsafe {
                    RoundRect(
                        self.dc,
                        rect_px.left,
                        rect_px.top,
                        rect_px.right,
                        rect_px.bottom,
                        self.s(radius * 2.0),
                        self.s(radius * 2.0),
                    )
                };
                unsafe { SelectObject(self.dc, old_brush) };
                unsafe { SelectObject(self.dc, old) };
                unsafe { DeleteObject(pen as _) };
            }
        }
    }

    pub fn border(&mut self, rect: &Rect, color: COLORREF) {
        let rect_px = self.px(rect);
        let pen = unsafe { CreatePen(PS_SOLID, 1, color) };
        let old = unsafe { SelectObject(self.dc, pen as _) };
        let null_brush = unsafe { GetStockObject(NULL_BRUSH) };
        let old_brush = unsafe { SelectObject(self.dc, null_brush) };
        let rect_px = RECT {
            right: rect_px.right.saturating_sub(1),
            bottom: rect_px.bottom.saturating_sub(1),
            ..rect_px
        };
        unsafe {
            RoundRect(
                self.dc,
                rect_px.left,
                rect_px.top,
                rect_px.right,
                rect_px.bottom,
                self.s(self.palette.radius * 2.0),
                self.s(self.palette.radius * 2.0),
            )
        };
        unsafe { SelectObject(self.dc, old_brush) };
        unsafe { SelectObject(self.dc, old) };
        unsafe { DeleteObject(pen as _) };
    }

    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: COLORREF, width: i32) {
        let pen = unsafe { CreatePen(PS_SOLID, width, color) };
        let old_pen = unsafe { SelectObject(self.dc, pen as _) };
        let old_brush = unsafe { SelectObject(self.dc, unsafe { GetStockObject(NULL_BRUSH) } as _) };
        unsafe {
            MoveToEx(self.dc, self.s(x1), self.s(y1), std::ptr::null_mut());
            LineTo(self.dc, self.s(x2), self.s(y2));
        };
        unsafe { SelectObject(self.dc, old_brush) };
        unsafe { SelectObject(self.dc, old_pen) };
        unsafe { DeleteObject(pen as _) };
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, fill: COLORREF, border: COLORREF) {
        let (left, top) = (self.s(cx - r), self.s(cy - r));
        let (right, bottom) = (self.s(cx + r), self.s(cy + r));
        let brush = self.brushes.get(fill);
        let pen = unsafe { CreatePen(PS_SOLID, 1, border) };
        let old_pen = unsafe { SelectObject(self.dc, pen as _) };
        let old_brush = unsafe { SelectObject(self.dc, brush as _) };
        unsafe { Ellipse(self.dc, left, top, right, bottom) };
        unsafe { SelectObject(self.dc, old_brush) };
        unsafe { SelectObject(self.dc, old_pen) };
        unsafe { DeleteObject(pen as _) };
    }

    pub fn font_handle(&self, font: Font) -> HFONT {
        match font {
            Font::Body => self.fonts.body,
            Font::Small => self.fonts.small,
            Font::Strong => self.fonts.strong,
            Font::Heading => self.fonts.heading,
            Font::Mono => self.fonts.mono,
        }
    }

    pub fn measure(&self, text: &str, font: Font) -> (f32, f32) {
        unsafe {
            let old = SelectObject(self.dc, self.font_handle(font) as _);
            let wide: Vec<u16> = text.encode_utf16().collect();
            let mut size = SIZE { cx: 0, cy: 0 };
            if !wide.is_empty() {
                GetTextExtentPoint32W(self.dc, wide.as_ptr(), wide.len() as i32, &mut size);
            }
            SelectObject(self.dc, old);
            (
                f32::from(size.cx) / self.scale,
                f32::from(size.cy) / self.scale,
            )
        }
    }

    /// Greedy word wrap for label text.
    pub fn wrap(&self, text: &str, font: Font, max_w: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            if paragraph.is_empty() {
                lines.push(String::new());
                continue;
            }
            let mut current = String::new();
            for word in paragraph.split(' ') {
                let candidate = if current.is_empty() {
                    word.to_owned()
                } else {
                    format!("{current} {word}")
                };
                if !current.is_empty() && self.measure(&candidate, font).0 > max_w {
                    lines.push(std::mem::take(&mut current));
                    current = word.to_owned();
                } else {
                    current = candidate;
                }
            }
            lines.push(current);
        }
        lines
    }

    pub fn text(&mut self, rect: &Rect, text: &str, font: Font, color: COLORREF, align: u32) {
        if text.is_empty() {
            return;
        }
        let rect_px = self.px(rect);
        unsafe {
            let old = SelectObject(self.dc, self.font_handle(font) as _);
            let wide: Vec<u16> = text.encode_utf16().collect();
            SetTextColor(self.dc, color);
            DrawTextW(
                self.dc,
                wide.as_ptr(),
                wide.len() as i32,
                &mut { rect_px },
                DT_NOPREFIX | align,
            );
            SelectObject(self.dc, old);
        }
    }

    /// Clips everything drawn inside the closure to `rect`.
    pub fn clip(&mut self, rect: &Rect, draw: impl FnOnce(&mut Self)) {
        let saved = unsafe { SaveDC(self.dc) };
        let rect_px = self.px(rect);
        unsafe { IntersectClipRect(self.dc, rect_px.left, rect_px.top, rect_px.right, rect_px.bottom) };
        draw(self);
        unsafe { RestoreDC(self.dc, saved) };
    }

    // ---- widgets ----

    pub fn label(&mut self, text: &str) {
        self.label_color(text, Font::Body, self.palette.text);
    }

    pub fn label_color(&mut self, text: &str, font: Font, color: COLORREF) {
        let lines = self.wrap(text, font, self.column_width());
        let line_h = self.fonts.line;
        for line in lines {
            let rect = self.item(self.column_width(), line_h);
            self.text(&rect, &line, font, color, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        }
    }

    pub fn strong(&mut self, text: &str) {
        self.label_color(text, Font::Strong, self.palette.text);
    }

    pub fn heading(&mut self, text: &str) {
        self.label_color(text, Font::Heading, self.palette.text);
    }

    pub fn hint(&mut self, text: &str) {
        self.label_color(text, Font::Small, self.palette.text_dim);
    }

    pub fn separator(&mut self) {
        let rect = self.item(self.column_width(), 1.0);
        self.fill(&rect, self.palette.border);
    }

    pub fn button(&mut self, id: u64, text: &str, primary: bool) -> bool {
        let width = self.column_width().min(160.0);
        let rect = self.item(width, 30.0);
        self.button_at(id, &rect, text, primary)
    }

    pub fn button_fixed(&mut self, id: u64, text: &str, w: f32, primary: bool) -> bool {
        let rect = self.item(w, 30.0);
        self.button_at(id, &rect, text, primary)
    }

    pub fn button_at(&mut self, id: u64, rect: &Rect, text: &str, primary: bool) -> bool {
        self.claim_press(id, rect);
        let (hovered, pressed) = self.hovered_or_pressed(id, rect);
        let button = &self.palette.button;
        let (top, bottom, border, color) = if primary {
            (
                button.primary_top,
                button.primary_bottom,
                button.primary_border,
                self.palette.text_inv,
            )
        } else if pressed {
            (
                button.press_top,
                button.press_bottom,
                button.press_border,
                self.palette.text,
            )
        } else if hovered {
            (
                button.hover_top,
                button.hover_bottom,
                button.hover_border,
                self.palette.text,
            )
        } else {
            (button.top, button.bottom, button.border, self.palette.text)
        };
        self.box_(rect, Some((top, bottom)), Some(border));
        // Aero gloss: a light band across the top third.
        if self.palette.chrome_gradient {
            let gloss = rect.with_margin(2.0);
            let gloss = Rect::new(gloss.x, gloss.y, gloss.w, gloss.h * 0.4);
            self.gradient_v(&gloss, lerp_color(top, 0xFFFFFF, 0.35), lerp_color(top, 0xFFFFFF, 0.05));
        }
        self.text(rect, text, Font::Body, color, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        self.click(id, rect)
    }

    /// Checkbox row; returns `true` when the value changed.
    pub fn checkbox(&mut self, id: u64, checked: &mut bool, text: &str) -> bool {
        let rect = self.item(self.column_width(), 22.0);
        self.claim_press(id, &rect);
        let clicked = self.click(id, &rect);
        if clicked {
            *checked = !*checked;
        }
        let box_rect = Rect::new(rect.x, rect.y + (rect.h - 15.0) / 2.0, 15.0, 15.0);
        self.box_(
            &box_rect,
            Some((
                if *checked { self.palette.accent } else { self.palette.field },
                if *checked { self.palette.accent } else { self.palette.field },
            )),
            Some(if *checked { self.palette.accent } else { self.palette.field_border }),
        );
        if *checked {
            self.text(&box_rect, "✓", Font::Small, self.palette.text_inv, DT_CENTER | DT_VCENTER);
        }
        let text_rect = Rect::new(
            box_rect.right() + 6.0,
            rect.y,
            (rect.w - box_rect.w - 6.0).max(0.0),
            rect.h,
        );
        self.text(&text_rect, text, Font::Body, self.palette.text, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        clicked
    }

    /// Label + value row above a track. Returns `true` when the value changed.
    pub fn slider(
        &mut self,
        id: u64,
        value: &mut f32,
        min: f32,
        max: f32,
        text: &str,
        display: &str,
    ) -> bool {
        let outer = self.item(self.column_width(), 44.0);
        let track = Rect::new(outer.x + 2.0, outer.y + 24.0, outer.w - 4.0, 18.0);
        // Pressing the track immediately jumps and claims the drag.
        if self.input.just_down && self.contains(&track) && self.widgets.press.is_none() {
            self.widgets.press = Some(id);
        }
        let mut changed = false;
        if self.widgets.press == Some(id) {
            let t = ((self.input.mouse.0 - track.x) / track.w).clamp(0.0, 1.0);
            let next = min + t * (max - min);
            if self.input.valid && (next - *value).abs() > f32::EPSILON {
                *value = next;
                changed = true;
            }
        }
        self.text(
            &Rect::new(outer.x, outer.y, outer.w - 90.0, 20.0),
            text,
            Font::Body,
            self.palette.text,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        self.text(
            &Rect::new(outer.x + outer.w - 90.0, outer.y, 90.0, 20.0),
            display,
            Font::Body,
            self.palette.text_dim,
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE,
        );
        let center = track.y + track.h / 2.0;
        let groove = Rect::new(track.x, center - 3.0, track.w, 6.0);
        self.box_(&groove, Some((self.palette.track, self.palette.track)), None);
        let t = ((value - min) / (max - min).max(f32::EPSILON)).clamp(0.0, 1.0);
        let fill_w = track.w * t;
        let fill_rect = Rect::new(track.x, center - 3.0, fill_w, 6.0);
        if t > 0.0 {
            self.box_(
                &fill_rect,
                Some((self.palette.track_fill, self.palette.accent_dim)),
                None,
            );
        }
        self.circle(
            track.x + fill_w,
            center,
            7.0,
            self.palette.field,
            self.palette.accent,
        );
        changed
    }

    /// Single-line text input. Returns `true` when the value changed.
    pub fn text_line(&mut self, id: u64, value: &mut String, hint: &str) -> bool {
        let rect = self.item(self.column_width().min(360.0), 28.0);
        self.text_input_at(id, &rect, value, false, Some(hint))
    }

    /// Single-line text input with an explicit width (point tables).
    pub fn text_line_fixed(&mut self, id: u64, value: &mut String, w: f32, hint: &str) -> bool {
        let rect = self.item(w, 26.0);
        self.text_input_at(id, &rect, value, false, Some(hint))
    }

    /// Multiline monospace editor (used for macro JSON). Returns `true` when
    /// the value changed.
    pub fn text_multiline(&mut self, id: u64, value: &mut String, height: f32) -> bool {
        let rect = self.item(self.column_width(), height);
        self.text_input_at(id, &rect, value, true, None)
    }

    fn text_input_at(&mut self, id: u64, rect: &Rect, value: &mut String, multiline: bool, hint: Option<&str>) -> bool {
        if self.input.just_down && self.contains(rect) && self.widgets.press.is_none() {
            self.widgets.press = Some(id);
            if self.widgets.focus != Some(id) {
                self.widgets.focus = Some(id);
                self.widgets.caret = if multiline { (0, 0) } else { (0, value.chars().count()) };
            } else {
                self.widgets.caret = edit::caret_at_point(self, rect, value, multiline);
            }
        }
        let focused = self.widgets.focus == Some(id);
        let mut changed = false;
        if focused {
            changed = edit::apply(self, value, multiline);
        }
        self.box_(
            rect,
            Some((self.palette.field, self.palette.field)),
            Some(if focused { self.palette.focus_border } else { self.palette.field_border }),
        );
        let inner = rect.with_margin(6.0);
        let lines = self.wrap_visible(value, multiline, &inner);
        let scroll = self.widgets.text_scroll.get(&id).copied().unwrap_or(0.0);
        let (caret_line, caret_col) = self.widgets.caret;
        let blink = (unsafe { GetTickCount64() } / 530) % 2 == 0;
        let mut y = inner.y - scroll;
        for (index, line) in lines.iter().enumerate() {
            if focused && blink && index == caret_line {
                let prefix: String = line.chars().take(caret_col).collect();
                let x = inner.x + self.measure(&prefix, Font::Mono).0;
                self.line(x, y, x, y + self.fonts.line, self.palette.text, 1);
            }
            self.text(
                &Rect::new(inner.x, y, inner.w, self.fonts.line),
                line,
                Font::Mono,
                self.palette.text,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += self.fonts.line;
            if y > inner.bottom() {
                break;
            }
        }
        if value.is_empty() {
            if let Some(hint) = hint {
                self.text(
                    &Rect::new(inner.x, inner.y, inner.w, self.fonts.line),
                    hint,
                    Font::Mono,
                    self.palette.text_dim,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                );
            }
        }
        // Multiline wheel scrolling while hovered.
        if multiline && self.contains(rect) && self.input.wheel != 0 {
            let max = (value.lines().count() as f32 * self.fonts.line - inner.h).max(0.0);
            let entry = self.widgets.text_scroll.entry(id).or_insert(0.0);
            *entry = (*entry - self.input.wheel as f32 * 3.0 * self.fonts.line).clamp(0.0, max);
        }
        changed
    }

    fn wrap_visible<'t>(&self, value: &'t str, multiline: bool, inner: &Rect) -> Vec<&'t str> {
        if multiline {
            let max = (inner.h / self.fonts.line).floor() as usize + 1;
            value.lines().take(max.max(1)).collect()
        } else {
            vec![value]
        }
    }

    /// Combobox face with dropdown popup. Returns the selected item index when
    /// a selection was made this frame.
    pub fn combo(
        &mut self,
        id: u64,
        items: &[String],
        selected: Option<usize>,
        placeholder: &str,
    ) -> Option<usize> {
        let rect = self.item(self.column_width().min(360.0), 28.0);
        self.claim_press(id, &rect);
        if self.input.just_down && self.contains(&rect) {
            if self.widgets.popup.as_ref().is_some_and(|p| p.id == id) {
                self.widgets.popup = None;
            } else {
                self.widgets.popup = Some(Popup {
                    id,
                    anchor: rect,
                    offset: 0,
                    items: items.to_vec(),
                    rect: Rect::default(),
                });
            }
        }
        let mut choice = None;
        if let Some((popup_id, index)) = self.widgets.popup_choice.take() {
            if popup_id == id {
                choice = Some(index);
                self.widgets.popup = None;
            }
        }
        self.box_(
            &rect,
            Some((self.palette.field, self.palette.field)),
            Some(self.palette.field_border),
        );
        let text = selected
            .and_then(|index| items.get(index))
            .map(String::as_str)
            .unwrap_or(placeholder);
        self.text(
            &Rect::new(rect.x + 7.0, rect.y, rect.w - 26.0, rect.h),
            text,
            Font::Body,
            if selected.is_some() { self.palette.text } else { self.palette.text_dim },
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        self.text(
            &Rect::new(rect.right() - 22.0, rect.y, 22.0, rect.h),
            "▾",
            Font::Body,
            self.palette.text_dim,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        choice
    }

    /// Draws the open combobox popup on top of everything and handles its
    /// item selection. Call once at the end of the frame, outside clips.
    pub fn draw_popup(&mut self) {
        let Some(popup) = self.widgets.popup.take() else {
            // A click anywhere else still consumes the "press" and unfocuses.
            return;
        };
        if popup.items.is_empty() {
            return;
        }
        let item_h = 26.0;
        let visible = popup.items.len().min(8) as i32;
        let list_h = f32::from(visible) * item_h;
        let below = popup.anchor.bottom() + 2.0;
        let (list_y, _up) = if below + list_h <= self.height - 4.0 {
            (below, false)
        } else {
            (popup.anchor.y - list_h - 2.0, true)
        };
        let list = Rect::new(popup.anchor.x, list_y, popup.anchor.w, list_h);
        // Outside click closes; wheel scrolls the list.
        if self.input.just_down && !self.contains(&list) {
            self.widgets.popup_choice = None;
            return;
        }
        let mut offset = popup
            .offset
            .clamp(0, (popup.items.len() as i32 - visible).max(0));
        if self.contains(&list) && self.input.wheel != 0 {
            offset = (offset - self.input.wheel).clamp(0, (popup.items.len() as i32 - visible).max(0));
        }
        self.box_(&list, Some((self.palette.field, self.palette.field)), Some(self.palette.border));
        let mut choice = None;
        for row in 0..visible {
            let index = offset + row;
            let Some(item) = popup.items.get(index as usize) else {
                break;
            };
            let rect = Rect::new(list.x + 1.0, list.y + f32::from(row) * item_h, list.w - 2.0, item_h);
            let hovered = self.contains(&rect);
            if hovered {
                self.fill(&rect, self.palette.list_hover);
            }
            if self.input.just_down && hovered {
                choice = Some(index as usize);
            }
            self.text(
                &rect,
                item,
                Font::Body,
                self.palette.text,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
        }
        if let Some(index) = choice {
            self.widgets.popup_choice = Some((popup.id, index));
        } else {
            self.widgets.popup = Some(Popup { offset, rect: list, ..popup });
        }
    }
}

/// Pure text-editing helpers (unit tested without Windows).
pub mod edit {
    use super::Ui;

    /// Byte offset of a (line, column) char caret.
    pub fn offset(value: &str, line: usize, column: usize) -> usize {
        let mut current_line = 0;
        let mut start = 0;
        for (index, ch) in value.char_indices() {
            if current_line == line {
                if column == 0 {
                    return index;
                }
                let chars = value[start..].lines().next().unwrap_or("").chars().count();
                if column >= chars {
                    return start + value[start..].lines().next().unwrap_or("").len();
                }
                let mut consumed = 0;
                for (i, c) in value[start..].char_indices() {
                    if consumed == column {
                        return start + i;
                    }
                    if c == '\n' {
                        return start + i;
                    }
                    consumed += 1;
                }
                return start + value[start..].lines().next().unwrap_or("").len();
            }
            if ch == '\n' {
                current_line += 1;
                start = index + 1;
            }
        }
        if line == current_line {
            return start;
        }
        value.len()
    }

    /// Caret (line, column) for a byte offset.
    pub fn caret(value: &str, offset: usize) -> (usize, usize) {
        let mut line = 0;
        let mut column = 0;
        for (index, ch) in value.char_indices() {
            if index >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }
        (line, column)
    }

    /// Caret for a click inside `rect` (single line only in v1).
    pub fn caret_at_point(ui: &Ui, rect: &crate::Rect, value: &str, multiline: bool) -> (usize, usize) {
        if !multiline {
            let inner = rect.with_margin(6.0);
            let x = (ui.input.mouse.0 - inner.x).max(0.0);
            let mut width = 0.0;
            let mut column = 0;
            for ch in value.chars() {
                let w = ui.measure(ch, crate::theme::Font::Mono).0;
                if width + w / 2.0 > x {
                    break;
                }
                width += w;
                column += 1;
            }
            return (0, column);
        }
        (0, 0)
    }

    /// Applies this frame's input to the focused value; returns whether it
    /// changed.
    pub fn apply(ui: &mut Ui, value: &mut String, multiline: bool) -> bool {
        let (line, column) = ui.widgets.caret;
        let mut offset = offset(value, line, column);
        let mut changed = false;
        // Characters typed this frame.
        for unit in ui.input.unicode.iter().copied() {
            match unit {
                8 => {
                    // Backspace.
                    if offset > 0 {
                        let mut previous = offset - 1;
                        while !value.is_char_boundary(previous) {
                            previous -= 1;
                        }
                        value.replace_range(previous..offset, "");
                        offset = previous;
                        changed = true;
                    }
                }
                13 if multiline => {
                    value.insert(offset, '\n');
                    offset += 1;
                    changed = true;
                }
                9 => {}
                code if code >= 0x20 && code != 0x7F => {
                    if let Some(ch) = char::from_u32(u32::from(code)) {
                        if !ch.is_control() {
                            value.insert(offset, ch);
                            offset += ch.len_utf8();
                            changed = true;
                        }
                    }
                }
                _ => {}
            }
        }
        // Keys.
        for key in ui.input.keys.iter().copied() {
            match key {
                0x2E => {
                    // Delete.
                    if offset < value.len() {
                        let mut end = offset + 1;
                        while end < value.len() && !value.is_char_boundary(end) {
                            end += 1;
                        }
                        value.replace_range(offset..end, "");
                        changed = true;
                    }
                }
                0x25 => offset = offset.saturating_sub(prev_char_len(value, offset)),
                0x26 if multiline => offset = line_start(value, offset).map(|o| line_start(value, o.saturating_sub(1)).unwrap_or(0)).unwrap_or(0),
                0x27 => {
                    if offset < value.len() {
                        offset += value[offset..].chars().next().map_or(1, char::len_utf8);
                    }
                }
                0x28 if multiline => {
                    let start = line_start(value, offset).unwrap_or(0);
                    offset = if start > 0 { line_start(value, start - 1).unwrap_or(0) } else { 0 };
                }
                0x24 => offset = line_start(value, offset).unwrap_or(0),
                0x23 => {
                    let rest = &value[offset..];
                    offset += rest.find('\n').unwrap_or(rest.len());
                }
                _ => {}
            }
        }
        if ui.input.paste {
            if let Some(text) = paste_from_clipboard() {
                let text = text.replace("\r\n", "\n").replace('\r', "\n");
                value.insert_str(offset, &text);
                offset += text.len();
                changed = true;
            }
        }
        if changed || ui.widgets.caret != (line, column) {
            ui.widgets.caret = caret(value, offset);
        }
        changed
    }

    fn prev_char_len(value: &str, offset: usize) -> usize {
        if offset == 0 {
            return 0;
        }
        let mut previous = offset - 1;
        while !value.is_char_boundary(previous) {
            previous -= 1;
        }
        offset - previous
    }

    fn line_start(value: &str, offset: usize) -> Option<usize> {
        if offset == 0 {
            return Some(0);
        }
        value[..offset].rfind('\n').map(|i| i + 1)
    }

    fn paste_from_clipboard() -> Option<String> {
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return None;
            }
            let handle = GetClipboardData(CF_UNICODETEXT);
            let mut text = None;
            if !handle.is_null() {
                let locked = GlobalLock(handle as HGLOBAL);
                if !locked.is_null() {
                    let slice = std::slice::from_raw_parts(locked.cast::<u16>(), 64 * 1024);
                    let len = slice.iter().position(|&u| u == 0).unwrap_or(slice.len());
                    text = Some(String::from_utf16_lossy(&slice[..len]));
                    GlobalUnlock(handle as HGLOBAL);
                }
            }
            CloseClipboard();
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::edit;

    #[test]
    fn caret_round_trips_through_offsets() {
        let value = "first line\nsecond ümlaut\nthird";
        for (line, column) in [(0, 0), (0, 5), (1, 7), (2, 5), (1, 0)] {
            let offset = edit::offset(value, line, column);
            assert_eq!(edit::caret(value, offset), (line, column));
        }
        // Out-of-range carets clamp to the end.
        let offset = edit::offset(value, 9, 9);
        assert_eq!(offset, value.len());
        assert_eq!(edit::caret(value, offset), (2, 5));
    }

    #[test]
    fn byte_offsets_lands_on_char_boundaries() {
        let value = "héllo wörld";
        let offset = edit::offset(value, 0, 6);
        assert!(value.is_char_boundary(offset));
        assert_eq!(&value[..offset], "héllo ");
        assert!(value.is_char_boundary(edit::offset(value, 0, 1)));
    }
}