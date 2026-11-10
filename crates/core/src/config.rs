use crate::catalog::{
    DEFAULT_POINTER, DEFAULT_TOOLBAR, InkChoice, InteractKey, KeyboardMode, MAX_PINNED, TapSound,
};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item};
pub use toml_edit::Value;

pub const SIZE_MIN: f64 = 0.08;
pub const SIZE_MAX: f64 = 0.20;
pub const SIZE_DEFAULT: f64 = 0.13;
pub const FADE_MAX: f64 = 3600.0;

pub const KEYS: [&str; 15] = [
    "pointer",
    "size",
    "shadow",
    "toolbar",
    "ink-color",
    "ink-fade",
    "ink-button",
    "tap-sound",
    "sound",
    "laser-sound",
    "interact-key",
    "keyboard",
    "tray",
    "toggle-shortcut",
    "clear-shortcut",
];

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub pointer: String,
    pub size: f64,
    pub shadow: bool,
    pub toolbar: Vec<String>,
    pub ink_color: InkChoice,
    pub ink_fade: f64,
    pub ink_button: bool,
    pub tap_sound: TapSound,
    pub sound: bool,
    pub laser_sound: bool,
    pub interact_key: InteractKey,
    pub keyboard: KeyboardMode,
    pub tray: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            pointer: DEFAULT_POINTER.into(),
            size: SIZE_DEFAULT,
            shadow: true,
            toolbar: DEFAULT_TOOLBAR.map(String::from).to_vec(),
            ink_color: InkChoice::Match,
            ink_fade: 3.0,
            ink_button: true,
            tap_sound: TapSound::Thock,
            sound: true,
            laser_sound: false,
            interact_key: InteractKey::Alt,
            keyboard: KeyboardMode::Grab,
            tray: true,
        }
    }
}

fn number(item: &Item) -> Option<f64> {
    item.as_float().or_else(|| item.as_integer().map(|i| i as f64)).filter(|v| v.is_finite())
}

fn toolbar(item: &Item) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for value in item.as_array()? {
        let id = value.as_str()?;
        if !id.is_empty() && out.len() < MAX_PINNED && !out.iter().any(|s| s == id) {
            out.push(id.into());
        }
    }
    if out.is_empty() {
        out = DEFAULT_TOOLBAR.map(String::from).to_vec();
    }
    Some(out)
}

impl Config {
    pub fn parse(text: &str) -> (Config, Vec<String>) {
        let mut config = Config::default();
        let mut warnings = Vec::new();
        let doc = match text.parse::<DocumentMut>() {
            Ok(doc) => doc,
            Err(err) => {
                let reason = err.to_string();
                let reason = reason.lines().next().unwrap_or_default().to_string();
                return (config, vec![format!("config is not valid TOML, using defaults: {reason}")]);
            }
        };

        macro_rules! read {
            ($key:literal, $field:ident, $convert:expr) => {
                if let Some(item) = doc.get($key) {
                    match $convert(item) {
                        Some(value) => config.$field = value,
                        None => warnings.push(format!("`{}` has an invalid value, using the default", $key)),
                    }
                }
            };
        }
        let text_of = |item: &Item| item.as_str().filter(|s| !s.is_empty()).map(String::from);

        read!("pointer", pointer, text_of);
        read!("size", size, |i| number(i).map(|v| v.clamp(SIZE_MIN, SIZE_MAX)));
        read!("shadow", shadow, Item::as_bool);
        read!("toolbar", toolbar, toolbar);
        read!("ink-color", ink_color, |i: &Item| i.as_str().and_then(InkChoice::parse));
        read!("ink-fade", ink_fade, |i| number(i).map(|v| v.clamp(0.0, FADE_MAX)));
        read!("ink-button", ink_button, Item::as_bool);
        read!("tap-sound", tap_sound, |i: &Item| i.as_str().and_then(TapSound::parse));
        read!("sound", sound, Item::as_bool);
        read!("laser-sound", laser_sound, Item::as_bool);
        read!("interact-key", interact_key, |i: &Item| i.as_str().and_then(InteractKey::parse));
        read!("keyboard", keyboard, |i: &Item| i.as_str().and_then(KeyboardMode::parse));
        read!("tray", tray, Item::as_bool);

        for (key, _) in doc.iter() {
            if !KEYS.contains(&key) {
                warnings.push(format!("unknown key `{key}`"));
            }
        }
        (config, warnings)
    }

