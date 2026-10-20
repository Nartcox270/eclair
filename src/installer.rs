//! Installation des applications courantes avec winget (le gestionnaire de paquets de Windows).
//! Identifiants vérifiés avec `winget show --id <id> -e`.

use std::collections::HashSet;
use std::os::windows::process::CommandExt;
use std::process::Command;

const NO_WINDOW: u32 = 0x0800_0000; // CREATE_NO_WINDOW

pub struct App {
    pub category: &'static str,
    pub name: &'static str,
    pub id: &'static str,
}

const fn app(category: &'static str, name: &'static str, id: &'static str) -> App {
    App { category, name, id }
}

pub const APPS: &[App] = &[
    app("Navigateurs", "Brave", "Brave.Brave"),
    app("Navigateurs", "Firefox", "Mozilla.Firefox"),
    app("Navigateurs", "Google Chrome", "Google.Chrome"),
    app("Navigateurs", "Opera GX", "Opera.OperaGX"),
    app("Communication", "Discord", "Discord.Discord"),
    app("Communication", "TeamSpeak", "TeamSpeakSystems.TeamSpeakClient"),
    app("Communication", "Telegram", "Telegram.TelegramDesktop"),
    app("Jeux", "Steam", "Valve.Steam"),
    app("Jeux", "Epic Games Launcher", "EpicGames.EpicGamesLauncher"),
    app("Jeux", "EA App", "ElectronicArts.EADesktop"),
    app("Jeux", "Ubisoft Connect", "Ubisoft.Connect"),
    app("Jeux", "GOG Galaxy", "GOG.Galaxy"),
    app("Jeux", "Battle.net", "Blizzard.BattleNet"),
    app("Jeux", "Medal (clips)", "MedalB.V.Medal"),
    app("Utilitaires", "7-Zip", "7zip.7zip"),
    app("Utilitaires", "WinRAR", "RARLab.WinRAR"),
    app("Utilitaires", "VLC", "VideoLAN.VLC"),
    app("Utilitaires", "Notepad++", "Notepad++.Notepad++"),
    app("Utilitaires", "Everything (recherche)", "voidtools.Everything"),
    app("Utilitaires", "PowerToys", "Microsoft.PowerToys"),
    app("Utilitaires", "qBittorrent", "qBittorrent.qBittorrent"),
    app("Utilitaires", "LibreOffice", "TheDocumentFoundation.LibreOffice"),
    app("Multimédia", "OBS Studio", "OBSProject.OBSStudio"),
    app("Multimédia", "Spotify", "Spotify.Spotify"),
    app("Matériel", "MSI Afterburner", "Guru3D.Afterburner"),
    app("Matériel", "CPU-Z", "CPUID.CPU-Z"),
    app("Matériel", "GPU-Z", "TechPowerUp.GPU-Z"),
    app("Matériel", "HWiNFO", "REALiX.HWiNFO"),
    app("Matériel", "CrystalDiskInfo", "CrystalDewWorld.CrystalDiskInfo"),
    app("Développement", "Visual Studio Code", "Microsoft.VisualStudioCode"),
    app("Développement", "Git", "Git.Git"),
];

/// Identifiants winget des applis déjà installées ; None si winget est absent.
pub fn installed() -> Option<HashSet<String>> {
    let file = std::env::temp_dir().join("optimiseur-winget-export.json");
    Command::new("winget")
        .args(["export", "--accept-source-agreements", "--disable-interactivity", "-o"])
        .arg(&file)
        .creation_flags(NO_WINDOW)
        .output()
        .ok()?;
    let json = std::fs::read_to_string(&file).ok()?;
    let _ = std::fs::remove_file(&file);
    Some(parse_export(&json))
}

// Lignes de la forme  "PackageIdentifier" : "Brave.Brave"
fn parse_export(json: &str) -> HashSet<String> {
    json.lines()
        .filter_map(|l| l.split_once("\"PackageIdentifier\""))
        .filter_map(|(_, rest)| rest.split('"').nth(1))
        .map(str::to_string)
        .collect()
}

/// Installe sans fenêtre ni question ; Err(message) en cas d'échec.
pub fn install(id: &str) -> Result<(), String> {
    let out = Command::new("winget")
        .args(["install", "--id", id, "-e", "--silent", "--accept-package-agreements", "--accept-source-agreements", "--disable-interactivity"])
        .creation_flags(NO_WINDOW)
        .output()
        .map_err(|e| format!("winget introuvable ({e})"))?;
    match out.status.code() {
        Some(0) => Ok(()),
        Some(code) => Err(format!("code {code:#x}")),
        None => Err("interrompu".into()),
    }
}

/// Met à jour toutes les applis que winget sait mettre à jour. Long : hors du thread de l'interface.
pub fn upgrade_all() -> Result<(), String> {
    let out = Command::new("winget")
        .args(["upgrade", "--all", "--silent", "--accept-package-agreements", "--accept-source-agreements", "--disable-interactivity"])
        .creation_flags(NO_WINDOW)
        .output()
        .map_err(|e| format!("winget introuvable ({e})"))?;
    // Code non nul aussi quand une seule appli échoue : on le signale sans tout considérer raté.
    match out.status.code() {
        Some(0) => Ok(()),
        Some(code) => Err(format!("code {code:#x}, certaines applis n'ont peut-être pas été mises à jour")),
        None => Err("interrompu".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_winget_export() {
        let json = "{\n\t\"Packages\" : [\n\t\t{\n\t\t\t\"PackageIdentifier\" : \"Brave.Brave\"\n\t\t},\n\t\t{\n\t\t\t\"PackageIdentifier\" : \"CPUID.CPU-Z.MSI\"\n\t\t}\n\t]\n}";
        let ids = parse_export(json);
        assert!(ids.contains("Brave.Brave"));
        assert!(!ids.contains("CPUID.CPU-Z"), "pas de faux positif sur un préfixe");
    }

    #[test]
    fn ids_unique() {
        let ids: HashSet<_> = APPS.iter().map(|a| a.id).collect();
        assert_eq!(ids.len(), APPS.len());
    }
}
