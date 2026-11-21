//! The icon set: one SVG per icon, compiled into the binary.
//!
//! `assets/icons/*.svg` are the source of truth and `include_image!` embeds
//! them at build time, so there is no `assets/` directory to find at runtime and
//! no PNG to fall out of date. egui's SVG loader rasterises one at the size it
//! is drawn and keeps the texture, which is why they stay sharp on a HiDPI
//! screen and on a window that was resized to 400 pixels wide.
//!
//! The files are drawn with `stroke="currentColor"` and no fill, so
//! [`Icon::image`] can tint them with the palette: one icon serves the hover
//! colour, the selected colour and the muted colour.

use egui::{Color32, Image, Response, Ui, Vec2};

/// Every icon the window uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Installed keyboards.
    Library,
    /// The typing sandbox.
    Keyboard,
    /// The key-by-key view.
    Grid,
    /// The keyboard source.
    Code,
    /// Settings.
    Sliders,
    /// Search fields.
    Search,
    /// Adding something.
    Plus,
    /// Removing something.
    Trash,
    /// Reloading the library.
    Refresh,
    /// A path or a directory.
    Folder,
    /// A confirmation.
    Check,
    /// A dismissal.
    Close,
    /// Sending text into the window.
    Play,
    /// A hint.
    Info,
    /// The engine itself.
    Bolt,
    /// Light.
    Sun,
    /// Dark.
    Moon,
    /// Follow the desktop.
    Monitor,
    /// The application's own mark.
    Logo,
    /// The bundled my-win layout.
    MyWin,
    /// The bundled Pyidaungsu layout.
    Pyidaungsu,
    /// The bundled MyanSan layout.
    MyanSan,
    /// The bundled Zawgyi layout.
    Zawgyi,
    /// The bundled ZawCode layout.
    ZawCode,
}

impl Icon {
    /// The five navigation icons, in the order the rail shows them.
    pub const NAV: [Self; 5] = [
        Self::Library,
        Self::Keyboard,
        Self::Grid,
        Self::Code,
        Self::Sliders,
    ];

    /// Every icon, so a test can walk the set without a list to keep in step.
    pub const ALL: [Self; 24] = [
        Self::Library,
        Self::Keyboard,
        Self::Grid,
        Self::Code,
        Self::Sliders,
        Self::Search,
        Self::Plus,
        Self::Trash,
        Self::Refresh,
        Self::Folder,
        Self::Check,
        Self::Close,
        Self::Play,
        Self::Info,
        Self::Bolt,
        Self::Sun,
        Self::Moon,
        Self::Monitor,
        Self::Logo,
        Self::MyWin,
        Self::Pyidaungsu,
        Self::MyanSan,
        Self::Zawgyi,
        Self::ZawCode,
    ];

