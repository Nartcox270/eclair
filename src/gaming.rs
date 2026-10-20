//! Réglages Windows pour le jeu dont l'effet est documenté (Microsoft) ou mesurable.
//! Case cochée = optimisation active.

use crate::ffi;

pub struct Tweak {
    pub label: &'static str,
    pub help: &'static str,
    /// Réglage machine : droits administrateur requis.
    pub admin: bool,
    pub reboot: bool,
    /// Compté dans le score de santé (pas les réglages qui baissent la sécurité).
    pub recommended: bool,
    pub get: fn() -> bool,
    pub set: fn(bool) -> bool,
}

const GAME_BAR: &str = r"Software\Microsoft\GameBar";
const GAME_DVR: &str = r"Software\Microsoft\Windows\CurrentVersion\GameDVR";
const GAME_CONFIG: &str = r"System\GameConfigStore";
const GRAPHICS: &str = r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers";
const DIRECTX: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";
const HVCI: &str = r"SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity";

pub const TWEAKS: &[Tweak] = &[
    Tweak {
        label: "Optimisations pour les jeux fenêtrés",
        help: "Jeux DirectX 10/11 en fenêtré ou sans bordure : modèle « flip », latence plus faible, VRR et Auto HDR possibles.",
        admin: false,
        reboot: false,
        recommended: true,
        get: || windowed_get(),
        set: |on| windowed_set(on),
    },
    Tweak {
        label: "Planification GPU matérielle (HAGS)",
        help: "La carte graphique gère sa propre mémoire vidéo. Requise pour la génération d'images DLSS 3 / FSR 3.",
        admin: true,
        reboot: true,
        recommended: true,
        get: || ffi::reg_get_dword(true, GRAPHICS, "HwSchMode") == Some(2),
        set: |on| ffi::reg_set_dword(true, GRAPHICS, "HwSchMode", if on { 2 } else { 1 }),
    },
    Tweak {
        label: "Couper l'enregistrement Xbox en arrière-plan",
        help: "La Xbox Game Bar n'enregistre plus le jeu en continu : moins de charge GPU et disque.",
        admin: false,
        reboot: false,
        recommended: true,
        get: || ffi::reg_get_dword(false, GAME_CONFIG, "GameDVR_Enabled") == Some(0),
        set: |on| {
            ffi::reg_set_dword(false, GAME_CONFIG, "GameDVR_Enabled", !on as u32)
                && ffi::reg_set_dword(false, GAME_DVR, "AppCaptureEnabled", !on as u32)
        },
    },
    Tweak {
        label: "Mode Jeu de Windows",
        help: "Windows donne la priorité au jeu et repousse les mises à jour pendant la partie.",
        admin: false,
        reboot: false,
        recommended: true,
        get: || ffi::reg_get_dword(false, GAME_BAR, "AutoGameModeEnabled") != Some(0),
        set: |on| ffi::reg_set_dword(false, GAME_BAR, "AutoGameModeEnabled", on as u32),
    },
    Tweak {
        label: "Désactiver l'accélération de la souris",
        help: "« Améliorer la précision du pointeur » coupé : un même geste donne toujours le même mouvement (visée).",
        admin: false,
        reboot: false,
        recommended: true,
        get: || !ffi::mouse_accel(),
        set: |on| ffi::set_mouse_accel(!on),
    },
    Tweak {
        label: "Désactiver l'intégrité de la mémoire (⚠ sécurité)",
        help: "Microsoft l'indique pour gagner quelques % en jeu, mais le PC devient plus vulnérable aux pilotes malveillants. À réactiver après le jeu.",
        admin: true,
        reboot: true,
        recommended: false,
        get: || ffi::reg_get_dword(true, HVCI, "Enabled") == Some(0),
        set: |on| ffi::reg_set_dword(true, HVCI, "Enabled", !on as u32),
    },
];

// "DirectXUserGlobalSettings" contient plusieurs réglages "clé=valeur;" : on ne touche qu'au nôtre.
const SWAP_KEY: &str = "SwapEffectUpgradeEnable";

fn windowed_get() -> bool {
    ffi::reg_get_sz(false, DIRECTX, "DirectXUserGlobalSettings")
        .is_some_and(|s| s.split(';').any(|t| t == format!("{SWAP_KEY}=1")))
}

fn windowed_set(on: bool) -> bool {
    let current = ffi::reg_get_sz(false, DIRECTX, "DirectXUserGlobalSettings").unwrap_or_default();
    ffi::reg_set_sz(false, DIRECTX, "DirectXUserGlobalSettings", &with_setting(&current, SWAP_KEY, on as u32))
}

/// Remplace ou ajoute "clé=valeur;" en gardant les autres réglages.
fn with_setting(s: &str, key: &str, value: u32) -> String {
    let mut out: String = s
        .split(';')
        .filter(|t| !t.is_empty() && !t.starts_with(&format!("{key}=")))
        .map(|t| format!("{t};"))
        .collect();
    out += &format!("{key}={value};");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directx_settings_keep_others() {
        assert_eq!(with_setting("", SWAP_KEY, 1), "SwapEffectUpgradeEnable=1;");
        assert_eq!(
            with_setting("VRROptimizeEnable=0;SwapEffectUpgradeEnable=0;", SWAP_KEY, 1),
            "VRROptimizeEnable=0;SwapEffectUpgradeEnable=1;"
        );
    }
}
