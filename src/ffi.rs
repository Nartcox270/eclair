//! Liaisons vers le coeur C++ (cpp/optimizer.h) et petites enveloppes sûres.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct MemInfo {
    pub load_percent: u32,
    pub total_phys: u64,
    pub avail_phys: u64,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct CleanResult {
    pub bytes: u64,
    pub files: u32,
    pub failed: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RawProcess {
    pid: u32,
    priority: u32,
    cpu_time: u64,
    working_set: u64,
    name: [u16; 260],
}

pub struct ProcessInfo {
    pub pid: u32,
    /// 0 inactive, 1 sous la normale, 2 normale, 3 supérieure, 4 haute, 5 temps réel
    pub priority: u32,
    /// Temps CPU cumulé, en unités de 100 ns.
    pub cpu_time: u64,
    pub working_set: u64,
    pub name: String,
}

extern "C" {
    fn opt_is_admin() -> i32;
    fn opt_memory_info(out: *mut MemInfo);
    fn opt_cpu_usage() -> f64;
    fn opt_disk_space(root: *const u16, free: *mut u64, total: *mut u64) -> i32;
    fn opt_uptime_ms() -> u64;
    fn opt_clean_directory(path: *const u16, dry_run: i32, out: *mut CleanResult);
    fn opt_recycle_bin_size(bytes: *mut u64, items: *mut u64) -> i32;
    fn opt_empty_recycle_bin() -> i32;
    fn opt_trim_working_sets() -> u32;
    fn opt_flush_dns() -> i32;
    fn opt_get_power_plan() -> i32;
    fn opt_set_power_plan(plan: i32) -> i32;
    fn opt_list_processes(out: *mut RawProcess, cap: u32) -> u32;
    fn opt_set_priority(pid: u32, level: u32) -> i32;
    fn opt_foreground_pid() -> u32;
    fn opt_memory_lists(standby: *mut u64, free: *mut u64) -> i32;
    fn opt_purge_standby() -> i32;
    fn opt_timer_resolution(coarsest: *mut u32, finest: *mut u32, current: *mut u32) -> i32;
    fn opt_set_timer_resolution(resolution: u32, enable: i32) -> i32;
    fn opt_reg_get_dword(hklm: i32, key: *const u16, name: *const u16, out: *mut u32) -> i32;
    fn opt_reg_get_sz(hklm: i32, key: *const u16, name: *const u16, buf: *mut u16, buf_chars: u32) -> i32;
    fn opt_reg_set_dword(hklm: i32, key: *const u16, name: *const u16, value: u32) -> i32;
    fn opt_reg_set_sz(hklm: i32, key: *const u16, name: *const u16, value: *const u16) -> i32;
    fn opt_reg_delete(hklm: i32, key: *const u16, name: *const u16) -> i32;
    fn opt_reg_enum_value(hklm: i32, key: *const u16, index: u32, name: *mut u16, name_chars: u32, data: *mut u16, data_chars: u32) -> i32;
    fn opt_reg_get_binary(hklm: i32, key: *const u16, name: *const u16, buf: *mut u8, len: *mut u32) -> i32;
    fn opt_reg_set_binary(hklm: i32, key: *const u16, name: *const u16, data: *const u8, len: u32) -> i32;
    fn opt_mouse_accel() -> i32;
    fn opt_set_mouse_accel(on: i32) -> i32;
    fn opt_tray_start(tip: *const u16) -> i32;
    fn opt_tray_stop();
    fn opt_tray_notify(title: *const u16, text: *const u16);
    fn opt_tray_quit_requested() -> i32;
    fn opt_hide_main_window() -> i32;
    fn opt_fullscreen_foreground_pid() -> u32;
    fn opt_gpu_usage() -> f64;
}

fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
}

pub fn is_admin() -> bool {
    unsafe { opt_is_admin() != 0 }
}

pub fn memory_info() -> MemInfo {
    let mut m = MemInfo::default();
    unsafe { opt_memory_info(&mut m) };
    m
}

/// À appeler depuis un seul thread (garde l'état de la mesure précédente).
pub fn cpu_usage() -> f64 {
    unsafe { opt_cpu_usage() }
}

/// (libre, total) en octets.
pub fn disk_space(root: &str) -> Option<(u64, u64)> {
    let (mut free, mut total) = (0, 0);
    let ok = unsafe { opt_disk_space(wide(OsStr::new(root)).as_ptr(), &mut free, &mut total) };
    (ok != 0).then_some((free, total))
}

