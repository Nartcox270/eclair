// Coeur natif de l'optimiseur : appels Win32 exposés en C pour Rust.
#pragma once
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    uint32_t load_percent;
    uint64_t total_phys;
    uint64_t avail_phys;
} OptMemInfo;

typedef struct {
    uint64_t bytes;
    uint32_t files;
    uint32_t failed;
} OptCleanResult;

// Infos système
int  opt_is_admin(void);
void opt_memory_info(OptMemInfo* out);
// Pourcentage CPU depuis l'appel précédent (le premier appel renvoie 0).
double opt_cpu_usage(void);
int  opt_disk_space(const wchar_t* root, uint64_t* free_bytes, uint64_t* total_bytes);
uint64_t opt_uptime_ms(void);

// Nettoyage. dry_run != 0 : ne fait que mesurer.
void opt_clean_directory(const wchar_t* path, int dry_run, OptCleanResult* out);
int  opt_recycle_bin_size(uint64_t* bytes, uint64_t* items);
int  opt_empty_recycle_bin(void);

// Performance
uint32_t opt_trim_working_sets(void);
int  opt_flush_dns(void);
// 0 = équilibré, 1 = performances élevées, 2 = économie d'énergie,
// 3 = performances max (plan créé par l'outil au premier usage), -1 = autre
int  opt_get_power_plan(void);
int  opt_set_power_plan(int plan);

// Processus. priority : 0 inactive, 1 sous la normale, 2 normale, 3 supérieure, 4 haute, 5 temps réel.
typedef struct {
    uint32_t pid;
    uint32_t priority;
    uint64_t cpu_time;     // temps CPU cumulé, en unités de 100 ns
    uint64_t working_set;  // octets
    wchar_t  name[260];
} OptProcess;

uint32_t opt_list_processes(OptProcess* out, uint32_t cap);
int  opt_set_priority(uint32_t pid, uint32_t level);  // level 0..4 (temps réel refusé)
uint32_t opt_foreground_pid(void);

// Liste d'attente (standby) de la mémoire et minuteur système. Admin requis pour la liste.
int  opt_memory_lists(uint64_t* standby, uint64_t* free_bytes);
int  opt_purge_standby(void);
// Résolutions en unités de 100 ns (5000 = 0,5 ms).
int  opt_timer_resolution(uint32_t* coarsest, uint32_t* finest, uint32_t* current);
int  opt_set_timer_resolution(uint32_t resolution, int enable);

// Registre. hklm != 0 : HKEY_LOCAL_MACHINE (admin requis pour écrire), sinon HKEY_CURRENT_USER.
// Les fonctions renvoient 1 en cas de succès (lecture : la valeur existe).
int  opt_reg_get_dword(int hklm, const wchar_t* key, const wchar_t* name, uint32_t* out);
int  opt_reg_get_sz(int hklm, const wchar_t* key, const wchar_t* name, wchar_t* buf, uint32_t buf_chars);
int  opt_reg_set_dword(int hklm, const wchar_t* key, const wchar_t* name, uint32_t value);
int  opt_reg_set_sz(int hklm, const wchar_t* key, const wchar_t* name, const wchar_t* value);
// Supprime la valeur ; une valeur déjà absente compte comme un succès.
int  opt_reg_delete(int hklm, const wchar_t* key, const wchar_t* name);
// Valeur n° index d'une clé : nom, et texte si c'est une chaîne (sinon data vide). 0 = plus de valeur.
int  opt_reg_enum_value(int hklm, const wchar_t* key, uint32_t index, wchar_t* name, uint32_t name_chars,
                        wchar_t* data, uint32_t data_chars);
// *len : taille du tampon en entrée, octets lus en sortie.
int  opt_reg_get_binary(int hklm, const wchar_t* key, const wchar_t* name, uint8_t* buf, uint32_t* len);
int  opt_reg_set_binary(int hklm, const wchar_t* key, const wchar_t* name, const uint8_t* data, uint32_t len);

// Accélération de la souris : 1 = activée.
int  opt_mouse_accel(void);
int  opt_set_mouse_accel(int on);

// Icône dans la zone de notification (thread à part). Clic : rouvre la fenêtre ; menu : Ouvrir / Quitter.
int  opt_tray_start(const wchar_t* tip);
void opt_tray_stop(void);
void opt_tray_notify(const wchar_t* title, const wchar_t* text);
int  opt_tray_quit_requested(void);
int  opt_hide_main_window(void);

// Jeu en plein écran au premier plan : son pid, sinon 0.
uint32_t opt_fullscreen_foreground_pid(void);
// Utilisation 3D de la carte graphique en %, -1 si indisponible.
double opt_gpu_usage(void);

#ifdef __cplusplus
}
#endif
