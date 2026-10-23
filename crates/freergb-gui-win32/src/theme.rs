//! Palette, fonts and DPI metrics for the Win32 GUI. Two skins ship:
//! a flat modern style and a Windows 7 Aero approximation (glossy gradient
//! buttons, Explorer-blue selection) drawn with plain GDI. All sizes are
//! logical pixels scaled by the window DPI.

use crate::ffi::*;
use std::collections::HashMap;

/// 0x00BBGGRR helper.
pub const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (b as u32) << 16 | (g as u32) << 8 | r as u32
}

/// Visual style used for every widget.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    /// Flat modern Windows look.
    Modern,
    /// Windows 7 Aero look: gradients, glossy buttons, Explorer blues.
    Aero,
}

impl Style {
    pub fn label(self) -> &'static str {
        match self {
            Self::Modern => "Modern",
            Self::Aero => "Aero (Win7)",
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Modern => Self::Aero,
            Self::Aero => Self::Modern,
        }
    }
}

/// Per-state button colours. Aero gradients are approximations of the
/// original msstyles bitmaps, drawn as vertical GDI gradients.
#[derive(Clone, Copy)]
pub struct ButtonColors {
    pub top: COLORREF,
    pub bottom: COLORREF,
    pub border: COLORREF,
    pub hover_top: COLORREF,
    pub hover_bottom: COLORREF,
    pub hover_border: COLORREF,
    pub press_top: COLORREF,
    pub press_bottom: COLORREF,
    pub press_border: COLORREF,
    pub primary_top: COLORREF,
    pub primary_bottom: COLORREF,
    pub primary_border: COLORREF,
    /// The greyed-out state Win7 shows for disabled command buttons.
    pub disabled_top: COLORREF,
    pub disabled_bottom: COLORREF,
    pub disabled_border: COLORREF,
}

/// Complete skin description; not every widget reads every palette entry
/// yet (the reserved colours keep both skins evolvable).
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct Palette {
    pub window: COLORREF,
    pub nav: COLORREF,
    pub devices: COLORREF,
    pub card: COLORREF,
    pub text: COLORREF,
    pub text_dim: COLORREF,
    pub text_inv: COLORREF,
    pub border: COLORREF,
    pub field: COLORREF,
    pub field_border: COLORREF,
    pub focus_border: COLORREF,
    pub hover: COLORREF,
    pub active: COLORREF,
    pub selected: COLORREF,
    pub selected_border: COLORREF,
    pub list_hover: COLORREF,
    pub list_selected: COLORREF,
    pub list_selected_border: COLORREF,
    pub track: COLORREF,
    pub track_fill: COLORREF,
    pub grid: COLORREF,
    pub accent: COLORREF,
    pub accent_dim: COLORREF,
    pub good: COLORREF,
    pub warn: COLORREF,
    pub bad: COLORREF,
    pub button: ButtonColors,
    /// Corner radius of boxes in logical pixels.
    pub radius: f32,
    /// Aero paints a subtle gradient strip behind the header.
    pub chrome_gradient: bool,
}

const ACCENT: COLORREF = rgb(0, 155, 222);

/// Healthy/connected accent.
pub const GOOD: COLORREF = rgb(0, 153, 51);
/// Advisory warning accent (conflicts, unverified states).
pub const WARN: COLORREF = rgb(217, 133, 0);
/// Error accent (unreachable, failed calls).
pub const BAD: COLORREF = rgb(196, 43, 28);

