#ifndef UNICODE
#define UNICODE
#endif
#define WIN32_LEAN_AND_MEAN
#define PSAPI_VERSION 2
#include <windows.h>
#include <psapi.h>
#include <shellapi.h>
#include <powrprof.h>
#include <tlhelp32.h>
#include <pdh.h>

#include <atomic>
#include <string>
#include <vector>

#include "optimizer.h"

// GUID des plans d'alimentation standard de Windows.
static const GUID kBalanced  = {0x381b4222, 0xf694, 0x41f0, {0x96, 0x85, 0xff, 0x5b, 0xb2, 0x60, 0xdf, 0x2e}};
static const GUID kHighPerf  = {0x8c5e7fda, 0xe8bf, 0x4a96, {0x9a, 0x85, 0xa6, 0xe2, 0x3a, 0x8c, 0x63, 0x5c}};
static const GUID kPowerSave = {0xa1841308, 0x3541, 0x4fab, {0xbc, 0x81, 0xf7, 0x15, 0x56, 0xf2, 0x0b, 0x4a}};

static uint64_t to_u64(FILETIME ft) {
    return (uint64_t(ft.dwHighDateTime) << 32) | ft.dwLowDateTime;
}

extern "C" int opt_is_admin(void) {
    HANDLE token = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) return 0;
    TOKEN_ELEVATION elev{};
    DWORD len = 0;
    BOOL ok = GetTokenInformation(token, TokenElevation, &elev, sizeof(elev), &len);
    CloseHandle(token);
    return ok && elev.TokenIsElevated ? 1 : 0;
}

extern "C" void opt_memory_info(OptMemInfo* out) {
    MEMORYSTATUSEX ms{};
    ms.dwLength = sizeof(ms);
    GlobalMemoryStatusEx(&ms);
    out->load_percent = ms.dwMemoryLoad;
    out->total_phys = ms.ullTotalPhys;
    out->avail_phys = ms.ullAvailPhys;
}

extern "C" double opt_cpu_usage(void) {
    static uint64_t prev_idle = 0, prev_total = 0;
    FILETIME idle, kernel, user;
    if (!GetSystemTimes(&idle, &kernel, &user)) return 0.0;
    uint64_t i = to_u64(idle);
    uint64_t t = to_u64(kernel) + to_u64(user);  // le temps noyau inclut l'idle
    uint64_t di = i - prev_idle, dt = t - prev_total;
    bool first = prev_total == 0;
    prev_idle = i;
    prev_total = t;
    if (first || dt == 0) return 0.0;
    return 100.0 * double(dt - di) / double(dt);
}

extern "C" int opt_disk_space(const wchar_t* root, uint64_t* free_bytes, uint64_t* total_bytes) {
    ULARGE_INTEGER avail, total, free;
    if (!GetDiskFreeSpaceExW(root, &avail, &total, &free)) return 0;
    *free_bytes = free.QuadPart;
    *total_bytes = total.QuadPart;
    return 1;
}

extern "C" uint64_t opt_uptime_ms(void) {
    return GetTickCount64();
}

// Supprime récursivement le contenu de `dir` (pas le dossier lui-même).
// Les fichiers verrouillés sont ignorés ; les jonctions ne sont jamais suivies.
static void clean_dir(const std::wstring& dir, bool dry_run, OptCleanResult* r) {
    WIN32_FIND_DATAW fd;
    HANDLE h = FindFirstFileExW((dir + L"\\*").c_str(), FindExInfoBasic, &fd,
                                FindExSearchNameMatch, nullptr, FIND_FIRST_EX_LARGE_FETCH);
    if (h == INVALID_HANDLE_VALUE) return;
    do {
        if (wcscmp(fd.cFileName, L".") == 0 || wcscmp(fd.cFileName, L"..") == 0) continue;
        std::wstring path = dir + L"\\" + fd.cFileName;
        bool is_dir = fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY;
        bool is_link = fd.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT;

        if (is_dir) {
            if (!is_link) clean_dir(path, dry_run, r);
            // Retire le lien lui-même, ou le dossier s'il est maintenant vide.
            if (!dry_run) RemoveDirectoryW(path.c_str());
            continue;
        }

        uint64_t size = (uint64_t(fd.nFileSizeHigh) << 32) | fd.nFileSizeLow;
        if (dry_run) {
            r->bytes += size;
            r->files++;
            continue;
        }
        if (fd.dwFileAttributes & FILE_ATTRIBUTE_READONLY)
            SetFileAttributesW(path.c_str(), FILE_ATTRIBUTE_NORMAL);
        if (DeleteFileW(path.c_str())) {
            r->bytes += size;
            r->files++;
        } else {
            r->failed++;
        }
    } while (FindNextFileW(h, &fd));
    FindClose(h);
}

