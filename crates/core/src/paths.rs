use std::env;
use std::path::PathBuf;

const APP: &str = "tinyhand";

fn xdg(var: &str, fallback: &str) -> PathBuf {
    env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(fallback))
        .join(APP)
}

pub fn config_file() -> PathBuf {
    env::var_os("TINYHAND_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| xdg("XDG_CONFIG_HOME", ".config").join("config.toml"))
}

pub fn data_home() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share")
}

pub fn custom_hands_dir() -> PathBuf {
    data_home().join("hands")
}

pub fn state_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state")
}

pub fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = env::var_os("TINYHAND_DATA_DIR") {
        dirs.push(PathBuf::from(dir));
    }
    if let Ok(exe) = env::current_exe()
        && let Some(prefix) = exe.parent().and_then(|bin| bin.parent())
    {
        dirs.push(prefix.join("share").join(APP));
    }
    let system = env::var("XDG_DATA_DIRS").unwrap_or_default();
    let system = if system.is_empty() { "/usr/local/share:/usr/share" } else { &system };
    dirs.extend(env::split_paths(system).map(|p| p.join(APP)));
    if cfg!(debug_assertions) {
        dirs.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets")));
    }
    dirs
}

pub fn find_data(rel: &str) -> Option<PathBuf> {
    data_dirs().into_iter().map(|d| d.join(rel)).find(|p| p.exists())
}
