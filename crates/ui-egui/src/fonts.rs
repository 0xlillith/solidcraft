//! System font loading and fallback setup for multi-language (CJK/Japanese) support.

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

/// Candidate font paths to look for on different operating systems.
#[cfg(not(target_arch = "wasm32"))]
const FONT_CANDIDATES: &[&str] = &[
    // Windows
    "C:\\Windows\\Fonts\\meiryo.ttc",
    "C:\\Windows\\Fonts\\YuGothM.ttc",
    "C:\\Windows\\Fonts\\YuGothR.ttc",
    "C:\\Windows\\Fonts\\msgothic.ttc",
    "C:\\Windows\\Fonts\\BIZ-UDGothicR.ttc",
    // macOS
    "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/System/Library/Fonts/PingFang.ttc",
    "/Library/Fonts/Arial Unicode.ttf",
    // Linux
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
];

/// Configure font definitions with system CJK fallback if available.
#[cfg(not(target_arch = "wasm32"))]
pub fn setup_fonts(ctx: &egui::Context) {
    let mut font_bytes: Option<Vec<u8>> = None;
    for path in FONT_CANDIDATES {
        if let Ok(bytes) = std::fs::read(path)
            && ttf_parser::Face::parse(&bytes, 0).is_ok()
        {
            font_bytes = Some(bytes);
            break;
        }
    }

    let Some(bytes) = font_bytes else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("cjk_fallback".to_owned(), Arc::new(egui::FontData::from_owned(bytes)));

    fonts.families.entry(egui::FontFamily::Proportional).or_default().push("cjk_fallback".to_owned());

    fonts.families.entry(egui::FontFamily::Monospace).or_default().push("cjk_fallback".to_owned());

    ctx.set_fonts(fonts);
}

/// Browsers cannot read system font files. A web host can supply licensed fonts through egui.
#[cfg(target_arch = "wasm32")]
pub fn setup_fonts(_ctx: &egui::Context) {}