pub fn uptime_ms() -> u64 {
    unsafe { opt_uptime_ms() }
}

pub fn clean_directory(path: &Path, dry_run: bool) -> CleanResult {
    let mut r = CleanResult::default();
    unsafe { opt_clean_directory(wide(path.as_os_str()).as_ptr(), dry_run as i32, &mut r) };
    r
}

/// (octets, éléments)
pub fn recycle_bin_size() -> Option<(u64, u64)> {
    let (mut bytes, mut items) = (0, 0);
    let ok = unsafe { opt_recycle_bin_size(&mut bytes, &mut items) };
    (ok != 0).then_some((bytes, items))
}

pub fn empty_recycle_bin() -> bool {
    unsafe { opt_empty_recycle_bin() != 0 }
}

pub fn trim_working_sets() -> u32 {
    unsafe { opt_trim_working_sets() }
}

pub fn flush_dns() -> bool {
    unsafe { opt_flush_dns() != 0 }
}

pub fn get_power_plan() -> i32 {
    unsafe { opt_get_power_plan() }
}

pub fn set_power_plan(plan: i32) -> bool {
    unsafe { opt_set_power_plan(plan) != 0 }
}

pub fn list_processes() -> Vec<ProcessInfo> {
    let empty = RawProcess { pid: 0, priority: 0, cpu_time: 0, working_set: 0, name: [0; 260] };
    let mut buf = vec![empty; 4096];
    let n = unsafe { opt_list_processes(buf.as_mut_ptr(), buf.len() as u32) } as usize;
    buf[..n]
        .iter()
        .map(|p| {
            let len = p.name.iter().position(|&c| c == 0).unwrap_or(p.name.len());
            ProcessInfo {
                pid: p.pid,
                priority: p.priority,
                cpu_time: p.cpu_time,
                working_set: p.working_set,
                name: String::from_utf16_lossy(&p.name[..len]),
            }
        })
        .collect()
}

pub fn set_priority(pid: u32, level: u32) -> bool {
    unsafe { opt_set_priority(pid, level) != 0 }
}

pub fn foreground_pid() -> u32 {
    unsafe { opt_foreground_pid() }
}

/// (liste d'attente, mémoire libre) en octets ; None sans droits admin.
pub fn memory_lists() -> Option<(u64, u64)> {
    let (mut standby, mut free) = (0, 0);
    let ok = unsafe { opt_memory_lists(&mut standby, &mut free) };
    (ok != 0).then_some((standby, free))
}

pub fn purge_standby() -> bool {
    unsafe { opt_purge_standby() != 0 }
}

/// (la plus grossière, la plus fine, actuelle), en unités de 100 ns.
pub fn timer_resolution() -> Option<(u32, u32, u32)> {
    let (mut a, mut b, mut c) = (0, 0, 0);
    let ok = unsafe { opt_timer_resolution(&mut a, &mut b, &mut c) };
    (ok != 0).then_some((a, b, c))
}

pub fn set_timer_resolution(resolution: u32, enable: bool) -> bool {
    unsafe { opt_set_timer_resolution(resolution, enable as i32) != 0 }
}

fn w(s: &str) -> Vec<u16> {
    wide(OsStr::new(s))
}

pub fn reg_get_dword(hklm: bool, key: &str, name: &str) -> Option<u32> {
    let mut v = 0;
    let ok = unsafe { opt_reg_get_dword(hklm as i32, w(key).as_ptr(), w(name).as_ptr(), &mut v) };
    (ok != 0).then_some(v)
}

pub fn reg_get_sz(hklm: bool, key: &str, name: &str) -> Option<String> {
    let mut buf = [0u16; 256];
    let ok = unsafe {
        opt_reg_get_sz(hklm as i32, w(key).as_ptr(), w(name).as_ptr(), buf.as_mut_ptr(), buf.len() as u32)
    };
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    (ok != 0).then(|| String::from_utf16_lossy(&buf[..len]))
}

pub fn reg_set_dword(hklm: bool, key: &str, name: &str, value: u32) -> bool {
    unsafe { opt_reg_set_dword(hklm as i32, w(key).as_ptr(), w(name).as_ptr(), value) != 0 }
}

