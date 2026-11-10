use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::path::Path;
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

const INDEX: &str = "hands.toml";
const NAME_MAX: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct CustomHand {
    pub id: String,
    pub name: String,
    pub ink: u32,
}

pub fn load(dir: &Path) -> Vec<CustomHand> {
    let Ok(text) = fs::read_to_string(dir.join(INDEX)) else {
        return Vec::new();
    };
    let Ok(doc) = text.parse::<DocumentMut>() else {
        return Vec::new();
    };
    let Some(tables) = doc.get("hand").and_then(Item::as_array_of_tables) else {
        return Vec::new();
    };
    tables
        .iter()
        .filter_map(|t| {
            let ink = t.get("ink")?.as_str()?.trim_start_matches('#');
            Some(CustomHand {
                id: t.get("id")?.as_str()?.into(),
                name: t.get("name")?.as_str()?.into(),
                ink: u32::from_str_radix(ink, 16).ok()?,
            })
        })
        .collect()
}

fn save(dir: &Path, hands: &[CustomHand]) -> io::Result<()> {
    let mut tables = ArrayOfTables::new();
    for hand in hands {
        let mut table = Table::new();
        table["id"] = value(&hand.id);
        table["name"] = value(&hand.name);
        table["ink"] = value(format!("#{:06x}", hand.ink));
        tables.push(table);
    }
    let mut doc = DocumentMut::new();
    doc["hand"] = Item::ArrayOfTables(tables);
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{INDEX}.tmp"));
    fs::write(&tmp, doc.to_string())?;
    fs::rename(tmp, dir.join(INDEX))
}

fn new_id(taken: &[CustomHand]) -> String {
    loop {
        let random = std::collections::hash_map::RandomState::new().build_hasher().finish();
        let id = format!("custom-{:08x}", random as u32);
        if !taken.iter().any(|h| h.id == id) {
            return id;
        }
    }
}

pub fn add(dir: &Path, name: &str, ink: u32) -> io::Result<CustomHand> {
    let mut hands = load(dir);
    let name = name.trim();
    let hand = CustomHand {
        id: new_id(&hands),
        name: if name.is_empty() {
            format!("My hand {}", hands.len() + 1)
        } else {
            name.chars().take(NAME_MAX).collect()
        },
        ink,
    };
    hands.push(hand.clone());
    save(dir, &hands)?;
    Ok(hand)
}

pub fn remove(dir: &Path, id: &str) -> io::Result<()> {
    let hands: Vec<CustomHand> = load(dir).into_iter().filter(|h| h.id != id).collect();
    match fs::remove_file(dir.join(format!("{id}.png"))) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => return Err(err),
        _ => {}
    }
    save(dir, &hands)
}

pub fn suggested_name(stem: &str, count: usize) -> String {
    let words: Vec<&str> = stem.split(['_', '-', ' ']).filter(|w| !w.is_empty()).collect();
    let name = words.iter().take(2).copied().collect::<Vec<_>>().join(" ");
    let generated = words.join(" ").to_lowercase().starts_with("chatgpt image");
    if name.is_empty() || name.chars().count() > NAME_MAX || generated {
        return format!("My hand {}", count + 1);
    }
    let mut chars = name.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tinyhand-custom-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn missing_directory_loads_as_empty() {
        assert!(load(&temp("missing")).is_empty());
    }

    #[test]
    fn add_load_remove_round_trip() {
        let dir = temp("roundtrip");
        let a = add(&dir, "  Wool  ", 0x112233).unwrap();
        let b = add(&dir, "", 0xABCDEF).unwrap();
        assert!(a.id.starts_with("custom-") && a.id.len() == 15);
        assert_ne!(a.id, b.id);
        assert_eq!(a.name, "Wool");
        assert_eq!(b.name, "My hand 2");
        fs::write(dir.join(format!("{}.png", a.id)), b"png").unwrap();
        assert_eq!(load(&dir), vec![a.clone(), b.clone()]);
        remove(&dir, &a.id).unwrap();
        assert_eq!(load(&dir), vec![b]);
        assert!(!dir.join(format!("{}.png", a.id)).exists());
    }

    #[test]
    fn names_are_cut_to_sixteen_characters() {
        let dir = temp("long");
        assert_eq!(add(&dir, "abcdefghijklmnopqrstuvwxyz", 0).unwrap().name, "abcdefghijklmnop");
    }

    #[test]
    fn suggested_names_come_from_the_file_name() {
        assert_eq!(suggested_name("knit_hand-final", 0), "Knit hand");
        assert_eq!(suggested_name("ChatGPT Image Nov 9", 2), "My hand 3");
        assert_eq!(suggested_name("", 0), "My hand 1");
        assert_eq!(suggested_name("extraordinarily longfilename", 0), "My hand 1");
    }
}