    pub fn load(path: &Path) -> (Config, Vec<String>) {
        match fs::read_to_string(path) {
            Ok(text) => Config::parse(&text),
            Err(err) if err.kind() == io::ErrorKind::NotFound => (Config::default(), Vec::new()),
            Err(err) => (Config::default(), vec![format!("cannot read {}: {err}", path.display())]),
        }
    }
}

fn target(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn edit(path: &Path, change: impl FnOnce(&mut DocumentMut)) -> io::Result<()> {
    let target = target(path);
    let text = match fs::read_to_string(&target) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err),
    };
    let mut doc = text
        .parse::<DocumentMut>()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    change(&mut doc);
    if let Some(dir) = target.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = target.with_extension("toml.tmp");
    fs::write(&tmp, doc.to_string())?;
    fs::rename(&tmp, &target).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

pub fn set_key(path: &Path, key: &str, mut value: Value) -> io::Result<()> {
    edit(path, |doc| {
        if let Some(old) = doc.get(key).and_then(Item::as_value) {
            *value.decor_mut() = old.decor().clone();
        }
        doc[key] = Item::Value(value);
    })
}

pub fn reset(path: &Path) -> io::Result<()> {
    edit(path, |doc| {
        for key in KEYS {
            doc.remove(key);
        }
    })
}