pub fn reg_set_sz(hklm: bool, key: &str, name: &str, value: &str) -> bool {
    unsafe { opt_reg_set_sz(hklm as i32, w(key).as_ptr(), w(name).as_ptr(), w(value).as_ptr()) != 0 }
}

pub fn reg_delete(hklm: bool, key: &str, name: &str) -> bool {
    unsafe { opt_reg_delete(hklm as i32, w(key).as_ptr(), w(name).as_ptr()) != 0 }
}

fn from_wide(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// Toutes les valeurs d'une clé : (nom, texte ou vide si ce n'est pas une chaîne).
pub fn reg_values(hklm: bool, key: &str) -> Vec<(String, String)> {
    let key = w(key);
    let mut out = Vec::new();
    let (mut name, mut data) = (vec![0u16; 512], vec![0u16; 2048]);
    for i in 0.. {
        let ok = unsafe {
            opt_reg_enum_value(hklm as i32, key.as_ptr(), i, name.as_mut_ptr(), name.len() as u32, data.as_mut_ptr(), data.len() as u32)
        };
        if ok == 0 {
            break;
        }
        out.push((from_wide(&name), from_wide(&data)));
    }
    out
}

pub fn reg_get_binary(hklm: bool, key: &str, name: &str) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; 64];
    let mut len = buf.len() as u32;
    let ok = unsafe { opt_reg_get_binary(hklm as i32, w(key).as_ptr(), w(name).as_ptr(), buf.as_mut_ptr(), &mut len) };
    (ok != 0).then(|| {
        buf.truncate(len as usize);
        buf
    })
}

pub fn reg_set_binary(hklm: bool, key: &str, name: &str, data: &[u8]) -> bool {
    unsafe { opt_reg_set_binary(hklm as i32, w(key).as_ptr(), w(name).as_ptr(), data.as_ptr(), data.len() as u32) != 0 }
}

pub fn mouse_accel() -> bool {
    unsafe { opt_mouse_accel() != 0 }
}

pub fn set_mouse_accel(on: bool) -> bool {
    unsafe { opt_set_mouse_accel(on as i32) != 0 }
}

pub fn tray_start(tip: &str) -> bool {
    unsafe { opt_tray_start(w(tip).as_ptr()) != 0 }
}

pub fn tray_stop() {
    unsafe { opt_tray_stop() }
}

pub fn tray_notify(title: &str, text: &str) {
    unsafe { opt_tray_notify(w(title).as_ptr(), w(text).as_ptr()) }
}

/// "Quitter" choisi dans le menu de l'icône : la fermeture doit vraiment quitter.
pub fn tray_quit_requested() -> bool {
    unsafe { opt_tray_quit_requested() != 0 }
}

pub fn hide_main_window() -> bool {
    unsafe { opt_hide_main_window() != 0 }
}

pub fn fullscreen_foreground_pid() -> u32 {
    unsafe { opt_fullscreen_foreground_pid() }
}

/// None si le compteur GPU n'existe pas sur ce PC. À appeler depuis un seul thread.
pub fn gpu_usage() -> Option<f64> {
    let v = unsafe { opt_gpu_usage() };
    (v >= 0.0).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_counter() {
        let first = gpu_usage();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let second = gpu_usage();
        println!("GPU : {first:?} puis {second:?}");
        assert!(second.is_some_and(|g| (0.0..=100.0).contains(&g)));
    }

    #[test]
    fn system_info() {
        let m = memory_info();
        assert!(m.total_phys > 0 && m.avail_phys <= m.total_phys);
        let (free, total) = disk_space("C:\\").unwrap();
        assert!(free <= total);
        assert!(uptime_ms() > 0);
    }

    #[test]
    fn clean_directory_dry_run_then_delete() {
        let dir = std::env::temp_dir().join("optimiseur-pc-test");
        let sub = dir.join("sous-dossier");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(dir.join("a.tmp"), [0u8; 100]).unwrap();
        std::fs::write(sub.join("b.tmp"), [0u8; 50]).unwrap();

        let r = clean_directory(&dir, true);
        assert_eq!((r.files, r.bytes), (2, 150));
        assert!(sub.join("b.tmp").exists(), "l'analyse ne doit rien supprimer");

        let r = clean_directory(&dir, false);
        assert_eq!((r.files, r.bytes, r.failed), (2, 150, 0));
        assert!(!sub.exists());
        std::fs::remove_dir(&dir).unwrap();
    }
}