extern "C" void opt_clean_directory(const wchar_t* path, int dry_run, OptCleanResult* out) {
    *out = OptCleanResult{};
    if (!path || !*path) return;
    clean_dir(path, dry_run != 0, out);
}

extern "C" int opt_recycle_bin_size(uint64_t* bytes, uint64_t* items) {
    SHQUERYRBINFO info{};
    info.cbSize = sizeof(info);
    if (FAILED(SHQueryRecycleBinW(nullptr, &info))) return 0;
    *bytes = uint64_t(info.i64Size);
    *items = uint64_t(info.i64NumItems);
    return 1;
}

extern "C" int opt_empty_recycle_bin(void) {
    HRESULT hr = SHEmptyRecycleBinW(nullptr, nullptr,
                                    SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND);
    // E_UNEXPECTED est renvoyé quand la corbeille est déjà vide.
    return SUCCEEDED(hr) || hr == E_UNEXPECTED ? 1 : 0;
}

extern "C" uint32_t opt_trim_working_sets(void) {
    DWORD pids[4096], needed = 0;
    if (!EnumProcesses(pids, sizeof(pids), &needed)) return 0;
    uint32_t trimmed = 0;
    for (DWORD i = 0; i < needed / sizeof(DWORD); i++) {
        if (pids[i] == 0) continue;
        HANDLE p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA, FALSE, pids[i]);
        if (!p) continue;
        if (EmptyWorkingSet(p)) trimmed++;
        CloseHandle(p);
    }
    return trimmed;
}

extern "C" int opt_flush_dns(void) {
    // DnsFlushResolverCache n'est pas dans les en-têtes publics : chargement dynamique.
    HMODULE dns = LoadLibraryW(L"dnsapi.dll");
    if (!dns) return 0;
    using FlushFn = BOOL(WINAPI*)();
    auto flush = reinterpret_cast<FlushFn>(GetProcAddress(dns, "DnsFlushResolverCache"));
    BOOL ok = flush ? flush() : FALSE;
    FreeLibrary(dns);
    return ok ? 1 : 0;
}

// Plan "performances max" créé par l'outil : aucun coeur mis en veille (core parking)
// et fréquence minimale à 100 %, sur secteur seulement pour ne pas vider la batterie.
static const wchar_t kMaxName[] = L"Optimiseur - Performances max";
static const GUID kSubProcessor = {0x54533251, 0x82be, 0x4824, {0x96, 0xc1, 0x47, 0xb6, 0x0b, 0x74, 0x0d, 0x00}};
static const GUID kCoreParkMin  = {0x0cc5b647, 0xc1df, 0x4637, {0x89, 0x1a, 0xde, 0xc3, 0x5c, 0x31, 0x85, 0x83}};
static const GUID kThrottleMin  = {0x893dee8e, 0x2bef, 0x41e0, {0x89, 0xc6, 0xb5, 0x5d, 0x09, 0x29, 0x96, 0x4c}};