    /// The file these bytes came from, for the image cache key.
    fn uri(self) -> &'static str {
        match self {
            Self::Library => "bytes://library.svg",
            Self::Keyboard => "bytes://keyboard.svg",
            Self::Grid => "bytes://grid.svg",
            Self::Code => "bytes://code.svg",
            Self::Sliders => "bytes://sliders.svg",
            Self::Search => "bytes://search.svg",
            Self::Plus => "bytes://plus.svg",
            Self::Trash => "bytes://trash.svg",
            Self::Refresh => "bytes://refresh.svg",
            Self::Folder => "bytes://folder.svg",
            Self::Check => "bytes://check.svg",
            Self::Close => "bytes://close.svg",
            Self::Play => "bytes://play.svg",
            Self::Info => "bytes://info.svg",
            Self::Bolt => "bytes://bolt.svg",
            Self::Sun => "bytes://sun.svg",
            Self::Moon => "bytes://moon.svg",
            Self::Monitor => "bytes://monitor.svg",
            Self::Logo => "bytes://logo.svg",
            Self::MyWin => "bytes://keyboard-myin.svg",
            Self::Pyidaungsu => "bytes://keyboard-pyidaungsu.svg",
            Self::MyanSan => "bytes://keyboard-myansan.svg",
            Self::Zawgyi => "bytes://keyboard-zawgyi.svg",
            Self::ZawCode => "bytes://keyboard-zawcode.svg",
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Library => include_bytes!("../../../assets/icons/library.svg"),
            Self::Keyboard => include_bytes!("../../../assets/icons/keyboard.svg"),
            Self::Grid => include_bytes!("../../../assets/icons/grid.svg"),
            Self::Code => include_bytes!("../../../assets/icons/code.svg"),
            Self::Sliders => include_bytes!("../../../assets/icons/sliders.svg"),
            Self::Search => include_bytes!("../../../assets/icons/search.svg"),
            Self::Plus => include_bytes!("../../../assets/icons/plus.svg"),
            Self::Trash => include_bytes!("../../../assets/icons/trash.svg"),
            Self::Refresh => include_bytes!("../../../assets/icons/refresh.svg"),
            Self::Folder => include_bytes!("../../../assets/icons/folder.svg"),
            Self::Check => include_bytes!("../../../assets/icons/check.svg"),
            Self::Close => include_bytes!("../../../assets/icons/close.svg"),
            Self::Play => include_bytes!("../../../assets/icons/play.svg"),
            Self::Info => include_bytes!("../../../assets/icons/info.svg"),
            Self::Bolt => include_bytes!("../../../assets/icons/bolt.svg"),
            Self::Sun => include_bytes!("../../../assets/icons/sun.svg"),
            Self::Moon => include_bytes!("../../../assets/icons/moon.svg"),
            Self::Monitor => include_bytes!("../../../assets/icons/monitor.svg"),
            Self::Logo => include_bytes!("../../../assets/icons/logo.svg"),
            Self::MyWin => include_bytes!("../../../assets/icons/keyboard-myin.svg"),
            Self::Pyidaungsu => include_bytes!("../../../assets/icons/keyboard-pyidaungsu.svg"),
            Self::MyanSan => include_bytes!("../../../assets/icons/keyboard-myansan.svg"),
            Self::Zawgyi => include_bytes!("../../../assets/icons/keyboard-zawgyi.svg"),
            Self::ZawCode => include_bytes!("../../../assets/icons/keyboard-zawcode.svg"),
        }
    }

    /// The mark of an installed keyboard, by the id the library gave it.
    ///
    /// The five that ship with the app have a mark of their own, drawn in the
    /// same language as the icons: a slab on a tilt with what the keyboard is
    /// cut into it. A keyboard a user installed has none, and gets `None` so the
    /// card can fall back to something the library does know about.
    pub fn for_keyboard(id: &str) -> Option<Self> {
        match id {
            "my-win" => Some(Self::MyWin),
            "pyidaungsu-mm" => Some(Self::Pyidaungsu),
            "myansan" => Some(Self::MyanSan),
            "zawgyi" => Some(Self::Zawgyi),
            "zawcode" => Some(Self::ZawCode),
            _ => None,
        }
    }

    /// The icon as an image of exactly `size` points, in `colour`.
    pub fn image(self, size: f32, colour: Color32) -> Image<'static> {
        Image::from_bytes(self.uri(), self.bytes())
            .fit_to_exact_size(Vec2::splat(size))
            .tint(colour)
    }

    /// Draws the icon and returns what the pointer did to it.
    pub fn show(self, ui: &mut Ui, size: f32, colour: Color32) -> Response {
        ui.add(self.image(size, colour))
    }
}

#[cfg(test)]
mod tests {
    use super::Icon;

    /// The bytes really are the file, not an empty include: a wrong path fails
    /// the build, but a path to something that is not SVG would not.
    #[test]
    fn every_icon_is_an_svg() {
        for icon in Icon::ALL {
            let bytes = icon.bytes();
            let text = std::str::from_utf8(bytes).expect("an SVG is text");
            assert!(
                text.trim_start().starts_with("<svg"),
                "{icon:?} is not an SVG"
            );
            assert!(
                text.contains("viewBox=\"0 0 24 24\""),
                "{icon:?} is not on the 24 point grid"
            );
            assert!(
                text.contains("currentColor"),
                "{icon:?} cannot be tinted: it does not use currentColor"
            );
        }
    }

    /// The nav row indexes `Icon::NAV`, so a duplicate there would show one
    /// page's icon on another page's row.
    #[test]
    fn the_nav_icons_are_distinct() {
        let mut seen = Vec::new();
        for icon in Icon::NAV {
            assert!(!seen.contains(&icon), "{icon:?} appears twice in NAV");
            seen.push(icon);
        }
    }

    /// Every icon has its own cache key, or egui would draw one icon's texture
    /// wherever the shared key was asked for.
    #[test]
    fn every_icon_has_its_own_uri() {
        let mut seen = Vec::new();
        for icon in Icon::ALL {
            let uri = icon.uri();
            assert!(!seen.contains(&uri), "{uri} is used by more than one icon");
            seen.push(uri);
        }
    }
}
