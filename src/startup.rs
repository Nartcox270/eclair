//! Programmes lancés au démarrage (clés Run), activés/désactivés exactement comme
//! le Gestionnaire des tâches : via StartupApproved, sans supprimer l'entrée.

use crate::ffi;

pub struct Entry {
    pub name: String,
    pub command: String,
    /// Pour tous les utilisateurs (HKLM) : admin requis pour modifier.
    pub hklm: bool,
    approved_key: &'static str,
    pub enabled: bool,
}

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const APPROVED32: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32";

/// Premier octet pair (02, 06) ou valeur absente = activé ; impair (03, 07) = désactivé.
fn is_enabled(flag: Option<&[u8]>) -> bool {
    flag.and_then(|b| b.first()).map_or(true, |b| b & 1 == 0)
}

pub fn list() -> Vec<Entry> {
    let mut out = Vec::new();
    for (hklm, run, approved_key) in [(false, RUN, APPROVED), (true, RUN, APPROVED), (true, RUN32, APPROVED32)] {
        for (name, command) in ffi::reg_values(hklm, run) {
            let enabled = is_enabled(ffi::reg_get_binary(hklm, approved_key, &name).as_deref());
            out.push(Entry { name, command, hklm, approved_key, enabled });
        }
    }
    out
}

pub fn set_enabled(e: &Entry, on: bool) -> bool {
    let mut flag = [0u8; 12];
    flag[0] = if on { 2 } else { 3 };
    ffi::reg_set_binary(e.hklm, e.approved_key, &e.name, &flag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags() {
        assert!(is_enabled(None));
        assert!(is_enabled(Some(&[2, 0, 0])));
        assert!(is_enabled(Some(&[6, 0, 0])));
        assert!(!is_enabled(Some(&[3, 0, 0])));
    }

    #[test]
    fn binary_round_trip() {
        let key = r"Software\OptimiseurPcTestStartup";
        assert!(ffi::reg_set_binary(false, key, "x", &[3, 0, 1, 2]));
        assert_eq!(ffi::reg_get_binary(false, key, "x"), Some(vec![3, 0, 1, 2]));
        assert!(ffi::reg_set_dword(false, key, "y", 7));
        let names: Vec<String> = ffi::reg_values(false, key).into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, ["x", "y"]);
        let _ = std::process::Command::new("reg").args(["delete", r"HKCU\Software\OptimiseurPcTestStartup", "/f"]).output();
    }
}