static bool find_max_plan(GUID* out) {
    for (ULONG i = 0;; i++) {
        GUID g;
        DWORD size = sizeof(g);
        if (PowerEnumerate(nullptr, nullptr, nullptr, ACCESS_SCHEME, i, (UCHAR*)&g, &size) != ERROR_SUCCESS)
            return false;
        wchar_t name[128];
        DWORD name_size = sizeof(name);
        if (PowerReadFriendlyName(nullptr, &g, nullptr, nullptr, (UCHAR*)name, &name_size) == ERROR_SUCCESS &&
            wcscmp(name, kMaxName) == 0) {
            *out = g;
            return true;
        }
    }
}

static bool create_max_plan(GUID* out) {
    GUID* g = nullptr;
    // Le plan "performances élevées" n'existe pas sur certains portables : repli sur "équilibré".
    if (PowerDuplicateScheme(nullptr, &kHighPerf, &g) != ERROR_SUCCESS &&
        PowerDuplicateScheme(nullptr, &kBalanced, &g) != ERROR_SUCCESS)
        return false;
    *out = *g;
    LocalFree(g);
    PowerWriteFriendlyName(nullptr, out, nullptr, nullptr, (UCHAR*)kMaxName, sizeof(kMaxName));
    PowerWriteACValueIndex(nullptr, out, &kSubProcessor, &kCoreParkMin, 100);
    PowerWriteACValueIndex(nullptr, out, &kSubProcessor, &kThrottleMin, 100);
    return true;
}

extern "C" int opt_get_power_plan(void) {
    GUID* active = nullptr;
    if (PowerGetActiveScheme(nullptr, &active) != ERROR_SUCCESS) return -1;
    GUID max;
    int plan = -1;
    if (IsEqualGUID(*active, kBalanced)) plan = 0;
    else if (IsEqualGUID(*active, kHighPerf)) plan = 1;
    else if (IsEqualGUID(*active, kPowerSave)) plan = 2;
    else if (find_max_plan(&max) && IsEqualGUID(*active, max)) plan = 3;
    LocalFree(active);
    return plan;
}

extern "C" int opt_set_power_plan(int plan) {
    GUID max;
    if (plan == 3 && !find_max_plan(&max) && !create_max_plan(&max)) return 0;
    const GUID* g = plan == 3 ? &max : plan == 1 ? &kHighPerf : plan == 2 ? &kPowerSave : &kBalanced;
    return PowerSetActiveScheme(nullptr, g) == ERROR_SUCCESS ? 1 : 0;
}

static const DWORD kPriorities[] = {IDLE_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
                                    ABOVE_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS};

extern "C" uint32_t opt_list_processes(OptProcess* out, uint32_t cap) {
    // Peut échouer (ERROR_BAD_LENGTH) quand des processus naissent pendant la capture : on réessaie.
    HANDLE snap = INVALID_HANDLE_VALUE;
    for (int i = 0; i < 5 && snap == INVALID_HANDLE_VALUE; i++)
        snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (snap == INVALID_HANDLE_VALUE) return 0;
    PROCESSENTRY32W pe{};
    pe.dwSize = sizeof(pe);
    uint32_t n = 0;
    for (BOOL ok = Process32FirstW(snap, &pe); ok && n < cap; ok = Process32NextW(snap, &pe)) {
        // Les processus inaccessibles (système, protégés) sont ignorés : on ne pourrait pas les régler.
        HANDLE p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pe.th32ProcessID);
        if (!p) continue;
        OptProcess& o = out[n++];
        o = OptProcess{};
        o.pid = pe.th32ProcessID;
        wcsncpy_s(o.name, pe.szExeFile, _TRUNCATE);
        FILETIME c, e, k, u;
        if (GetProcessTimes(p, &c, &e, &k, &u)) o.cpu_time = to_u64(k) + to_u64(u);
        PROCESS_MEMORY_COUNTERS mc{};
        if (GetProcessMemoryInfo(p, &mc, sizeof(mc))) o.working_set = mc.WorkingSetSize;
        DWORD cls = GetPriorityClass(p);
        o.priority = 2;
        for (uint32_t i = 0; i < 6; i++)
            if (kPriorities[i] == cls) o.priority = i;
        CloseHandle(p);
    }
    CloseHandle(snap);
    return n;
}

