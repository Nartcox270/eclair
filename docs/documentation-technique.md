# Éclair — Documentation technique

Version 1.1.0

Ce document décrit l'architecture d'Éclair, le rôle de chaque module, l'interface entre Rust et C++, les algorithmes du moteur de priorités, les modifications apportées à Windows, ainsi que les procédures de compilation, de test et de publication.

---

## Sommaire

1. [Vue d'ensemble](#1-vue-densemble)
2. [Arborescence du projet](#2-arborescence-du-projet)
3. [Architecture](#3-architecture)
4. [Cœur natif C++](#4-cœur-natif-c)
5. [Modules Rust](#5-modules-rust)
6. [Moteur de priorités](#6-moteur-de-priorités)
7. [Mémoire en attente et minuteur](#7-mémoire-en-attente-et-minuteur)
8. [Modifications apportées à Windows](#8-modifications-apportées-à-windows)
9. [Fichiers et persistance](#9-fichiers-et-persistance)
10. [Interface graphique](#10-interface-graphique)
11. [Compilation](#11-compilation)
12. [Tests](#12-tests)
13. [Publication](#13-publication)
14. [Limites connues](#14-limites-connues)
15. [Ajouter une fonctionnalité](#15-ajouter-une-fonctionnalité)

---

## 1. Vue d'ensemble

| Élément | Choix |
|---|---|
| Langages | Rust (application, interface, logique) et C++17 (appels système Win32) |
| Interface graphique | egui / eframe 0.29, rendu OpenGL (glow) |
| Liaison Rust / C++ | Fonctions `extern "C"` compilées par le crate `cc` dans `build.rs` |
| Ressources Windows | Icône et manifeste intégrés par le crate `embed-resource` |
| Cible | Windows 10 et 11, x86-64, chaîne MSVC |
| Dépendances d'exécution | Aucune en dehors de Windows (winget et DISM sont appelés s'ils sont présents) |
| Installateur | Inno Setup 6 |

Principe de répartition : le C++ contient uniquement des appels Win32 sans logique applicative ; toute décision (seuils, modes, interface, persistance) est en Rust.

---

## 2. Arborescence du projet

```
assets/
  app.rc, app-admin.rc   scripts de ressources (icône ; icône + manifeste administrateur)
  app.manifest           manifeste Windows (requireAdministrator)
  eclair.ico, .png       logo
  make_icon.ps1          génération du logo
cpp/
  optimizer.h            interface C exposée à Rust
  optimizer.cpp          implémentation Win32
docs/                    documentation
installer/
  eclair.iss             script Inno Setup
src/
  main.rs                point d'entrée, état de l'application, actions
  layout.rs              cadre de fenêtre, barre latérale, journal, tableau de bord, réglages
  pages.rs               pages Optimisation et Installation
  style.rs               thème, cartes, interrupteurs
  ffi.rs                 liaisons vers le C++ et enveloppes sûres
  procs.rs               mesure du processeur, moteur de priorités, journal de reprise
  memory.rs              purge de la mémoire en attente, minuteur
  gaming.rs              réglages Windows pour le jeu
  privacy.rs             réglages de confidentialité
  startup.rs             programmes au démarrage
  installer.rs           installation et mise à jour via winget
  restore.rs             point de restauration
  config.rs              lecture et écriture des réglages
build.rs                 compilation du C++ et des ressources
Cargo.toml
```

---

## 3. Architecture

### 3.1 Couches

```
+-------------------------------------------------------------+
| Interface (egui)   layout.rs, pages.rs, style.rs             |
+-------------------------------------------------------------+
| Application        main.rs : état App, actions, travaux      |
+-------------------------------------------------------------+
| Domaines           procs, memory, gaming, privacy, startup,  |
|                    installer, restore, config                |
+-------------------------------------------------------------+
| Liaisons           ffi.rs (unsafe confiné ici)               |
+-------------------------------------------------------------+
| Cœur natif         cpp/optimizer.cpp (Win32)                 |
+-------------------------------------------------------------+
```

Le code `unsafe` est limité à `ffi.rs`, qui expose des fonctions Rust sûres (conversion des chaînes en UTF-16 terminé par zéro, tampons dimensionnés, codes de retour convertis en `bool` ou `Option`).

### 3.2 Threads

| Thread | Créé par | Rôle | Communication |
|---|---|---|---|
| Interface | eframe | Affichage, actions, rafraîchissement des statistiques chaque seconde | — |
| Moteur de priorités | `procs::Engine::start` | Boucle d'une seconde : modes Équilibré, Gaming, Gaming automatique, journal de reprise | Atomiques et `Mutex` partagés, canal `events` vers le journal |
| Mémoire | `memory::StandbyCleaner::start` | Boucle d'une seconde : purge automatique, minuteur | Atomiques, canal `events` |
| Zone de notification | `opt_tray_start` (C++) | Fenêtre cachée, boucle de messages, menu de l'icône | Variable atomique `g_quit`, appels `ShowWindow` |
| Travaux ponctuels | `App::spawn` | Nettoyage, libération mémoire, installations, mises à jour | Canal `Msg` (un seul travail à la fois) |
| Point de restauration | `App::create_restore_point` | Appel PowerShell `Checkpoint-Computer` | Canal `notes` |

Les boucles du moteur et de la mémoire sont indépendantes de l'affichage : elles continuent lorsque la fenêtre est masquée ou réduite.

### 3.3 Cycle d'une image

`App::update` (environ une fois par seconde au repos, dix fois par seconde pendant un travail) :

1. Réduction au démarrage si l'argument `--minimized` est présent.
2. Interception de la fermeture : `CancelClose` puis masquage, sauf si l'utilisateur a demandé à quitter.
3. Lecture des messages des travaux, du moteur, de la mémoire et des notes.
4. Rafraîchissement des statistiques (au plus une fois par seconde, ou immédiatement après une action).
5. Dessin du cadre et de la page courante.

---

## 4. Cœur natif C++

Fichier `cpp/optimizer.cpp`, interface `cpp/optimizer.h`. Toutes les fonctions sont `extern "C"` et renvoient `1` en cas de succès sauf mention contraire.

### 4.1 Informations système

| Fonction | API Win32 | Description |
|---|---|---|
| `opt_is_admin` | `GetTokenInformation(TokenElevation)` | Processus élevé ou non |
| `opt_memory_info` | `GlobalMemoryStatusEx` | Charge, mémoire totale et disponible |
| `opt_cpu_usage` | `GetSystemTimes` | Charge processeur depuis l'appel précédent (état statique, un seul thread appelant) |
| `opt_disk_space` | `GetDiskFreeSpaceExW` | Espace libre et total d'un volume |
| `opt_uptime_ms` | `GetTickCount64` | Durée depuis le démarrage |
| `opt_gpu_usage` | PDH, compteur `\GPU Engine(*engtype_3D)\Utilization Percentage` | Somme de l'utilisation 3D, plafonnée à 100, -1 si indisponible (état statique) |

### 4.2 Nettoyage

| Fonction | Description |
|---|---|
| `opt_clean_directory(path, dry_run, out)` | Parcours récursif par `FindFirstFileExW`. Supprime les fichiers (attribut lecture seule retiré au préalable), puis les dossiers vidés. Les points d'analyse (jonctions, liens) ne sont jamais suivis : seul le lien est retiré. Les fichiers verrouillés sont comptés dans `failed`. En mode `dry_run`, mesure seulement. |
| `opt_recycle_bin_size` | `SHQueryRecycleBinW` sur tous les lecteurs |
| `opt_empty_recycle_bin` | `SHEmptyRecycleBinW` sans confirmation ; `E_UNEXPECTED` (corbeille vide) est traité comme un succès |

### 4.3 Performance et alimentation

| Fonction | Description |
|---|---|
| `opt_trim_working_sets` | `EnumProcesses` puis `EmptyWorkingSet` sur chaque processus accessible ; renvoie le nombre de processus traités |
| `opt_flush_dns` | `DnsFlushResolverCache`, chargée dynamiquement depuis `dnsapi.dll` (fonction non documentée) |
| `opt_get_power_plan` / `opt_set_power_plan` | Plans par GUID : 0 Équilibré, 1 Performances élevées, 2 Économie d'énergie, 3 Performances max, -1 autre |

Plan « Performances max » : recherché par son nom avec `PowerEnumerate` et `PowerReadFriendlyName`. À défaut, créé par `PowerDuplicateScheme` à partir de Performances élevées (repli sur Équilibré), renommé, puis réglé sur secteur uniquement : `CPMINCORES` (nombre minimal de cœurs actifs) à 100 et `PROCTHROTTLEMIN` (état minimal du processeur) à 100.

### 4.4 Processus

| Fonction | Description |
|---|---|
| `opt_list_processes(out, cap)` | `CreateToolhelp32Snapshot` (cinq tentatives, la capture pouvant échouer pendant des créations de processus), puis pour chaque processus ouvrable avec `PROCESS_QUERY_LIMITED_INFORMATION` : nom, temps processeur cumulé (`GetProcessTimes`), mémoire de travail (`GetProcessMemoryInfo`), classe de priorité (`GetPriorityClass`). Les processus inaccessibles sont omis. |
| `opt_set_priority(pid, level)` | `SetPriorityClass`, niveaux 0 à 4. Le niveau 5 (temps réel) est refusé. |
| `opt_foreground_pid` | Processus de la fenêtre au premier plan |
| `opt_fullscreen_foreground_pid` | Processus de la fenêtre au premier plan si son rectangle couvre entièrement son moniteur, sinon 0. Le bureau et la fenêtre du shell sont exclus. |

Correspondance des niveaux : 0 `IDLE`, 1 `BELOW_NORMAL`, 2 `NORMAL`, 3 `ABOVE_NORMAL`, 4 `HIGH`, 5 `REALTIME`.

### 4.5 Mémoire et minuteur

Fonctions non documentées de `ntdll`, chargées par `GetProcAddress`.

| Fonction | Description |
|---|---|
| `opt_memory_lists` | `NtQuerySystemInformation(SystemMemoryListInformation = 80)` : somme des pages en attente des huit priorités, pages libres et mises à zéro, multipliées par la taille de page |
| `opt_purge_standby` | `NtSetSystemInformation(SystemMemoryListInformation, MemoryPurgeStandbyList = 4)` |
| `opt_timer_resolution` | `NtQueryTimerResolution` : résolutions la plus grossière, la plus fine, actuelle (unités de 100 ns) |
| `opt_set_timer_resolution` | `NtSetTimerResolution` : demande ou libère une résolution pour le processus |

La lecture et la purge exigent le privilège `SeProfileSingleProcessPrivilege`, activé une fois par `AdjustTokenPrivileges` (disponible uniquement pour un administrateur élevé).

### 4.6 Registre

| Fonction | Description |
|---|---|
| `opt_reg_get_dword`, `opt_reg_get_sz`, `opt_reg_get_binary` | `RegGetValueW` avec restriction de type |
| `opt_reg_set_dword`, `opt_reg_set_sz`, `opt_reg_set_binary` | `RegSetKeyValueW` (crée la clé si nécessaire) |
| `opt_reg_delete` | `RegDeleteKeyValueW` ; une valeur absente est un succès |
| `opt_reg_enum_value` | `RegEnumValueW` : nom et, pour les chaînes, contenu |

Le paramètre `hklm` choisit `HKEY_LOCAL_MACHINE` (écriture réservée aux administrateurs) ou `HKEY_CURRENT_USER`. Le processus étant 64 bits, la vue 64 bits du registre est utilisée.

### 4.7 Souris

`opt_mouse_accel` et `opt_set_mouse_accel` utilisent `SystemParametersInfoW(SPI_GETMOUSE / SPI_SETMOUSE)` avec `SPIF_UPDATEINIFILE | SPIF_SENDCHANGE`. Activation : seuils 6 et 10, accélération 1 (valeurs par défaut de Windows). Désactivation : 0, 0, 0.

### 4.8 Zone de notification

Un thread dédié crée une fenêtre cachée de classe `EclairTray`, ajoute l'icône (ressource 1 de l'exécutable) avec `Shell_NotifyIconW` et exécute sa propre boucle de messages.

| Événement | Action |
|---|---|
| Clic gauche | `ShowWindow(SW_SHOW)`, `SW_RESTORE`, `SetForegroundWindow` sur la fenêtre principale |
| Clic droit | Menu Ouvrir / Quitter. Quitter positionne `g_quit`, réaffiche la fenêtre puis lui envoie `WM_CLOSE` (une fenêtre masquée ne traiterait pas la fermeture) |

La fenêtre principale est la seule fenêtre de premier niveau titrée du processus, trouvée par `EnumWindows` et mise en cache. `opt_tray_stop` envoie `WM_CLOSE` de façon synchrone (`SendMessageW`) afin que l'icône soit retirée avant la fin du processus. `opt_tray_notify` affiche une bulle (`NIF_INFO`).

---

## 5. Modules Rust

| Module | Responsabilité | Éléments principaux |
|---|---|---|
| `main.rs` | Point d'entrée, état `App`, actions | `App::new`, `refresh_stats`, `spawn`, `start_clean`, `trim_memory`, `set_mode`, `set_persistent`, `persist`, `before_change`, `create_restore_point`, `start_install`, `start_upgrade_all`, `hide_to_tray`, `set_autostart`, `run_dism` |
| `layout.rs` | Cadre et tableau de bord | `ui_frame`, `ui_dashboard`, `ui_settings`, `health`, graphique, anneau de santé |
| `pages.rs` | Pages | `ui_cleaning`, `ui_performance`, `ui_processes`, `ui_gaming`, `ui_privacy`, `ui_startup`, `ui_installation` |
| `style.rs` | Thème | Couleurs, `apply`, `card`, `section`, `page_header`, `toggle`, `toggle_row`, `wide_button` |
| `ffi.rs` | Liaisons | Enveloppes sûres de toutes les fonctions C |
| `procs.rs` | Priorités | `Sampler`, `Engine`, modes, journal, `persist_priority` |
| `memory.rs` | Mémoire | `StandbyCleaner`, `purge`, `should_purge` |
| `gaming.rs` | Réglages jeu | Table `TWEAKS` (`get` et `set` par réglage) |
| `privacy.rs` | Confidentialité | Table `SETTINGS` (clé, valeurs, valeur « activé » et « désactivé ») |
| `startup.rs` | Démarrage | `list`, `set_enabled` |
| `installer.rs` | Applications | Table `APPS`, `installed`, `install`, `upgrade_all` |
| `restore.rs` | Restauration | `create`, `message`, `open_system_restore` |
| `config.rs` | Réglages | `Config::load`, `save`, `dir` |

---

## 6. Moteur de priorités

### 6.1 Mesure du processeur par processus

`Sampler::sample` lit la liste des processus et calcule, pour chacun :

```
cpu % = (temps processeur cumulé - temps précédent) / (durée écoulée × nombre de processeurs logiques) × 100
```

La valeur est exprimée en pourcentage de la capacité totale du processeur. Le premier échantillon renvoie 0.

### 6.2 Boucle

Toutes les secondes, sous le verrou de la table `changed` (programme modifié vers priorité d'origine) :

1. Lecture du mode choisi et, si le mode est Désactivé et le Gaming automatique actif, détection d'un jeu prioritaire en plein écran. La détection prolonge le mode Gaming de 15 secondes (`AUTO_GRACE`).
2. Publication du mode effectif (`effective`), lu par l'interface.
3. Au changement de mode : la liste des processus, lue avant la restauration, est corrigée avec les priorités d'origine, puis toutes les priorités modifiées sont rendues.
4. Si le mode est Désactivé ou si la capture a échoué (liste vide), arrêt de l'itération. Une liste vide n'est jamais interprétée comme « tous les processus ont disparu ».
5. Retrait des processus terminés de la table.
6. Application du mode Équilibré ou Gaming.
7. Écriture du journal de reprise si son contenu a changé.

### 6.3 Mode Équilibré

| Paramètre | Valeur |
|---|---|
| Charge globale minimale | 40 % (`BUSY`) |
| Seuil de ralentissement | min(10 %, 75 % d'un cœur) |
| Seuil de rétablissement | min(5 %, 30 % d'un cœur) |
| Durée de dépassement | 2 secondes consécutives (`SUSTAIN`) |

Les seuils sont relatifs à un cœur : sur un processeur de 20 threads, un programme qui sature un cœur ne représente que 5 % du total et ne serait jamais détecté avec un seuil fixe de 10 %. Ce défaut a été mis en évidence par le test de charge.

Un programme ralenti retrouve sa priorité lorsque sa charge passe sous le seuil de rétablissement, lorsque la charge globale passe sous 20 %, ou lorsque sa fenêtre passe au premier plan. Seuls les programmes en priorité normale sont ralentis ; la fenêtre au premier plan et Éclair lui-même sont exclus.

### 6.4 Mode Gaming

Priorité cible par programme (`gaming_target`) :

| Condition | Cible |
|---|---|
| Nom présent dans les favoris (insensible à la casse) | Haute |
| Fenêtre au premier plan, Éclair, `explorer.exe`, `dwm.exe`, `audiodg.exe`, `csrss.exe` | Aucune (priorité d'origine) |
| Autre programme en priorité normale | Sous la normale |
| Autre programme dans une autre priorité | Inchangé (réglage manuel respecté) |

Un échec de `SetPriorityClass` (processus protégé, droits insuffisants) place le processus dans une liste d'exclusion pour la durée du mode, afin de ne pas réessayer chaque seconde.

### 6.5 Restitution des priorités

| Situation | Mécanisme |
|---|---|
| Changement de mode | `restore_all` dans la boucle |
| Fermeture normale | `Drop` de `Engine` : restitution puis suppression du journal |
| Arrêt brutal | Journal `%APPDATA%\Eclair\priorites.txt` relu par `recover` au lancement suivant |

Format du journal : une ligne par programme, `pid<TAB>priorité d'origine<TAB>nom`. À la reprise, une priorité n'est rendue que si le processus de même identifiant existe et porte le même nom, ce qui évite d'agir sur un identifiant réutilisé par un autre programme. Le journal est supprimé après lecture.

### 6.6 Priorités persistantes

En mode Persistant, la priorité des favoris (et les priorités réglées manuellement) est enregistrée dans :

```
HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\<exe>\PerfOptions
  CpuPriorityClass (DWORD) : 1 inactive, 5 sous la normale, 2 normale, 6 supérieure, 3 haute
```

Windows applique cette priorité à chaque lancement de l'exécutable, indépendamment d'Éclair. La liste des exécutables concernés est conservée dans `config.txt` (`persisted=`) pour permettre leur effacement lors du retour en mode Temporaire.

---

## 7. Mémoire en attente et minuteur

Boucle d'une seconde dans `memory.rs` :

- **Purge automatique** si activée et si `liste d'attente >= seuil_liste` et `mémoire libre < seuil_libre` (1024 Mo et 1024 Mo par défaut, modifiables de 128 à 65536 Mo).
- **Minuteur** : à l'activation, `NtSetTimerResolution(résolution la plus fine, TRUE)` ; à la désactivation, libération. La demande appartient au processus et cesse à sa fin.

La résolution globale (Windows 11) est la valeur `GlobalTimerResolutionRequests = 1` sous `HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\kernel`.

---

## 8. Modifications apportées à Windows

### 8.1 Page Jeu

| Réglage | Emplacement | Valeur optimisée | Valeur écrite à l'annulation |
|---|---|---|---|
| Jeux fenêtrés | `HKCU\Software\Microsoft\DirectX\UserGpuPreferences`, `DirectXUserGlobalSettings` (REG_SZ) | jeton `SwapEffectUpgradeEnable=1;` (les autres jetons sont conservés) | `SwapEffectUpgradeEnable=0;` |
| HAGS | `HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers`, `HwSchMode` | 2 | 1 |
| Enregistrement Xbox | `HKCU\System\GameConfigStore`, `GameDVR_Enabled` et `HKCU\Software\Microsoft\Windows\CurrentVersion\GameDVR`, `AppCaptureEnabled` | 0 | 1 |
| Mode Jeu | `HKCU\Software\Microsoft\GameBar`, `AutoGameModeEnabled` | 1 | 0 |
| Accélération souris | `SPI_SETMOUSE` | 0, 0, 0 | 6, 10, 1 |
| Intégrité de la mémoire | `HKLM\SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity`, `Enabled` | 0 | 1 |

### 8.2 Page Confidentialité

Une valeur absente est considérée comme « activée », conformément au comportement par défaut de Windows.

| Réglage | Ruche | Clé | Valeur(s) | Désactivé | Activé |
|---|---|---|---|---|---|
| Localisation | HKCU | `Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location` | `Value` (REG_SZ) | `Deny` | `Allow` |
| Télémétrie | HKLM | `SOFTWARE\Policies\Microsoft\Windows\DataCollection` | `AllowTelemetry` | 0 | valeur supprimée |
| Programme d'amélioration | HKLM | `SOFTWARE\Policies\Microsoft\SQMClient\Windows` | `CEIPEnable` | 0 | valeur supprimée |
| Télémétrie des applis | HKLM | `SOFTWARE\Policies\Microsoft\Windows\AppCompat` | `AITEnable` | 0 | valeur supprimée |
| Suivi du lancement | HKCU | `Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced` | `Start_TrackProgs` | 0 | 1 |
| Améliorer l'écriture | HKCU | `Software\Microsoft\Input\TIPC` | `Enabled` | 0 | 1 |
| Personnaliser l'écriture | HKCU | `Software\Microsoft\Personalization\Settings` | `AcceptedPrivacyPolicy` | 0 | 1 |
| Rapports d'erreurs | HKLM | `SOFTWARE\Microsoft\Windows\Windows Error Reporting` | `Disabled` | 1 | 0 |
| Reconnaissance vocale | HKCU | `Software\Microsoft\Speech_OneCore\Settings\OnlineSpeechPrivacy` | `HasAccepted` | 0 | 1 |
| Expériences personnalisées | HKCU | `Software\Microsoft\Windows\CurrentVersion\Privacy` | `TailoredExperiencesWithDiagnosticDataEnabled` | 0 | 1 |
| ID de publicité | HKCU | `Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo` | `Enabled` | 0 | 1 |
| Contenu suggéré (Paramètres) | HKCU | `Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager` | `SubscribedContent-338393Enabled`, `-353694Enabled`, `-353696Enabled` | 0 | 1 |
| Suggestions menu Démarrer | HKCU | idem | `SystemPaneSuggestionsEnabled`, `SubscribedContent-338388Enabled` | 0 | 1 |
| Notifications suggérées | HKCU | `Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\Windows.SystemToast.Suggested` | `Enabled` | 0 | 1 |
| Écran de verrouillage | HKCU | `...\ContentDeliveryManager` | `RotatingLockScreenOverlayEnabled`, `SubscribedContent-338387Enabled` | 0 | 1 |

Un réglage à plusieurs valeurs est considéré comme désactivé uniquement si toutes ses valeurs le sont.

### 8.3 Page Démarrage

Entrées lues dans :

| Portée | Clé Run | Clé d'approbation |
|---|---|---|
| Utilisateur | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` | `HKCU\...\Explorer\StartupApproved\Run` |
| Machine | `HKLM\Software\Microsoft\Windows\CurrentVersion\Run` | `HKLM\...\Explorer\StartupApproved\Run` |
| Machine 32 bits | `HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run` | `HKLM\...\Explorer\StartupApproved\Run32` |

L'état est la valeur binaire de même nom dans la clé d'approbation : premier octet pair (02, 06) ou valeur absente signifie activé, impair (03, 07) signifie désactivé. Éclair écrit 12 octets commençant par 02 ou 03, comme le Gestionnaire des tâches. L'entrée Run n'est jamais supprimée.

### 8.4 Autres modifications

| Fonction | Modification |
|---|---|
| Plan Performances max | Nouveau plan d'alimentation « Optimiseur - Performances max » |
| Mode Persistant | Tâche planifiée `Eclair` (ouverture de session, privilèges les plus élevés, argument `--minimized`), valeurs `CpuPriorityClass` sous Image File Execution Options |
| Point de restauration | `Checkpoint-Computer -RestorePointType MODIFY_SETTINGS -Description Eclair` ; code de sortie 2 lorsque Windows signale un point de moins de 24 heures |
| Nettoyage DISM | `dism /English /Online /Cleanup-Image /AnalyzeComponentStore` (analyse) et `/StartComponentCleanup` (nettoyage) |
| Installation | `winget install --id <id> -e --silent --accept-package-agreements --accept-source-agreements --disable-interactivity` |
| Mise à jour | `winget upgrade --all --silent ...` |
| Détection des applications installées | `winget export` vers un fichier temporaire, puis recherche des lignes `PackageIdentifier` |

Les commandes externes sont lancées avec `CREATE_NO_WINDOW`.

---

## 9. Fichiers et persistance

Dossier `%APPDATA%\Eclair` :

| Fichier | Contenu |
|---|---|
| `config.txt` | Réglages, une ligne `clé=valeur` |
| `priorites.txt` | Journal de reprise ; présent uniquement lorsque des priorités sont modifiées |

Clés de `config.txt` :

| Clé | Type | Défaut | Description |
|---|---|---|---|
| `mode` | 0, 1, 2 | 0 | Mode de priorité (Désactivé, Équilibré, Gaming) |
| `persistent` | 0 ou 1 | 0 | Mode Persistant |
| `auto_purge` | 0 ou 1 | 0 | Purge automatique de la mémoire en attente |
| `list_mb` | entier | 1024 | Seuil de la liste d'attente |
| `free_mb` | entier | 1024 | Seuil de mémoire libre |
| `fine_timer` | 0 ou 1 | 0 | Minuteur à la résolution la plus fine |
| `auto_gaming` | 0 ou 1 | 0 | Gaming automatique |
| `favorite` | texte, répétable | `discord.exe` | Jeu ou application prioritaire |
| `persisted` | texte, répétable | — | Exécutable dont la priorité est enregistrée dans Windows |

Les valeurs de mode, purge, minuteur et Gaming automatique ne sont appliquées au lancement qu'en mode Persistant. Les favoris sont toujours conservés.

---

## 10. Interface graphique

### 10.1 Fenêtre

Fenêtre de 1100 × 720 pixels, sans décoration Windows (`with_decorations(false)`) et non redimensionnable. La barre de titre est dessinée par l'application : le glisser déclenche `ViewportCommand::StartDrag`, les boutons envoient `Minimized` et `Close`.

### 10.2 Thème

| Jeton | Couleur | Usage |
|---|---|---|
| `BG` | #000000 | Fond |
| `SIDEBAR` | #0A0A0A | Barre latérale, journal |
| `CARD` | #141414 | Cartes |
| `BUTTON` | #282828 | Boutons |
| `BORDER` | #222222 | Bordures, grilles |
| `ACCENT` | #22E07A | Accent, interrupteurs actifs, courbe processeur |
| `RED` | #EB4848 | Alertes |
| `TEXT` | #ECECEC | Texte, courbe mémoire |
| `MUTED` | #8C8C8C | Texte secondaire |
| Courbe GPU | #6EA0FF | Graphique |

Composants réutilisables (`style.rs`) : `card`, `section` (titre en capitales et carte pleine largeur), `page_header`, `toggle` (interrupteur animé), `toggle_row` (ligne-carte avec titre, aide et interrupteur), `wide_button`.

### 10.3 Score de santé

```
appliqués = réglages Jeu recommandés activés + réglages Confidentialité désactivés
total     = réglages Jeu recommandés (5) + réglages Confidentialité (15)
santé     = appliqués / total
```

Le réglage « intégrité de la mémoire » porte `recommended: false` et n'entre pas dans le calcul, afin que le score n'incite pas à réduire la sécurité.

### 10.4 Symboles

Les icônes sont des caractères des polices intégrées à egui. Certains caractères Unicode y sont absents (par exemple le symbole de fermeture standard) ; les icônes ont été vérifiées par capture d'écran.

---

## 11. Compilation

### 11.1 Prérequis

- Rust (chaîne `stable-x86_64-pc-windows-msvc`), installable par `winget install Rustlang.Rustup`.
- Visual Studio Build Tools 2022, charge de travail « Développement Desktop en C++ » (compilateur MSVC et compilateur de ressources).
- Pour l'installateur : Inno Setup 6 (`winget install JRSoftware.InnoSetup`).

### 11.2 Commandes

```
cargo build             # debug : sans manifeste administrateur
cargo build --release   # release : exécutable target\release\Eclair.exe
```

### 11.3 Script de compilation

`build.rs` :

1. compile `cpp/optimizer.cpp` (C++17, `/EHsc`, `/utf-8`) en bibliothèque statique ;
2. lie `psapi`, `shell32`, `powrprof`, `advapi32`, `user32`, `pdh` ;
3. intègre les ressources au binaire `Eclair` : `assets/app.rc` (icône) en debug, `assets/app-admin.rc` (icône et manifeste `requireAdministrator`) en release.

Le manifeste administrateur n'est ajouté qu'en release afin que les versions de développement et les tests se lancent sans élévation.

### 11.4 Logo

`assets/make_icon.ps1` dessine le logo avec System.Drawing et produit `eclair.png` (256 px, icône de fenêtre) et `eclair.ico` (16, 32, 48 et 256 px au format PNG).

---

## 12. Tests

Les tests s'exécutent en debug ; en release, le manifeste exige une élévation que le lanceur de tests ne demande pas (erreur 740).

```
cargo test                                   # tests unitaires et d'intégration
cargo test -- --include-ignored              # avec le test de charge
cargo test stress -- --ignored --nocapture   # test de charge seul, avec mesures
```

| Test | Module | Vérifie |
|---|---|---|
| `system_info` | ffi | Mémoire, disque et durée de fonctionnement cohérents |
| `gpu_counter` | ffi | Lecture du compteur GPU entre 0 et 100 |
| `clean_directory_dry_run_then_delete` | ffi | L'analyse ne supprime rien, le nettoyage supprime fichiers et sous-dossiers |
| `toggle_round_trip` | privacy | Désactivation et réactivation d'un réglage sur une clé de test |
| `directx_settings_keep_others` | gaming | Modification d'un jeton sans perte des autres |
| `flags`, `binary_round_trip` | startup | Interprétation des octets d'approbation, lecture et écriture binaires |
| `parses_winget_export`, `ids_unique` | installer | Analyse de l'export winget sans faux positif, identifiants uniques |
| `round_trip` | config | Écriture puis relecture de tous les réglages |
| `purge_thresholds`, `timer_resolution_readable` | memory | Conditions de purge, lecture du minuteur |
| `sampler_sees_own_cpu` | procs | Mesure de la charge d'un processus occupé |
| `set_priority_round_trip` | procs | Changement de priorité d'un processus enfant, refus du temps réel |
| `gaming_targets` | procs | Cibles du mode Gaming (favori, premier plan, protégé) |
| `thresholds_follow_core_count` | procs | Seuils relatifs au nombre de cœurs |
| `journal_recovers_after_crash` | procs | Reprise après arrêt brutal, pid réutilisé ignoré |
| `stress_modes` (ignoré par défaut) | procs | Charge sur la moitié des cœurs, puis Équilibré, Gaming, 50 processus éphémères et retour à l'état d'origine |

Les tests qui écrivent dans le registre utilisent des clés dédiées (`HKCU\Software\OptimiseurPcTest`, `HKCU\Software\OptimiseurPcTestStartup`) supprimées en fin de test. Les tests qui changent des priorités agissent sur des processus enfants, jamais sur le processus de test : un processus enfant hérite de la priorité de son parent, ce qui fausserait les tests exécutés en parallèle.

Le test de charge modifie réellement la priorité des programmes ouverts pendant quelques secondes. Il ne doit pas être interrompu de force : les priorités ne seraient pas rendues avant le lancement suivant d'Éclair.

---

## 13. Publication

1. Mettre à jour la version dans `Cargo.toml`, `assets/app.manifest` et `installer/eclair.iss` (`AppVersion`).
2. Lancer les tests : `cargo test -- --include-ignored`.
3. Compiler : `cargo build --release`.
4. Archive portable : `Eclair.exe` et `README.md` dans `dist\Eclair-v<version>.zip`.
5. Installateur : `ISCC.exe installer\eclair.iss`, qui produit `dist\Eclair-Setup-<version>.exe`.
6. Vérifier l'installation et la désinstallation silencieuses :

```
Eclair-Setup-<version>.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /CURRENTUSER
"%LOCALAPPDATA%\Programs\Eclair\unins000.exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
```

L'installateur fonctionne par utilisateur (`PrivilegesRequired=lowest`), avec possibilité d'installer pour tous les utilisateurs. Le script doit être enregistré en UTF-8 avec BOM pour que les caractères accentués soient correctement lus par Inno Setup.

Signature : les exécutables ne sont pas signés. Une signature Authenticode (certificat de signature de code) supprimerait l'avertissement SmartScreen.

---

## 14. Limites connues

| Limite | Conséquence |
|---|---|
| Exécutable non signé | Avertissement SmartScreen à la première exécution |
| Fenêtre non redimensionnable | Taille fixe de 1100 × 720 |
| Liste des processus limitée aux processus accessibles | Les processus système et protégés ne sont ni listés ni modifiés |
| Détection des applications installées par identifiant winget exact | Une variante (par exemple `CPUID.CPU-Z.MSI`) n'est pas reconnue comme `CPUID.CPU-Z` |
| Page Démarrage | Dossier Démarrage et tâches planifiées non pris en charge |
| Zone de notification | L'icône n'est pas réinscrite si l'Explorateur Windows redémarre |
| Gaming automatique | Ne détecte que le plein écran (y compris fenêtré sans bordure couvrant l'écran) d'un programme listé comme prioritaire |
| Courbe GPU | Somme de l'utilisation 3D de tous les adaptateurs graphiques |
| Plan d'alimentation | Nom historique « Optimiseur - Performances max » |
| Point de restauration | Limité par Windows à un toutes les 24 heures ; nécessite la protection du système |

---

## 15. Ajouter une fonctionnalité

**Un réglage de confidentialité** : ajouter une entrée à `privacy::SETTINGS` (clé, valeurs, valeur activée, valeur désactivée). L'interface, le score de santé et le point de restauration automatique le prennent en charge sans autre modification.

**Un réglage de jeu** : ajouter un `Tweak` à `gaming::TWEAKS` avec ses fonctions `get` et `set`, `admin`, `reboot` et `recommended`.

**Une application installable** : ajouter une ligne à `installer::APPS` après avoir vérifié l'identifiant avec `winget show --id <id> -e`.

**Un appel système** : déclarer la fonction dans `cpp/optimizer.h`, l'implémenter dans `cpp/optimizer.cpp` avec `extern "C"`, ajouter la déclaration et une enveloppe sûre dans `src/ffi.rs`, et la bibliothèque Windows nécessaire dans `build.rs`. Lors d'un ajout par script shell, vérifier que les barres obliques inverses des chaînes C++ sont conservées.

**Une page** : ajouter une variante à `Tab` (`main.rs`), une entrée à `NAV` (`layout.rs`) et une méthode `ui_*` dans `pages.rs` utilisant `style::page_header`, `style::section` et `style::toggle_row`. Appeler `self.before_change()` avant toute modification de Windows.
