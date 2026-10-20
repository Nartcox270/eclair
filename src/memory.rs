//! Purge automatique de la liste d'attente mémoire et minuteur fin, façon ISLC.
//! Tourne dans son propre thread pour continuer quand la fenêtre est réduite (en jeu).

use crate::ffi;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

const MB: u64 = 1024 * 1024;

#[derive(Clone)]
pub struct Settings {
    pub auto_purge: Arc<AtomicBool>,
    /// Purger quand la liste d'attente dépasse ce seuil…
    pub list_mb: Arc<AtomicU32>,
    /// …et que la mémoire libre passe sous celui-ci.
    pub free_mb: Arc<AtomicU32>,
    pub fine_timer: Arc<AtomicBool>,
}

pub struct StandbyCleaner {
    pub settings: Settings,
    pub events: Receiver<String>,
}

impl StandbyCleaner {
    pub fn start() -> Self {
        let settings = Settings {
            auto_purge: Arc::new(AtomicBool::new(false)),
            list_mb: Arc::new(AtomicU32::new(1024)),
            free_mb: Arc::new(AtomicU32::new(1024)),
            fine_timer: Arc::new(AtomicBool::new(false)),
        };
        let (tx, events) = channel();
        let s = settings.clone();
        std::thread::spawn(move || run(&s, &tx));
        Self { settings, events }
    }
}

/// Vrai quand il faut purger : liste d'attente pleine ET peu de mémoire libre.
fn should_purge(standby: u64, free: u64, list_mb: u32, free_mb: u32) -> bool {
    standby >= list_mb as u64 * MB && free < free_mb as u64 * MB
}

/// Purge et renvoie le message pour le journal.
pub fn purge() -> String {
    let before = ffi::memory_lists().map_or(0, |(s, _)| s);
    if ffi::purge_standby() {
        let after = ffi::memory_lists().map_or(0, |(s, _)| s);
        format!("Liste d'attente purgée ({} Mo rendus libres).", before.saturating_sub(after) / MB)
    } else {
        "Impossible de purger la liste d'attente (admin requis).".into()
    }
}

fn run(s: &Settings, tx: &Sender<String>) {
    let mut timer_on = false;
    loop {
        if s.auto_purge.load(Ordering::Relaxed) {
            if let Some((standby, free)) = ffi::memory_lists() {
                if should_purge(standby, free, s.list_mb.load(Ordering::Relaxed), s.free_mb.load(Ordering::Relaxed)) {
                    let _ = tx.send(purge());
                }
            }
        }
        // La résolution ne tient que tant que l'optimiseur tourne : Windows la relâche à sa fermeture.
        let want = s.fine_timer.load(Ordering::Relaxed);
        if want != timer_on {
            let finest = ffi::timer_resolution().map_or(5000, |(_, f, _)| f);
            if ffi::set_timer_resolution(finest, want) {
                timer_on = want;
                let _ = tx.send(if want {
                    format!("Minuteur système à {:.2} ms.", finest as f64 / 10_000.0)
                } else {
                    "Minuteur système rendu à Windows.".into()
                });
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purge_thresholds() {
        assert!(should_purge(2048 * MB, 512 * MB, 1024, 1024));
        assert!(!should_purge(512 * MB, 512 * MB, 1024, 1024), "liste trop petite");
        assert!(!should_purge(2048 * MB, 4096 * MB, 1024, 1024), "assez de mémoire libre");
    }

    #[test]
    fn timer_resolution_readable() {
        let (coarsest, finest, current) = ffi::timer_resolution().unwrap();
        assert!(finest <= current && current <= coarsest);
    }
}