extern "C" int opt_set_priority(uint32_t pid, uint32_t level) {
    if (level > 4) return 0;  // pas de "temps réel" : peut figer tout le PC
    HANDLE p = OpenProcess(PROCESS_SET_INFORMATION, FALSE, pid);
    if (!p) return 0;
    BOOL ok = SetPriorityClass(p, kPriorities[level]);
    CloseHandle(p);
    return ok ? 1 : 0;
}

extern "C" uint32_t opt_foreground_pid(void) {
    DWORD pid = 0;
    GetWindowThreadProcessId(GetForegroundWindow(), &pid);
    return pid;
}

static HKEY root(int hklm) {
    return hklm ? HKEY_LOCAL_MACHINE : HKEY_CURRENT_USER;
}

extern "C" int opt_reg_get_dword(int hklm, const wchar_t* key, const wchar_t* name, uint32_t* out) {
    DWORD size = sizeof(*out);
    return RegGetValueW(root(hklm), key, name, RRF_RT_REG_DWORD, nullptr, out, &size) == ERROR_SUCCESS;
}

extern "C" int opt_reg_get_sz(int hklm, const wchar_t* key, const wchar_t* name, wchar_t* buf, uint32_t buf_chars) {
    DWORD size = buf_chars * sizeof(wchar_t);
    return RegGetValueW(root(hklm), key, name, RRF_RT_REG_SZ, nullptr, buf, &size) == ERROR_SUCCESS;
}

extern "C" int opt_reg_set_dword(int hklm, const wchar_t* key, const wchar_t* name, uint32_t value) {
    DWORD v = value;
    return RegSetKeyValueW(root(hklm), key, name, REG_DWORD, &v, sizeof(v)) == ERROR_SUCCESS;
}

extern "C" int opt_reg_set_sz(int hklm, const wchar_t* key, const wchar_t* name, const wchar_t* value) {
    DWORD size = DWORD((wcslen(value) + 1) * sizeof(wchar_t));
    return RegSetKeyValueW(root(hklm), key, name, REG_SZ, value, size) == ERROR_SUCCESS;
}

extern "C" int opt_reg_delete(int hklm, const wchar_t* key, const wchar_t* name) {
    LSTATUS s = RegDeleteKeyValueW(root(hklm), key, name);
    return s == ERROR_SUCCESS || s == ERROR_FILE_NOT_FOUND;
}

// --- Liste d'attente mémoire et minuteur (idées d'ISLC) ---
// Fonctions non documentées de ntdll : chargées dynamiquement.
using NtQuerySysInfoFn = LONG(NTAPI*)(ULONG, PVOID, ULONG, PULONG);
using NtSetSysInfoFn = LONG(NTAPI*)(ULONG, PVOID, ULONG);
using NtQueryTimerFn = LONG(NTAPI*)(PULONG, PULONG, PULONG);
using NtSetTimerFn = LONG(NTAPI*)(ULONG, BOOLEAN, PULONG);

static const ULONG kSystemMemoryListInformation = 80;
static const ULONG kMemoryPurgeStandbyList = 4;

struct MemoryListInfo {
    ULONG_PTR zero, free, modified, modified_no_write, bad;
    ULONG_PTR standby_by_priority[8];
    ULONG_PTR repurposed_by_priority[8];
    ULONG_PTR modified_page_file;
};

static FARPROC ntdll(const char* name) {
    return GetProcAddress(GetModuleHandleW(L"ntdll.dll"), name);
}

