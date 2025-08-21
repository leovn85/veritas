use egui_colors::{tokens::ThemeColor, Theme};

pub const VERMILLION: Theme = [
    ThemeColor::Gold,
    ThemeColor::Gold,
    ThemeColor::Gold,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Cyan,
    ThemeColor::Cyan,
    ThemeColor::Custom([232, 67, 0]),
    ThemeColor::Custom([232, 67, 0]),
    ThemeColor::Custom([180, 180, 180]),
    ThemeColor::Cyan,
];

pub const EGUI_THEME: Theme = [
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::EguiBlue,
    ThemeColor::EguiBlue,
    ThemeColor::Custom([180, 180, 180]),
    ThemeColor::Gray,
];

pub const OFFICE_GRAY: Theme = [
    ThemeColor::Custom([140, 149, 138]),
    ThemeColor::Custom([140, 149, 138]),
    ThemeColor::Custom([140, 149, 138]),
    ThemeColor::Custom([122, 166, 168]),
    ThemeColor::Gray,
    ThemeColor::Custom([122, 166, 168]),
    ThemeColor::Custom([122, 166, 168]),
    ThemeColor::Custom([122, 166, 168]),
    ThemeColor::Custom([59, 71, 97]),
    ThemeColor::Custom([59, 71, 97]),
    ThemeColor::Custom([185, 178, 168]),
    ThemeColor::Custom([185, 178, 168]),
];

pub const INDIGO_JADE: Theme = [
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Indigo,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Jade,
    ThemeColor::Jade,
    ThemeColor::Jade,
    ThemeColor::Custom([180, 180, 180]),
    ThemeColor::Gray,
];

pub const GRASS_BRONZE: Theme = [
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Grass,
    ThemeColor::Bronze,
    ThemeColor::Bronze,
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Green,
    ThemeColor::Bronze,
    ThemeColor::Bronze,
    ThemeColor::Custom([180, 180, 180]),
    ThemeColor::Gray,
];

pub const WARM: Theme = [
    ThemeColor::Gray,
    ThemeColor::Gray,
    ThemeColor::Orange,
    ThemeColor::Gold,
    ThemeColor::Gold,
    ThemeColor::Gold,
    ThemeColor::Red,
    ThemeColor::Red,
    ThemeColor::Gold,
    ThemeColor::Gold,
    ThemeColor::Custom([180, 180, 180]),
    ThemeColor::Teal,
];

pub const COOL: Theme = [
    ThemeColor::Gray,
    ThemeColor::Indigo,
    ThemeColor::Indigo,
    ThemeColor::Iris,
    ThemeColor::Indigo,
    ThemeColor::Gray,
    ThemeColor::Iris,
    ThemeColor::Indigo,
    ThemeColor::Blue,
    ThemeColor::Indigo,
    ThemeColor::Orange,
    ThemeColor::Gray,
];

pub const SEVENTIES: Theme = [
    ThemeColor::Custom([95, 78, 163]),
    ThemeColor::Pink,
    ThemeColor::Pink,
    ThemeColor::Custom([95, 78, 163]),
    ThemeColor::Custom([95, 78, 163]),
    ThemeColor::Custom([254, 180, 0]),
    ThemeColor::Custom([95, 78, 163]),
    ThemeColor::Custom([95, 78, 163]),
    ThemeColor::Custom([254, 180, 0]),
    ThemeColor::Custom([254, 180, 0]),
    ThemeColor::Custom([180, 180, 180]),
    ThemeColor::Gray,
];

pub const THEMES: [Theme; 8] = [
    EGUI_THEME,
    INDIGO_JADE,
    GRASS_BRONZE,
    WARM,
    COOL,
    SEVENTIES,
    OFFICE_GRAY,
    VERMILLION,
];


pub const THEME_NAMES: [&str; 8] = [
    "Egui",
    "Indigo/jade",
    "Grass/bronze",
    "Warm",
    "Cool",
    "Seventies",
    "Office Gray",
    "Vermillion",
];
