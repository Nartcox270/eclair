//! Réglages de confidentialité et de publicité de Windows, sous forme de valeurs de registre.

use crate::ffi;

#[derive(Clone, Copy)]
pub enum Val {
    Dword(u32),
    Sz(&'static str),
}

pub struct Setting {
    pub group: &'static str,
    pub label: &'static str,
    /// HKEY_LOCAL_MACHINE : il faut les droits administrateur pour modifier.
    pub hklm: bool,
    key: &'static str,
    names: &'static [&'static str],
    /// None : supprimer la valeur pour revenir au comportement par défaut de Windows.
    on: Option<Val>,
    off: Val,
}

const CDM: &str = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
const CONF: &str = "Confidentialité";
const PUBS: &str = "Publicités et suggestions";

const fn hkcu(group: &'static str, label: &'static str, key: &'static str, names: &'static [&'static str]) -> Setting {
    Setting { group, label, hklm: false, key, names, on: Some(Val::Dword(1)), off: Val::Dword(0) }
}

// Stratégies machine : désactivées en posant la valeur, réactivées en la supprimant.
const fn policy(label: &'static str, key: &'static str, name: &'static [&'static str]) -> Setting {
    Setting { group: CONF, label, hklm: true, key, names: name, on: None, off: Val::Dword(0) }
}

pub const SETTINGS: &[Setting] = &[
    Setting {
        group: CONF,
        label: "Localisation",
        hklm: false,
        key: r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location",
        names: &["Value"],
        on: Some(Val::Sz("Allow")),
        off: Val::Sz("Deny"),
    },
    policy("Télémétrie", r"SOFTWARE\Policies\Microsoft\Windows\DataCollection", &["AllowTelemetry"]),
    policy("Programme d'amélioration de l'expérience utilisateur", r"SOFTWARE\Policies\Microsoft\SQMClient\Windows", &["CEIPEnable"]),
    policy("Télémétrie des applis", r"SOFTWARE\Policies\Microsoft\Windows\AppCompat", &["AITEnable"]),
    hkcu(CONF, "Suivi du lancement d'appli", r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", &["Start_TrackProgs"]),
    hkcu(CONF, "Améliorer l'écriture et la saisie", r"Software\Microsoft\Input\TIPC", &["Enabled"]),
    hkcu(CONF, "Personnaliser l'écriture et la saisie", r"Software\Microsoft\Personalization\Settings", &["AcceptedPrivacyPolicy"]),
    Setting {
        group: CONF,
        label: "Rapports d'erreurs",
        hklm: true,
        key: r"SOFTWARE\Microsoft\Windows\Windows Error Reporting",
        names: &["Disabled"],
        on: Some(Val::Dword(0)),
        off: Val::Dword(1),
    },
    hkcu(CONF, "Reconnaissance vocale en ligne", r"Software\Microsoft\Speech_OneCore\Settings\OnlineSpeechPrivacy", &["HasAccepted"]),
    hkcu(PUBS, "Expériences personnalisées", r"Software\Microsoft\Windows\CurrentVersion\Privacy", &["TailoredExperiencesWithDiagnosticDataEnabled"]),
    hkcu(PUBS, "ID de publicité", r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo", &["Enabled"]),
    hkcu(PUBS, "Contenu suggéré dans les paramètres", CDM, &["SubscribedContent-338393Enabled", "SubscribedContent-353694Enabled", "SubscribedContent-353696Enabled"]),
    hkcu(PUBS, "Suggestions dans le menu Démarrer", CDM, &["SystemPaneSuggestionsEnabled", "SubscribedContent-338388Enabled"]),
    hkcu(PUBS, "Notifications suggérées", r"Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\Windows.SystemToast.Suggested", &["Enabled"]),
    hkcu(PUBS, "Messages sur l'écran de verrouillage", CDM, &["RotatingLockScreenOverlayEnabled", "SubscribedContent-338387Enabled"]),
];

impl Setting {
    fn is_off(&self, name: &str) -> bool {
        match self.off {
            Val::Dword(v) => ffi::reg_get_dword(self.hklm, self.key, name) == Some(v),
            Val::Sz(s) => ffi::reg_get_sz(self.hklm, self.key, name).as_deref() == Some(s),
        }
    }

    /// Une valeur absente compte comme activée : c'est le défaut de Windows.
    pub fn is_on(&self) -> bool {
        !self.names.iter().all(|n| self.is_off(n))
    }

    pub fn set(&self, on: bool) -> bool {
        let val = if on { self.on } else { Some(self.off) };
        self.names.iter().all(|n| match val {
            None => ffi::reg_delete(self.hklm, self.key, n),
            Some(Val::Dword(v)) => ffi::reg_set_dword(self.hklm, self.key, n, v),
            Some(Val::Sz(s)) => ffi::reg_set_sz(self.hklm, self.key, n, s),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_round_trip() {
        // Clé de test à part : ne touche à aucun vrai réglage.
        let s = Setting {
            group: CONF,
            label: "test",
            hklm: false,
            key: r"Software\OptimiseurPcTest",
            names: &["A", "B"],
            on: None,
            off: Val::Sz("Deny"),
        };
        assert!(s.is_on(), "absente = activée");
        assert!(s.set(false) && !s.is_on());
        assert!(s.set(true) && s.is_on());
        let _ = std::process::Command::new("reg").args(["delete", r"HKCU\Software\OptimiseurPcTest", "/f"]).output();
    }
}