// Lire et purger la liste d'attente demandent ce privilège (comptes admin seulement).
static void enable_profile_privilege() {
    static bool done = false;
    if (done) return;
    done = true;
    HANDLE token;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &token)) return;
    TOKEN_PRIVILEGES tp{};
    tp.PrivilegeCount = 1;
    tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
    if (LookupPrivilegeValueW(nullptr, L"SeProfileSingleProcessPrivilege", &tp.Privileges[0].Luid))
        AdjustTokenPrivileges(token, FALSE, &tp, 0, nullptr, nullptr);
    CloseHandle(token);
}

extern "C" int opt_memory_lists(uint64_t* standby, uint64_t* free_bytes) {
    enable_profile_privilege();
    auto query = reinterpret_cast<NtQuerySysInfoFn>(ntdll("NtQuerySystemInformation"));
    MemoryListInfo info{};
    if (!query || query(kSystemMemoryListInformation, &info, sizeof(info), nullptr) < 0) return 0;
    SYSTEM_INFO si;
    GetSystemInfo(&si);
    uint64_t pages = 0;
    for (ULONG_PTR p : info.standby_by_priority) pages += p;
    *standby = pages * si.dwPageSize;
    *free_bytes = uint64_t(info.zero + info.free) * si.dwPageSize;
    return 1;
}

extern "C" int opt_purge_standby(void) {
    enable_profile_privilege();
    auto set = reinterpret_cast<NtSetSysInfoFn>(ntdll("NtSetSystemInformation"));
    ULONG cmd = kMemoryPurgeStandbyList;
    return set && set(kSystemMemoryListInformation, &cmd, sizeof(cmd)) >= 0 ? 1 : 0;
}

extern "C" int opt_timer_resolution(uint32_t* coarsest, uint32_t* finest, uint32_t* current) {
    auto query = reinterpret_cast<NtQueryTimerFn>(ntdll("NtQueryTimerResolution"));
    ULONG a, b, c;
    if (!query || query(&a, &b, &c) < 0) return 0;
    *coarsest = a;
    *finest = b;
    *current = c;
    return 1;
}

extern "C" int opt_set_timer_resolution(uint32_t resolution, int enable) {
    auto set = reinterpret_cast<NtSetTimerFn>(ntdll("NtSetTimerResolution"));
    ULONG actual;
    return set && set(resolution, enable ? TRUE : FALSE, &actual) >= 0 ? 1 : 0;
}

extern "C" int opt_reg_enum_value(int hklm, const wchar_t* key, uint32_t index, wchar_t* name, uint32_t name_chars,
                                  wchar_t* data, uint32_t data_chars) {
    HKEY h;
    if (RegOpenKeyExW(root(hklm), key, 0, KEY_READ, &h) != ERROR_SUCCESS) return 0;
    DWORD nlen = name_chars, type = 0, dlen = (data_chars - 1) * sizeof(wchar_t);
    LSTATUS s = RegEnumValueW(h, index, name, &nlen, nullptr, &type, (BYTE*)data, &dlen);
    RegCloseKey(h);
    if (s != ERROR_SUCCESS && s != ERROR_MORE_DATA) return 0;
    // Valeur non texte ou trop longue : on garde le nom, sans la commande.
    if (s == ERROR_MORE_DATA || (type != REG_SZ && type != REG_EXPAND_SZ)) dlen = 0;
    data[dlen / sizeof(wchar_t)] = 0;
    return 1;
}

extern "C" int opt_reg_get_binary(int hklm, const wchar_t* key, const wchar_t* name, uint8_t* buf, uint32_t* len) {
    DWORD size = *len;
    if (RegGetValueW(root(hklm), key, name, RRF_RT_REG_BINARY, nullptr, buf, &size) != ERROR_SUCCESS) return 0;
    *len = size;
    return 1;
}

extern "C" int opt_reg_set_binary(int hklm, const wchar_t* key, const wchar_t* name, const uint8_t* data, uint32_t len) {
    return RegSetKeyValueW(root(hklm), key, name, REG_BINARY, data, len) == ERROR_SUCCESS;
}

