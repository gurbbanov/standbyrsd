#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod carousel;
mod config;
mod l10n;
mod media;
mod message;
mod settings;
mod slide_pages;
mod update;
mod weather;
mod widgets;
use crate::app::Application;
use iced::font::{Family, Stretch, Style, Weight};
use iced::{Font, Settings};
use self_update::cargo_crate_version;

const CURRENT_VERSION: &str = cargo_crate_version!();

const SF_PRO_EXPANDED_BOLD: Font = Font {
    family: Family::Name("SF Pro"),
    weight: Weight::Bold,
    stretch: Stretch::Expanded,
    style: Style::Normal,
};

const SF_PRO_COMPRESSED_SEMIBOLD: Font = Font {
    family: Family::Name("SF Pro"),
    weight: Weight::Bold,
    stretch: Stretch::Condensed,
    style: Style::Normal,
};

const SF_PRO_ROUNDED_BLACK: Font = Font {
    family: Family::Name("SF Pro Rounded"),
    weight: Weight::Black,
    ..Font::DEFAULT
};

const SF_PRO_DISPLAY_BOLD: Font = Font {
    family: Family::Name("SF Pro Display"),
    weight: Weight::Bold,
    ..Font::DEFAULT
};

const SF_PRO_DISPLAY_BLACK: Font = Font {
    family: Family::Name("SF Pro Display"),
    weight: Weight::Black,
    ..Font::DEFAULT
};

const SF_PRO_DISPLAY_MEDIUM: Font = Font {
    family: Family::Name("SF Pro Display"),
    weight: Weight::Medium,
    ..Font::DEFAULT
};

const PAGE_COUNT: usize = 2;
const SNAP_THRESHOLD: f32 = 0.025;
const IDLE_MS: u64 = 16;
const SNAP_DURATION_MS: u64 = 420;

const FULLSCREEN_EXIT_SVG: &[u8] = include_bytes!("../icons/fullscreen-exit.svg");
const FULLSCREEN_ENTER_SVG: &[u8] = include_bytes!("../icons/fullscreen-enter.svg");

fn main() -> iced::Result {
    iced::daemon(Application::new, Application::update, Application::view)
        .subscription(Application::subscription)
        .settings(Settings {
            fonts: vec![
                include_bytes!("../fonts/SF-Pro-Rounded.ttf").into(),
                include_bytes!("../fonts/SF-Pro-Expanded.ttf").into(),
                include_bytes!("../fonts/SF-Pro-Display-Black.otf").into(),
                include_bytes!("../fonts/SF-Pro-Display-Bold.otf").into(),
                include_bytes!("../fonts/SF-Pro-Compressed.ttf").into(),
                include_bytes!("../fonts/SF-Pro-Display-Medium.ttf").into(),
            ],
            default_font: SF_PRO_DISPLAY_BOLD,
            ..Settings::default()
        })
        .theme(Application::theme)
        .antialiasing(true)
        .run()
}