pub fn palette(style: Style) -> Palette {
    match style {
        Style::Modern => Palette {
            window: rgb(243, 243, 243),
            nav: rgb(235, 235, 235),
            devices: rgb(238, 238, 238),
            card: rgb(252, 252, 252),
            text: rgb(30, 30, 30),
            text_dim: rgb(100, 100, 100),
            text_inv: rgb(255, 255, 255),
            border: rgb(214, 214, 214),
            field: rgb(255, 255, 255),
            field_border: rgb(150, 150, 150),
            focus_border: ACCENT,
            hover: rgb(244, 244, 244),
            active: rgb(226, 226, 226),
            selected: rgb(224, 236, 244),
            selected_border: ACCENT,
            list_hover: rgb(236, 242, 246),
            list_selected: rgb(207, 235, 247),
            list_selected_border: ACCENT,
            track: rgb(196, 196, 196),
            track_fill: ACCENT,
            grid: rgb(226, 226, 226),
            accent: ACCENT,
            accent_dim: rgb(207, 235, 247),
            good: rgb(16, 140, 80),
            warn: rgb(176, 124, 0),
            bad: rgb(196, 60, 60),
            button: ButtonColors {
                top: rgb(253, 253, 253),
                bottom: rgb(243, 243, 243),
                border: rgb(176, 176, 176),
                hover_top: rgb(247, 249, 250),
                hover_bottom: rgb(233, 238, 241),
                hover_border: rgb(140, 150, 158),
                press_top: rgb(230, 235, 238),
                press_bottom: rgb(214, 221, 226),
                press_border: rgb(110, 120, 128),
                primary_top: rgb(24, 150, 210),
                primary_bottom: rgb(0, 120, 185),
                primary_border: rgb(0, 105, 165),
                disabled_top: rgb(249, 249, 249),
                disabled_bottom: rgb(240, 240, 240),
                disabled_border: rgb(211, 211, 211),
            },
            radius: 5.0,
            chrome_gradient: false,
        },
        Style::Aero => Palette {
            window: rgb(240, 240, 240),
            nav: rgb(240, 240, 240),
            devices: rgb(244, 244, 244),
            card: rgb(255, 255, 255),
            text: rgb(0, 0, 0),
            text_dim: rgb(109, 109, 109),
            text_inv: rgb(255, 255, 255),
            border: rgb(204, 204, 204),
            field: rgb(255, 255, 255),
            field_border: rgb(122, 122, 122),
            focus_border: rgb(86, 157, 229),
            hover: rgb(234, 246, 253),
            active: rgb(219, 230, 240),
            selected: rgb(203, 232, 252),
            selected_border: rgb(125, 162, 206),
            list_hover: rgb(232, 242, 251),
            list_selected: rgb(203, 232, 252),
            list_selected_border: rgb(125, 162, 206),
            track: rgb(216, 216, 216),
            track_fill: rgb(90, 170, 230),
            grid: rgb(230, 230, 230),
            accent: ACCENT,
            accent_dim: rgb(211, 239, 252),
            good: rgb(0, 153, 51),
            warn: rgb(217, 133, 0),
            bad: rgb(196, 43, 28),
            button: ButtonColors {
                // Aero push button: soft grey gloss.
                top: rgb(242, 242, 242),
                bottom: rgb(226, 226, 226),
                border: rgb(130, 130, 130),
                // Hover: the characteristic pale blue wash.
                hover_top: rgb(234, 246, 253),
                hover_bottom: rgb(210, 234, 250),
                hover_border: rgb(60, 127, 177),
                // Pressed: deeper blue, high contrast border.
                press_top: rgb(202, 230, 247),
                press_bottom: rgb(170, 208, 240),
                press_border: rgb(44, 98, 139),
                // Default/primary: UAC-style blue gloss with white text.
                primary_top: rgb(86, 160, 222),
                primary_bottom: rgb(45, 113, 184),
                primary_border: rgb(31, 92, 158),
                // Disabled: flat pale grey, dim text.
                disabled_top: rgb(243, 244, 245),
                disabled_bottom: rgb(233, 235, 237),
                disabled_border: rgb(198, 199, 200),
            },
            radius: 3.0,
            chrome_gradient: true,
        },
    }
}

#[derive(Clone, Copy)]
pub enum Font {
    Body,
    Small,
    Strong,
    Heading,
    Mono,
}