// Accélération de la souris ("Améliorer la précision du pointeur").
extern "C" int opt_mouse_accel(void) {
    int p[3] = {};
    return SystemParametersInfoW(SPI_GETMOUSE, 0, p, 0) && p[2] != 0;
}

extern "C" int opt_set_mouse_accel(int on) {
    int p[3] = {6, 10, on ? 1 : 0};  // seuils par défaut de Windows
    if (!on) p[0] = p[1] = 0;
    return SystemParametersInfoW(SPI_SETMOUSE, 0, p, SPIF_UPDATEINIFILE | SPIF_SENDCHANGE) ? 1 : 0;
}

// --- Icône dans la zone de notification ---
// Thread à part avec sa propre fenêtre cachée et sa boucle de messages ; l'interface Rust
// ne fait que cacher sa fenêtre et lire opt_tray_quit_requested().
static const UINT WM_TRAY = WM_APP + 1;
static HWND g_tray = nullptr;
static HWND g_main = nullptr;
static std::atomic<bool> g_quit{false};
static NOTIFYICONDATAW g_nid{};

// La fenêtre principale : la seule fenêtre de premier niveau titrée de ce processus.
static HWND main_window() {
    if (g_main) return g_main;
    EnumWindows(
        [](HWND h, LPARAM) -> BOOL {
            DWORD pid;
            GetWindowThreadProcessId(h, &pid);
            if (pid == GetCurrentProcessId() && h != g_tray && !GetWindow(h, GW_OWNER) && GetWindowTextLengthW(h) > 0) {
                g_main = h;
                return FALSE;
            }
            return TRUE;
        },
        0);
    return g_main;
}

static void show_main() {
    if (HWND h = main_window()) {
        ShowWindow(h, SW_SHOW);
        ShowWindow(h, SW_RESTORE);
        SetForegroundWindow(h);
    }
}

static LRESULT CALLBACK tray_proc(HWND h, UINT msg, WPARAM w, LPARAM l) {
    if (msg == WM_TRAY) {
        UINT ev = LOWORD(l);
        if (ev == WM_LBUTTONUP || ev == WM_LBUTTONDBLCLK) {
            show_main();
        } else if (ev == WM_RBUTTONUP) {
            POINT p;
            GetCursorPos(&p);
            HMENU menu = CreatePopupMenu();
            AppendMenuW(menu, MF_STRING, 1, L"Ouvrir");
            AppendMenuW(menu, MF_STRING, 2, L"Quitter");
            SetForegroundWindow(h);  // sinon le menu ne se ferme pas en cliquant ailleurs
            int cmd = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON, p.x, p.y, 0, h, nullptr);
            DestroyMenu(menu);
            if (cmd == 1) show_main();
            if (cmd == 2) {
                g_quit = true;
                show_main();  // une fenêtre cachée ne traiterait pas la fermeture
                PostMessageW(main_window(), WM_CLOSE, 0, 0);
            }
        }
        return 0;
    }
    if (msg == WM_DESTROY) {
        Shell_NotifyIconW(NIM_DELETE, &g_nid);
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(h, msg, w, l);
}

static DWORD WINAPI tray_thread(LPVOID tip) {
    WNDCLASSW wc{};
    wc.lpfnWndProc = tray_proc;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.lpszClassName = L"EclairTray";
    RegisterClassW(&wc);
    g_tray = CreateWindowW(wc.lpszClassName, L"", 0, 0, 0, 0, 0, nullptr, nullptr, wc.hInstance, nullptr);
    if (!g_tray) return 0;
    g_nid.cbSize = sizeof(g_nid);
    g_nid.hWnd = g_tray;
    g_nid.uID = 1;
    g_nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    g_nid.uCallbackMessage = WM_TRAY;
    g_nid.hIcon = LoadIconW(wc.hInstance, MAKEINTRESOURCEW(1));
    if (!g_nid.hIcon) g_nid.hIcon = LoadIconW(nullptr, IDI_APPLICATION);
    wcsncpy_s(g_nid.szTip, static_cast<const wchar_t*>(tip), _TRUNCATE);
    Shell_NotifyIconW(NIM_ADD, &g_nid);
    MSG m;
    while (GetMessageW(&m, nullptr, 0, 0) > 0) {
        TranslateMessage(&m);
        DispatchMessageW(&m);
    }
    return 0;
}

