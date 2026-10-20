//! Réglages de l'outil, dans %APPDATA%\Eclair\config.txt (une ligne clé=valeur).

use std::path::PathBuf;

pub struct Config {
    pub mode: u8,
    /// Persistant : réglages rétablis au lancement, favoris en priorité haute dans Windows,
    /// et optimiseur lancé à l'ouverture de session.
    pub persistent: bool,
    pub favorites: Vec<String>,
    /// Exécutables dont la priorité a été enregistrée dans Windows, pour pouvoir l'effacer.
    pub persisted: Vec<String>,
    pub auto_purge: bool,
    pub list_mb: u32,
    pub free_mb: u32,
    pub fine_timer: bool,
    pub auto_gaming: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: 0,
            persistent: false,
            favorites: vec!["discord.exe".into()],
            persisted: Vec::new(),
            auto_purge: false,
            list_mb: 1024,
            free_mb: 1024,
            fine_timer: false,
            auto_gaming: false,
        }
    }
}

/// Dossier des fichiers d'Éclair (réglages, journal des priorités).
pub fn dir() -> Option<PathBuf> {
    let d = PathBuf::from(std::env::var("APPDATA").ok()?).join("Eclair");
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

fn path() -> Option<PathBuf> {
    dir().map(|d| d.join("config.txt"))
}

impl Config {
    pub fn load() -> Self {
        path().and_then(|p| std::fs::read_to_string(p).ok()).map_or_else(Self::default, |s| Self::parse(&s))
    }

    pub fn save(&self) {
        if let Some(p) = path() {
            let _ = std::fs::write(p, self.format());
        }
    }

    fn parse(s: &str) -> Self {
        let mut c = Self { favorites: Vec::new(), ..Self::default() };
        for (k, v) in s.lines().filter_map(|l| l.split_once('=')) {
            match k {
                "mode" => c.mode = v.parse().unwrap_or(0),
                "persistent" => c.persistent = v == "1",
                "favorite" => c.favorites.push(v.to_lowercase()),
                "persisted" => c.persisted.push(v.to_lowercase()),
                "auto_purge" => c.auto_purge = v == "1",
                "list_mb" => c.list_mb = v.parse().unwrap_or(c.list_mb),
                "free_mb" => c.free_mb = v.parse().unwrap_or(c.free_mb),
                "fine_timer" => c.fine_timer = v == "1",
                "auto_gaming" => c.auto_gaming = v == "1",
                _ => {}
            }
        }
        c
    }

    fn format(&self) -> String {
        let mut s = format!(
            "mode={}\npersistent={}\nauto_purge={}\nlist_mb={}\nfree_mb={}\nfine_timer={}\nauto_gaming={}\n",
            self.mode,
            self.persistent as u8,
            self.auto_purge as u8,
            self.list_mb,
            self.free_mb,
            self.fine_timer as u8,
            self.auto_gaming as u8
        );
        for f in &self.favorites {
            s += &format!("favorite={f}\n");
        }
        for f in &self.persisted {
            s += &format!("persisted={f}\n");
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let c = Config {
            mode: 2,
            persistent: true,
            favorites: vec!["a.exe".into(), "b.exe".into()],
            persisted: vec!["a.exe".into()],
            auto_purge: true,
            list_mb: 2048,
            free_mb: 512,
            fine_timer: true,
            auto_gaming: true,
        };
        let back = Config::parse(&c.format());
        assert_eq!((back.mode, back.persistent, back.auto_purge, back.fine_timer), (2, true, true, true));
        assert_eq!((back.list_mb, back.free_mb, back.auto_gaming), (2048, 512, true));
        assert_eq!(back.favorites, c.favorites);
        assert_eq!(back.persisted, c.persisted);
        assert!(Config::parse("").favorites.is_empty(), "liste vidée par l'utilisateur = vide");
    }
}
