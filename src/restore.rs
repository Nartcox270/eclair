//! Point de restauration du système, pour pouvoir annuler les réglages d'Éclair.

use std::os::windows::process::CommandExt;
use std::process::Command;

pub enum Outcome {
    Created,
    /// Windows refuse plus d'un point de restauration par 24 h : le précédent suffit.
    Recent,
    Failed,
}

/// Crée un point "Éclair". Long (jusqu'à une minute) : à lancer hors du thread de l'interface.
pub fn create() -> Outcome {
    // Checkpoint-Computer avertit (sans échouer) quand un point existe depuis moins de 24 h.
    let script = "$w = $null; \
        Checkpoint-Computer -Description 'Eclair' -RestorePointType MODIFY_SETTINGS -WarningVariable w -WarningAction SilentlyContinue -ErrorAction Stop; \
        if ($w) { exit 2 }";
    let status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .status();
    match status.map(|s| s.code()) {
        Ok(Some(0)) => Outcome::Created,
        Ok(Some(2)) => Outcome::Recent,
        _ => Outcome::Failed,
    }
}

pub fn message(o: &Outcome) -> &'static str {
    match o {
        Outcome::Created => "Point de restauration créé : tu pourras annuler les réglages d'Éclair.",
        Outcome::Recent => "Un point de restauration de moins de 24 h existe déjà : il servira pour annuler.",
        Outcome::Failed => "Point de restauration impossible (admin requis, ou protection du système désactivée).",
    }
}

/// Ouvre l'outil "Restauration du système" de Windows.
pub fn open_system_restore() -> bool {
    Command::new("rstrui.exe").spawn().is_ok()
}