extern "C" int opt_tray_start(const wchar_t* tip) {
    static std::wstring copy;  // doit vivre aussi longtemps que le thread
    copy = tip;
    HANDLE t = CreateThread(nullptr, 0, tray_thread, (LPVOID)copy.c_str(), 0, nullptr);
    if (!t) return 0;
    CloseHandle(t);
    return 1;
}

extern "C" void opt_tray_stop(void) {
    // Synchrone : l'icône doit disparaître avant la fin du processus.
    if (g_tray) SendMessageW(g_tray, WM_CLOSE, 0, 0);
}

extern "C" void opt_tray_notify(const wchar_t* title, const wchar_t* text) {
    if (!g_tray) return;
    NOTIFYICONDATAW n = g_nid;
    n.uFlags = NIF_INFO;
    n.dwInfoFlags = NIIF_USER;
    wcsncpy_s(n.szInfoTitle, title, _TRUNCATE);
    wcsncpy_s(n.szInfo, text, _TRUNCATE);
    Shell_NotifyIconW(NIM_MODIFY, &n);
}

extern "C" int opt_tray_quit_requested(void) {
    return g_quit ? 1 : 0;
}

extern "C" int opt_hide_main_window(void) {
    HWND h = main_window();
    return h && ShowWindow(h, SW_HIDE) ? 1 : 0;
}

// Processus de la fenêtre au premier plan si elle couvre tout son écran (jeu en plein écran), sinon 0.
extern "C" uint32_t opt_fullscreen_foreground_pid(void) {
    HWND h = GetForegroundWindow();
    if (!h || h == GetDesktopWindow() || h == GetShellWindow()) return 0;
    RECT r;
    MONITORINFO mi{};
    mi.cbSize = sizeof(mi);
    if (!GetWindowRect(h, &r) || !GetMonitorInfoW(MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST), &mi)) return 0;
    const RECT& m = mi.rcMonitor;
    if (r.left > m.left || r.top > m.top || r.right < m.right || r.bottom < m.bottom) return 0;
    DWORD pid = 0;
    GetWindowThreadProcessId(h, &pid);
    return pid;
}

// Utilisation 3D de la carte graphique (compteur "GPU Engine", toutes marques), -1 si indisponible.
// Comme pour le CPU, mesuré depuis l'appel précédent : à appeler depuis un seul thread.
extern "C" double opt_gpu_usage(void) {
    static PDH_HQUERY query = nullptr;
    static PDH_HCOUNTER counter = nullptr;
    static bool unavailable = false;
    if (unavailable) return -1.0;
    if (!query) {
        if (PdhOpenQueryW(nullptr, 0, &query) != ERROR_SUCCESS ||
            PdhAddEnglishCounterW(query, L"\\GPU Engine(*engtype_3D)\\Utilization Percentage", 0, &counter) != ERROR_SUCCESS) {
            unavailable = true;
            return -1.0;
        }
        PdhCollectQueryData(query);
        return 0.0;
    }
    if (PdhCollectQueryData(query) != ERROR_SUCCESS) return 0.0;
    DWORD size = 0, count = 0;
    PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &size, &count, nullptr);
    std::vector<BYTE> buf(size);
    auto items = reinterpret_cast<PDH_FMT_COUNTERVALUE_ITEM_W*>(buf.data());
    if (size == 0 || PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &size, &count, items) != ERROR_SUCCESS) return 0.0;
    double sum = 0;
    for (DWORD i = 0; i < count; i++)
        if (items[i].FmtValue.CStatus == ERROR_SUCCESS) sum += items[i].FmtValue.doubleValue;
    return sum > 100.0 ? 100.0 : sum;
}
