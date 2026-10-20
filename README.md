# Éclair

Optimiseur pour Windows 10 et 11. Interface graphique en Rust (egui), fonctions système en C++ (API Win32).

## Documentation

- [Guide utilisateur](docs/guide-utilisateur.md) : installation, utilisation de chaque page, sécurité, désinstallation, dépannage.
- [Documentation technique](docs/documentation-technique.md) : architecture, modules, interface C++, moteur de priorités, modifications apportées à Windows, compilation, tests, publication.

## Fonctions

**Tableau de bord** : score de santé, optimisations actives, programmes au démarrage, graphique processeur, mémoire et carte graphique, actions rapides.

**Optimisation**
- **Performance** : libération de la mémoire vive, cache DNS, plans d'alimentation (dont « Performances max »), purge automatique de la mémoire en attente, minuteur système.
- **Jeu** : optimisations pour les jeux fenêtrés, planification GPU matérielle, enregistrement Xbox, Mode Jeu, accélération de la souris, intégrité de la mémoire.
- **Nettoyage** : fichiers temporaires, corbeille, anciens composants des mises à jour Windows (DISM), avec analyse préalable.
- **Processus** : priorités des programmes, modes Équilibré et Gaming, Gaming automatique en plein écran.
- **Démarrage** : activation et désactivation des programmes lancés à l'ouverture de session.
- **Confidentialité** : télémétrie, identifiant de publicité, suggestions, localisation.

**Installation** : installation et mise à jour des applications courantes via winget.

**Sécurité** : point de restauration automatique avant la première modification de chaque session, restitution des priorités après un arrêt inattendu.

**Arrière-plan** : la fermeture de la fenêtre laisse Éclair actif dans la zone de notification.

**Réglages temporaires ou persistants** : en mode persistant, Éclair démarre à l'ouverture de session et rétablit ses réglages.

## Installation

Lancez `Eclair-Setup-<version>.exe`, ou décompressez `Eclair-v<version>.zip` et lancez `Eclair.exe`. Éclair demande les droits administrateur au lancement. Ses réglages sont stockés dans `%APPDATA%\Eclair`.

## Compilation

Prérequis : [Rust](https://rustup.rs) et Visual Studio Build Tools (charge de travail « Développement Desktop en C++ »).

```
cargo build --release                        # target\release\Eclair.exe
cargo test                                   # tests (en debug)
cargo test stress -- --ignored --nocapture   # test de charge du moteur de priorités
```

L'installateur se construit avec [Inno Setup](https://jrsoftware.org/isinfo.php) à partir de `installer\eclair.iss`. Le logo se régénère avec `assets\make_icon.ps1`. La procédure complète de publication figure dans la [documentation technique](docs/documentation-technique.md#13-publication).

## Structure

```
assets/       logo, ressources et manifeste Windows
cpp/          cœur C++ (Win32)
docs/         documentation
installer/    script Inno Setup
src/          application Rust
build.rs      compilation du C++ et des ressources
```
