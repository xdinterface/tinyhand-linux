pub struct Hand {
    pub id: &'static str,
    pub title: &'static str,
    pub ink: u32,
}

pub const PURPLE: u32 = 0x8754F2;
pub const RED: u32 = 0xFF291F;
pub const LASER: &str = "laser";
pub const LASER_TITLE: &str = "Laser dot";
pub const DEFAULT_POINTER: &str = "lavender";
pub const DEFAULT_TOOLBAR: [&str; 6] = ["lavender", "glove", "coral", "plush", "robot", LASER];
pub const MAX_PINNED: usize = 8;

const fn hand(id: &'static str, title: &'static str, ink: u32) -> Hand {
    Hand { id, title, ink }
}

pub const BUILTIN: [Hand; 36] = [
    hand("lavender", "Lavender toy", PURPLE),
    hand("glove", "Cursor glove", PURPLE),
    hand("coral", "Coral vinyl", RED),
    hand("plush", "Plush", 0x2F86F0),
    hand("robot", "Robot", 0x16A87A),
    hand("pink", "Bubblegum", 0xEE4F98),
    hand("blue", "Sky", 0x2F86F0),
    hand("mint", "Mint", 0x16A87A),
    hand("lemon", "Lemon", 0xF2B705),
    hand("black", "Midnight", 0x1D1D21),
    hand("chrome", "Chrome", 0x6B7380),
    hand("holo", "Holo", PURPLE),
    hand("fuzzy", "Fuzzy", PURPLE),
    hand("wood", "Wood", 0x9A5E2C),
    hand("marble", "Marble", 0x6B7380),
    hand("terrazzo", "Terrazzo", 0xF0561D),
    hand("glass", "Glass", 0x1597E5),
    hand("jelly", "Jelly", 0xA33FE6),
    hand("pastel", "Pastel", 0xEE4F98),
    hand("pixel", "Pixel", 0x1D1D21),
    hand("lowpoly", "Low poly", PURPLE),
    hand("knit", "Knit", PURPLE),
    hand("skeleton", "Skeleton", 0x9A5E2C),
    hand("astro", "Astronaut", 0x2F86F0),
    hand("space", "Space", RED),
    hand("monster", "Monster", 0x3E9E2E),
    hand("water", "Water", 0x1597E5),
    hand("lava", "Lava", 0xF0561D),
    hand("grass", "Grass", 0x3E9E2E),
    hand("cactus", "Cactus", 0x3E9E2E),
    hand("choco", "Chocolate", 0x9A5E2C),
    hand("cloud", "Cloud", 0x2F86F0),
    hand("mech", "Mech", 0xF0561D),
    hand("neon", "Neon", 0xE0309A),
    hand("leaf", "Leaf", 0x3E9E2E),
    hand("cookie", "Gingerbread", 0x9A5E2C),
];

pub fn builtin(id: &str) -> Option<&'static Hand> {
    BUILTIN.iter().find(|h| h.id == id)
}

macro_rules! choice {
    ($name:ident { $($variant:ident = $key:literal, $title:literal;)+ }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $key),+ }
            }

            pub fn title(self) -> &'static str {
                match self { $($name::$variant => $title),+ }
            }

            pub fn parse(s: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|v| v.as_str() == s)
            }
        }
    };
}

choice!(InkChoice {
    Match = "match", "Match the pointer";
    Red = "red", "Red";
    Orange = "orange", "Orange";
    Yellow = "yellow", "Yellow";
    Green = "green", "Green";
    Blue = "blue", "Blue";
    Purple = "purple", "Purple";
    White = "white", "White";
    Black = "black", "Black";
});

impl InkChoice {
    pub fn color(self) -> Option<u32> {
        Some(match self {
            InkChoice::Match => return None,
            InkChoice::Red => RED,
            InkChoice::Orange => 0xFF8A00,
            InkChoice::Yellow => 0xFFCC00,
            InkChoice::Green => 0x30C25A,
            InkChoice::Blue => 0x0A84FF,
            InkChoice::Purple => PURPLE,
            InkChoice::White => 0xFFFFFF,
            InkChoice::Black => 0x1D1D21,
        })
    }
}

choice!(TapSound {
    Thock = "thock", "Thock";
    Click = "click", "Click";
    Pop = "pop", "Pop";
    Wood = "wood", "Wood block";
    Boop = "boop", "Boop";
    Squeak = "squeak", "Squeak";
});

choice!(InteractKey {
    Alt = "alt", "Alt";
    Ctrl = "ctrl", "Ctrl";
    Super = "super", "Super";
    Shift = "shift", "Shift";
    Off = "off", "Off";
});

choice!(KeyboardMode {
    Grab = "grab", "TinyHand holds the keyboard";
    Pass = "pass", "Apps keep the keyboard";
});

pub fn rgb(color: u32) -> (f32, f32, f32) {
    (
        ((color >> 16) & 0xFF) as f32 / 255.0,
        ((color >> 8) & 0xFF) as f32 / 255.0,
        (color & 0xFF) as f32 / 255.0,
    )
}