pub fn is_writable(path: &Path) -> bool {
    let target = target(path);
    let dir_ok = |dir: &Path| {
        let probe = dir.join(".tinyhand-probe");
        let ok = fs::write(&probe, b"").is_ok();
        let _ = fs::remove_file(&probe);
        ok
    };
    match target.parent() {
        Some(dir) if dir.exists() => dir_ok(dir),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{DEFAULT_TOOLBAR, InteractKey, KeyboardMode, TapSound};
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tinyhand-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_match_upstream() {
        let c = Config::default();
        assert_eq!(c.pointer, "lavender");
        assert_eq!(c.size, 0.13);
        assert!(c.shadow);
        assert_eq!(c.ink_fade, 3.0);
        assert_eq!(c.tap_sound, TapSound::Thock);
        assert!(c.sound);
        assert!(!c.laser_sound);
        assert_eq!(c.toolbar, DEFAULT_TOOLBAR);
        assert_eq!(c.interact_key, InteractKey::Alt);
        assert_eq!(c.keyboard, KeyboardMode::Grab);
        assert!(c.tray);
    }

    #[test]
    fn empty_file_is_defaults_without_warnings() {
        let (c, warnings) = Config::parse("");
        assert_eq!(c, Config::default());
        assert!(warnings.is_empty());
    }

    #[test]
    fn bad_values_fall_back_per_key() {
        let (c, warnings) = Config::parse("size = \"big\"\nshadow = false\n");
        assert_eq!(c.size, 0.13);
        assert!(!c.shadow);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("size"));
    }

    #[test]
    fn unknown_enum_value_falls_back() {
        let (c, warnings) = Config::parse("tap-sound = \"gong\"\nink-color = \"blue\"\n");
        assert_eq!(c.tap_sound, TapSound::Thock);
        assert_eq!(c.ink_color, crate::catalog::InkChoice::Blue);
        assert!(warnings[0].contains("tap-sound"));
    }

    #[test]
    fn unknown_key_is_reported() {
        let (_, warnings) = Config::parse("sizee = 0.1\n");
        assert!(warnings[0].contains("sizee"));
    }

    #[test]
    fn syntax_error_gives_defaults_and_a_warning() {
        let (c, warnings) = Config::parse("size = = 3\n");
        assert_eq!(c, Config::default());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn size_is_clamped() {
        assert_eq!(Config::parse("size = 0.5").0.size, 0.20);
        assert_eq!(Config::parse("size = 0.01").0.size, 0.08);
        assert_eq!(Config::parse("ink-fade = 6").0.ink_fade, 6.0);
        assert_eq!(Config::parse("ink-fade = -2").0.ink_fade, 0.0);
    }

    #[test]
    fn non_finite_numbers_fall_back_with_a_warning() {
        let (c, warnings) = Config::parse("size = nan\nink-fade = inf\n");
        assert_eq!(c.size, 0.13);
        assert_eq!(c.ink_fade, 3.0);
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn huge_fades_are_capped_to_an_hour() {
        assert_eq!(Config::parse("ink-fade = 1e20").0.ink_fade, 3600.0);
    }

    #[test]
    fn toolbar_is_deduped_and_capped() {
        let (c, _) = Config::parse(
            r#"toolbar = ["a", "b", "a", "c", "d", "e", "f", "g", "h", "i"]"#,
        );
        assert_eq!(c.toolbar, ["a", "b", "c", "d", "e", "f", "g", "h"]);
        assert_eq!(Config::parse("toolbar = []").0.toolbar, DEFAULT_TOOLBAR);
    }

    #[test]
    fn set_key_preserves_comments() {
        let file = temp("comments").join("config.toml");
        fs::write(&file, "# mine\nsize = 0.1 # keep\n").unwrap();
        set_key(&file, "shadow", false.into()).unwrap();
        set_key(&file, "size", 0.15.into()).unwrap();
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.contains("# mine"));
        assert!(text.contains("# keep"));
        let (c, warnings) = Config::parse(&text);
        assert!(!c.shadow);
        assert_eq!(c.size, 0.15);
        assert!(warnings.is_empty());
    }

    #[test]
    fn set_key_creates_missing_file_and_dirs() {
        let file = temp("create").join("nested").join("config.toml");
        set_key(&file, "pointer", "robot".into()).unwrap();
        assert_eq!(Config::load(&file).0.pointer, "robot");
    }

    #[test]
    fn set_key_refuses_to_overwrite_a_broken_file() {
        let file = temp("broken").join("config.toml");
        fs::write(&file, "size = = 3\n").unwrap();
        assert!(set_key(&file, "shadow", false.into()).is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), "size = = 3\n");
    }

    #[test]
    fn set_key_writes_through_symlink() {
        let dir = temp("symlink");
        let target = dir.join("store").join("config.toml");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "size = 0.1\n").unwrap();
        let link = dir.join("config.toml");
        symlink(&target, &link).unwrap();
        set_key(&link, "shadow", false.into()).unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert!(fs::read_to_string(&target).unwrap().contains("shadow = false"));
    }

    #[test]
    fn set_key_fails_on_readonly_target() {
        let dir = temp("readonly");
        let store = dir.join("store");
        fs::create_dir_all(&store).unwrap();
        let target = store.join("config.toml");
        fs::write(&target, "size = 0.1\n").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o444)).unwrap();
        fs::set_permissions(&store, fs::Permissions::from_mode(0o555)).unwrap();
        let link = dir.join("config.toml");
        symlink(&target, &link).unwrap();
        assert!(!is_writable(&link));
        assert!(set_key(&link, "shadow", false.into()).is_err());
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), "size = 0.1\n");
        fs::set_permissions(&store, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn reset_removes_known_keys_only() {
        let file = temp("reset").join("config.toml");
        fs::write(&file, "# mine\nsize = 0.1\nmystery = 1\n").unwrap();
        reset(&file).unwrap();
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.contains("mystery = 1"));
        assert!(!text.contains("size"));
    }
}