/// Cached GDI font handles and the measured body line height.
#[derive(Clone, Copy, Default)]
pub struct Fonts {
    pub body: HFONT,
    pub small: HFONT,
    pub strong: HFONT,
    pub heading: HFONT,
    pub mono: HFONT,
    /// Height of one body text line in pixels.
    pub line: f32,
}

pub fn create_fonts(dc: HDC, scale: f32) -> Fonts {
    unsafe {
        let pixel = |pt: f32| (-pt * scale * 96.0 / 72.0).round() as i32;
        let face = |name: &str| {
            let mut wide: Vec<u16> = name.encode_utf16().collect();
            wide.push(0);
            wide
        };
        let make = |pt: f32, weight: i32, name: &str| {
            CreateFontW(
                pixel(pt),
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                DEFAULT_CHARSET as u32,
                OUT_DEFAULT_PRECIS as u32,
                CLIP_DEFAULT_PRECIS as u32,
                CLEARTYPE_QUALITY as u32,
                DEFAULT_PITCH as u32,
                face(name).as_ptr(),
            )
        };
        let fonts = Fonts {
            body: make(9.0, FW_NORMAL as i32, "Segoe UI"),
            small: make(8.0, FW_NORMAL as i32, "Segoe UI"),
            strong: make(9.0, FW_SEMIBOLD as i32, "Segoe UI"),
            heading: make(11.0, FW_SEMIBOLD as i32, "Segoe UI"),
            mono: make(9.0, FW_NORMAL as i32, "Consolas"),
            line: 0.0,
        };
        // Measure the body line height with DrawText so labels get the
        // same padding GDI text controls would use.
        let old = SelectObject(dc, fonts.body as _);
        let text: Vec<u16> = "Ag".encode_utf16().collect();
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 200,
            bottom: 40,
        };
        DrawTextW(
            dc,
            text.as_ptr(),
            text.len() as i32,
            &mut rect,
            DT_CALCRECT,
        );
        SelectObject(dc, old);
        Fonts {
            line: ((rect.bottom - rect.top) as f32).max(14.0 * scale),
            ..fonts
        }
    }
}

/// Brush cache so repeated solid fills do not churn GDI objects every frame.
#[derive(Default)]
pub struct Brushes {
    cache: HashMap<COLORREF, HBRUSH>,
}

impl Brushes {
    pub fn get(&mut self, color: COLORREF) -> HBRUSH {
        if let Some(brush) = self.cache.get(&color) {
            return *brush;
        }
        let brush = unsafe { CreateSolidBrush(color) };
        if self.cache.len() >= 256 {
            // One-shot cache reset; the new brush is kept.
            for (_, old) in self.cache.drain() {
                unsafe { DeleteObject(old as _) };
            }
        }
        self.cache.insert(color, brush);
        brush
    }
}

/// Per-user style preference, stored next to the Classic GUI's files.
pub fn load_style() -> Style {
    let Some(root) = config_root() else {
        return Style::Modern;
    };
    match std::fs::read_to_string(root.join("win32-style.json")) {
        Ok(text) if text.trim() == "\"aero\"" => Style::Aero,
        Ok(text) if text.trim() == "\"modern\"" => Style::Modern,
        _ => Style::Modern,
    }
}

pub fn save_style(style: Style) {
    let Some(root) = config_root() else {
        return;
    };
    let _ = std::fs::create_dir_all(&root).and_then(|()| {
        std::fs::write(
            root.join("win32-style.json"),
            if style == Style::Aero {
                "\"aero\""
            } else {
                "\"modern\""
            },
        )
    });
}

fn config_root() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA").map(|p| std::path::PathBuf::from(p).join("freeRGB"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorref_is_bgr() {
        assert_eq!(rgb(0, 155, 222), 0x00DE_9B00);
        assert_eq!(rgb(255, 0, 0), 0x0000_00FF);
    }

    #[test]
    fn styles_toggle() {
        assert_eq!(Style::Aero.other(), Style::Modern);
        assert_eq!(Style::Modern.other(), Style::Aero);
        assert_eq!(Style::Aero.label(), "Aero (Win7)");
    }
}