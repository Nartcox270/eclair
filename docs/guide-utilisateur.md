# Éclair — Guide utilisateur

Version 1.1.0

Éclair est un utilitaire d'optimisation pour Windows 10 et Windows 11. Il regroupe dans une seule interface le nettoyage du disque, la gestion des priorités des programmes, les réglages Windows utiles au jeu, la confidentialité, les programmes lancés au démarrage et l'installation d'applications courantes.

Ce guide décrit l'installation, chaque écran de l'application, les précautions d'usage et la résolution des problèmes courants.

---

## Sommaire

1. [Configuration requise](#1-configuration-requise)
2. [Installation](#2-installation)
3. [Premier lancement](#3-premier-lancement)
4. [Présentation de l'interface](#4-présentation-de-linterface)
5. [Tableau de bord](#5-tableau-de-bord)
6. [Optimisation](#6-optimisation)
7. [Installation d'applications](#7-installation-dapplications)
8. [Réglages](#8-réglages)
9. [Fonctionnement en arrière-plan](#9-fonctionnement-en-arrière-plan)
10. [Sécurité et retour en arrière](#10-sécurité-et-retour-en-arrière)
11. [Désinstallation](#11-désinstallation)
12. [Dépannage](#12-dépannage)
13. [Questions fréquentes](#13-questions-fréquentes)

---

## 1. Configuration requise

| Élément | Exigence |
|---|---|
| Système | Windows 10 ou Windows 11, 64 bits |
| Droits | Compte administrateur (Éclair demande l'élévation à chaque lancement) |
| Espace disque | Environ 10 Mo |
| Installation d'applications | winget (« App Installer », présent par défaut sur Windows 11 et Windows 10 récents) |
| Point de restauration | Protection du système activée sur le lecteur C: |

---

## 2. Installation

### 2.1 Avec l'installateur (recommandé)

1. Lancez `Eclair-Setup-1.1.0.exe`.
2. Si Windows SmartScreen affiche « Windows a protégé votre ordinateur », cliquez sur **Informations complémentaires**, puis sur **Exécuter quand même**. Ce message apparaît parce que l'exécutable n'est pas signé numériquement ; il ne signale pas de problème détecté.
3. Choisissez si l'installation concerne uniquement votre compte (par défaut, sans demande administrateur) ou tous les utilisateurs.
4. Cochez éventuellement **Créer une icône sur le Bureau**.
5. Terminez l'assistant. Éclair peut être lancé directement depuis la dernière page.

Par défaut, Éclair est installé dans `%LOCALAPPDATA%\Programs\Eclair` et un raccourci est créé dans le menu Démarrer.

### 2.2 Version portable

L'archive `Eclair-v1.1.0.zip` contient `Eclair.exe`. Décompressez-la dans un dossier de votre choix et lancez l'exécutable. Aucune installation n'est nécessaire ; la désinstallation consiste à supprimer le dossier.

### 2.3 Installation silencieuse

Pour un déploiement sans interface :

```
Eclair-Setup-1.1.0.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /CURRENTUSER
```

---

## 3. Premier lancement

Au lancement, Windows affiche le contrôle de compte d'utilisateur (UAC). Acceptez-le : la plupart des fonctions d'Éclair modifient des paramètres système qui exigent les droits administrateur.

Si Éclair est lancé sans ces droits (par exemple depuis un compte standard), la mention **Mode limité : lancez en administrateur** apparaît en haut de la fenêtre. Les fonctions qui en dépendent sont alors grisées et signalées « Admin requis ».

Avant la première modification de Windows de chaque session, Éclair crée automatiquement un point de restauration du système (voir [section 10](#10-sécurité-et-retour-en-arrière)).

---

## 4. Présentation de l'interface

La fenêtre est composée de quatre zones :

| Zone | Rôle |
|---|---|
| Barre latérale (gauche) | Accès aux pages. Survolez une icône pour afficher son nom. L'icône de la page ouverte est entourée en vert. L'icône du bas ouvre les Réglages. |
| Barre de titre (haut) | Déplacement de la fenêtre par glisser, boutons Réduire et Fermer. |
| Zone principale | Contenu de la page sélectionnée. |
| Journal (bas) | Dernier message de l'application. Cliquez sur **Journal** pour afficher l'historique complet de la session. |

Pages disponibles, dans l'ordre de la barre latérale : Tableau de bord, Performance, Jeu, Nettoyage, Processus, Démarrage, Confidentialité, Installation, puis Réglages en bas.

Les réglages se présentent sous forme d'interrupteurs : vert signifie activé. Un interrupteur grisé n'est pas modifiable dans la situation actuelle (droits insuffisants, en général).

---

## 5. Tableau de bord

Le tableau de bord résume l'état de la machine.

| Élément | Description |
|---|---|
| Santé | Pourcentage des optimisations recommandées déjà appliquées (réglages de la page Jeu, hors intégrité de la mémoire, et réglages de la page Confidentialité). |
| Optimisations actives | Nombre d'optimisations recommandées appliquées. Le bouton ouvre la page Jeu. |
| Programmes au démarrage | Nombre de programmes lancés à l'ouverture de session. Affiché en rouge au-delà de 8. Le bouton ouvre la page Démarrage. |
| Jeux boostés | Nombre de jeux et applications prioritaires. Le bouton ouvre la page Processus. |
| Utilisation système | Courbes du processeur (vert), de la mémoire vive (blanc) et de la carte graphique (bleu) sur la dernière minute. |
| Statut du PC | Appréciation globale : « À optimiser » (moins de 40 %), « En forme » (de 40 à 74 %), « Optimisé » (75 % et plus), avec l'espace libre sur C: et la durée depuis le dernier démarrage. |

Actions rapides en bas de page :

| Action | Effet |
|---|---|
| Nettoyage | Ouvre la page Nettoyage. |
| Libérer la RAM | Vide la mémoire de travail des processus. |
| Mode Gaming | Active ou désactive le mode Gaming. La tuile indique « Gaming : auto » lorsque le mode a été activé automatiquement. |
| Restauration | Ouvre la page Réglages, section Restauration. |
| Installation | Ouvre la page Installation. |

---

## 6. Optimisation

### 6.1 Performance

**Actions rapides**

- **Libérer la mémoire vive** : demande à chaque processus de rendre sa mémoire de travail inutilisée. L'effet est immédiat mais temporaire ; les programmes rechargent ce dont ils ont besoin.
- **Vider le cache DNS** : efface les adresses de sites mémorisées. Utile lorsqu'un site ne se charge plus après un changement d'hébergement ou de réseau.

**Plan d'alimentation**

| Plan | Usage |
|---|---|
| Performances max | Plan créé par Éclair au premier usage : aucun cœur du processeur mis en veille et fréquence minimale à 100 %, uniquement sur secteur. Sur batterie, le comportement reste normal. |
| Performances élevées | Plan standard de Windows. Absent sur certains portables. |
| Équilibré | Plan par défaut de Windows. |
| Économie d'énergie | Réduit la consommation au détriment des performances. |

**Mémoire en attente (standby)**

Windows conserve en mémoire des données récemment utilisées (liste d'attente). Sur certaines configurations, une liste d'attente pleine combinée à peu de mémoire libre provoque des saccades en jeu.

- **Purger automatiquement** : vide la liste d'attente lorsqu'elle dépasse le premier seuil (1024 Mo par défaut) et que la mémoire libre est inférieure au second seuil (1024 Mo par défaut). Vérification toutes les secondes.
- **Purger maintenant** : purge immédiate.

Ces fonctions nécessitent les droits administrateur.

**Minuteur système**

- **Forcer la résolution la plus fine** (généralement 0,5 ms) : réduit la latence de certains jeux. Le réglage s'applique tant qu'Éclair est en cours d'exécution ; Windows rétablit la valeur normale à sa fermeture. Il augmente légèrement la consommation d'énergie.
- **Résolution globale** (Windows 11) : permet à la résolution demandée de s'appliquer à tout le système, y compris lorsque la fenêtre d'Éclair est réduite. Nécessite un redémarrage et les droits administrateur.

### 6.2 Jeu

Chaque réglage de cette page a un effet documenté par Microsoft ou mesurable. Un interrupteur activé signifie que l'optimisation est appliquée.

| Réglage | Effet | Conditions |
|---|---|---|
| Optimisations pour les jeux fenêtrés | Les jeux DirectX 10 et 11 en mode fenêtré ou sans bordure utilisent le modèle de présentation « flip » : latence réduite, prise en charge de l'affichage à fréquence variable et de l'Auto HDR. | — |
| Planification GPU matérielle (HAGS) | La carte graphique gère sa propre mémoire vidéo. Requise pour la génération d'images DLSS 3 et FSR 3. | Administrateur, redémarrage |
| Couper l'enregistrement Xbox en arrière-plan | La Xbox Game Bar cesse d'enregistrer le jeu en continu, ce qui réduit la charge sur la carte graphique et le disque. | — |
| Mode Jeu de Windows | Windows donne la priorité au jeu et diffère les mises à jour pendant la partie. Activé par défaut dans Windows. | — |
| Désactiver l'accélération de la souris | Désactive « Améliorer la précision du pointeur » : un même geste produit toujours le même déplacement, ce qui facilite la visée. Effet immédiat. | — |
| Désactiver l'intégrité de la mémoire | Gain de quelques pour cent dans certains jeux. Microsoft présente cette option pour le jeu mais précise qu'elle rend l'ordinateur plus vulnérable. Ce réglage n'est pas compté dans le score de santé. | Administrateur, redémarrage |

Avertissement : la désactivation de l'intégrité de la mémoire affaiblit la protection de Windows contre les pilotes malveillants. Réactivez-la après vos sessions de jeu.

### 6.3 Nettoyage

Sélectionnez les éléments à traiter, puis :

- **Analyser** calcule l'espace récupérable sans rien supprimer.
- **Nettoyer** supprime les fichiers et indique l'espace libéré dans le journal.

| Élément | Contenu | Remarque |
|---|---|---|
| Fichiers temporaires utilisateur | Dossier `%TEMP%` de la session | Les fichiers en cours d'utilisation sont ignorés. |
| Fichiers temporaires Windows | Dossier `C:\Windows\Temp` | Droits administrateur conseillés. |
| Corbeille | Contenu de la corbeille de tous les lecteurs | Suppression définitive. |
| Anciens composants des mises à jour Windows | Nettoyage du magasin de composants par l'outil DISM de Microsoft | Droits administrateur requis. Durée de plusieurs minutes. |

### 6.4 Processus

**Modes**

| Mode | Comportement |
|---|---|
| Désactivé | Aucune priorité n'est modifiée automatiquement. |
| Équilibré | Lorsque le processeur est chargé (40 % ou plus), un programme qui occupe une part importante du processeur pendant deux secondes consécutives passe en priorité « sous la normale ». Il retrouve sa priorité dès qu'il se calme, que la charge globale diminue, ou que vous passez sur sa fenêtre. |
| Gaming | Les jeux et applications prioritaires passent en priorité haute. Les autres programmes en priorité normale passent en priorité « sous la normale ». La fenêtre au premier plan, l'Explorateur Windows, le gestionnaire de fenêtres et le service audio ne sont jamais ralentis. |

Dans tous les cas, les priorités d'origine sont rendues lorsque le mode change, lorsque vous quittez Éclair, et au lancement suivant si Éclair a été arrêté brutalement.

**Gaming automatique**

Lorsque le mode est « Désactivé » et que cette option est active, Éclair passe automatiquement en mode Gaming dès qu'un jeu prioritaire est affiché en plein écran au premier plan. Le mode est coupé 15 secondes après la sortie du plein écran, ce qui évite les bascules lors d'un passage rapide vers une autre fenêtre.

**Jeux et applications prioritaires**

Discord figure dans la liste par défaut. Pour ajouter un programme :

- saisissez le nom de son exécutable (par exemple `jeu.exe`) puis cliquez sur **Ajouter** ;
- ou cliquez sur l'étoile à gauche de son nom dans la liste des programmes.

Cliquez sur un élément de la liste pour le retirer.

**Liste des programmes**

La liste affiche chaque programme accessible avec son utilisation du processeur, sa mémoire et sa priorité, triée du plus gourmand au moins gourmand. Le menu de priorité permet de la modifier manuellement. La priorité « Temps réel » n'est pas proposée : elle peut bloquer l'ensemble du système.

### 6.5 Démarrage

Cette page liste les programmes lancés à l'ouverture de session (pour votre compte et pour tous les utilisateurs). Désactiver un programme l'empêche de démarrer automatiquement sans le désinstaller ; le mécanisme est identique à celui du Gestionnaire des tâches de Windows, qui affichera le même état.

Les entrées « Tous les utilisateurs » nécessitent les droits administrateur.

Non couverts : les raccourcis du dossier « Démarrage » et les tâches planifiées.

### 6.6 Confidentialité

Cette page coupe les fonctions qui envoient des données d'utilisation à Microsoft ou affichent des suggestions et publicités. Un interrupteur désactivé signifie que la fonction est coupée.

**Confidentialité** : localisation, télémétrie, programme d'amélioration de l'expérience utilisateur, télémétrie des applications, suivi du lancement des applications, amélioration et personnalisation de l'écriture et de la saisie, rapports d'erreurs, reconnaissance vocale en ligne.

**Publicités et suggestions** : expériences personnalisées, identifiant de publicité, contenu suggéré dans les paramètres, suggestions dans le menu Démarrer, notifications suggérées, messages sur l'écran de verrouillage.

La télémétrie, le programme d'amélioration, la télémétrie des applications et les rapports d'erreurs s'appliquent à toute la machine et nécessitent les droits administrateur. Certains changements prennent effet après un redémarrage.

---

## 7. Installation d'applications

La page Installation installe des applications courantes dans leur dernière version officielle, sans fenêtre ni question, au moyen de winget.

1. Cochez les applications souhaitées. Les applications déjà installées apparaissent cochées et grisées.
2. Cliquez sur **Installer la sélection**. La progression s'affiche dans le journal.

Le bouton **Tout mettre à jour** met à jour l'ensemble des applications installées que winget sait gérer.

| Catégorie | Applications |
|---|---|
| Navigateurs | Brave, Firefox, Google Chrome, Opera GX |
| Communication | Discord, TeamSpeak, Telegram |
| Jeux | Steam, Epic Games Launcher, EA App, Ubisoft Connect, GOG Galaxy, Battle.net, Medal |
| Utilitaires | 7-Zip, WinRAR, VLC, Notepad++, Everything, PowerToys, qBittorrent, LibreOffice |
| Multimédia | OBS Studio, Spotify |
| Matériel | MSI Afterburner, CPU-Z, GPU-Z, HWiNFO, CrystalDiskInfo |
| Développement | Visual Studio Code, Git |

Lancer une installation vaut acceptation de la licence de chaque application installée. Certaines applications sont payantes au-delà d'une période d'essai (WinRAR, par exemple).

---

## 8. Réglages

**Application des réglages**

| Option | Comportement |
|---|---|
| Temporaire (par défaut) | Les modes et réglages d'Éclair cessent à sa fermeture. Les priorités modifiées sont rendues. |
| Persistant | Le mode de priorité, le Gaming automatique, la purge mémoire et le minuteur sont rétablis à chaque lancement ; les applications prioritaires sont enregistrées en priorité haute dans Windows (elles démarrent en priorité haute même sans Éclair) ; Éclair se lance automatiquement, caché dans la zone de notification, à l'ouverture de session. |

Les réglages des pages Jeu, Confidentialité et Démarrage sont des paramètres Windows : ils restent en place quel que soit ce choix, jusqu'à ce que vous les modifiiez à nouveau.

Repasser en mode Temporaire efface les priorités enregistrées dans Windows et retire le lancement automatique.

**Restauration** : création manuelle d'un point de restauration et accès à l'outil Restauration du système de Windows (voir [section 10](#10-sécurité-et-retour-en-arrière)).

**À propos** : version, état des droits administrateur, et bouton **Quitter Éclair**.

---

## 9. Fonctionnement en arrière-plan

Les modes de priorité, le Gaming automatique, la purge de la mémoire en attente et le minuteur ne fonctionnent que tant qu'Éclair est en cours d'exécution. Pour cette raison :

- le bouton Fermer de la fenêtre masque Éclair au lieu de le quitter ; une notification le rappelle la première fois ;
- l'icône d'Éclair reste dans la zone de notification (à côté de l'horloge ; elle peut se trouver dans le menu des icônes masquées, accessible par la flèche) ;
- un clic sur l'icône rouvre la fenêtre ;
- un clic droit sur l'icône propose **Ouvrir** et **Quitter**.

Pour arrêter complètement Éclair, utilisez **Quitter** dans le menu de l'icône ou dans la page Réglages.

---

## 10. Sécurité et retour en arrière

### 10.1 Point de restauration

Avant la première modification de Windows de chaque session (page Jeu, Confidentialité, Démarrage, plan d'alimentation, résolution globale du minuteur, passage en mode Persistant), Éclair crée un point de restauration nommé « Eclair ».

Windows n'autorise qu'un point de restauration toutes les 24 heures par défaut. Si un point récent existe, Éclair l'indique dans le journal et le point existant sert de référence.

Pour annuler l'ensemble des réglages appliqués :

1. Ouvrez **Réglages**, puis **Ouvrir la Restauration du système**.
2. Choisissez le point « Eclair » correspondant.
3. Suivez l'assistant. L'ordinateur redémarre.

La création d'un point de restauration nécessite les droits administrateur et la protection du système activée sur le lecteur C: (Panneau de configuration, Système, Protection du système).

### 10.2 Arrêt inattendu

Si Éclair est arrêté brutalement (fin de tâche, plantage, coupure) alors qu'un mode de priorité est actif, les priorités modifiées sont notées sur le disque et rendues automatiquement au lancement suivant. Le journal indique alors le nombre de priorités rendues.

### 10.3 Ce qu'Éclair ne fait pas

- Il ne supprime aucun fichier personnel : le nettoyage se limite aux dossiers temporaires, à la corbeille et au magasin de composants de Windows.
- Il ne désinstalle aucun programme.
- Il ne modifie aucun réglage sans action de votre part.
- Il ne transmet aucune donnée.

---

## 11. Désinstallation

Utilisez **Paramètres Windows, Applications, Applications installées, Éclair, Désinstaller**, ou le raccourci **Désinstaller Éclair** du menu Démarrer.

La désinstallation supprime l'application, ses raccourcis, ses réglages (`%APPDATA%\Eclair`) et la tâche de lancement automatique.

Elle n'annule pas les paramètres Windows modifiés avec Éclair (pages Jeu, Confidentialité, Démarrage, plan d'alimentation). Pour les rétablir, remettez les interrupteurs dans leur état d'origine avant de désinstaller, ou utilisez le point de restauration. Si le mode Persistant était actif, repassez en mode Temporaire avant la désinstallation afin d'effacer les priorités enregistrées dans Windows.

---

## 12. Dépannage

| Problème | Solution |
|---|---|
| « Windows a protégé votre ordinateur » au lancement de l'installateur | Cliquez sur **Informations complémentaires**, puis **Exécuter quand même**. L'exécutable n'est pas signé. |
| « Mode limité » affiché en haut de la fenêtre | Éclair n'a pas les droits administrateur. Relancez-le et acceptez la demande de contrôle de compte. |
| Un interrupteur est grisé | Le réglage demande les droits administrateur, ou n'est pas applicable sur cette machine. |
| « Impossible d'activer Performances élevées » | Ce plan n'existe pas sur certains portables. Utilisez « Performances max ». |
| « Point de restauration impossible » | Activez la protection du système sur le lecteur C:, puis réessayez. |
| La mémoire en attente indique « Lecture impossible » | Éclair n'a pas les droits administrateur. |
| La courbe de la carte graphique n'apparaît pas | Le compteur de performance « GPU Engine » n'est pas disponible (pilote graphique ancien ou absent). |
| « winget introuvable » | Installez « App Installer » depuis le Microsoft Store. |
| Une installation d'application échoue | Vérifiez la connexion internet et que l'application n'est pas en cours d'exécution, puis relancez l'installation. Le code d'erreur figure dans le journal. |
| Éclair semble ne pas se fermer | Le bouton Fermer masque la fenêtre. Utilisez **Quitter** dans le menu de l'icône de la zone de notification. |
| Un réglage n'a pas d'effet | Plusieurs réglages nécessitent un redémarrage ; c'est indiqué dans leur description. |

---

## 13. Questions fréquentes

**Éclair augmente-t-il le nombre d'images par seconde ?**
Les gains dépendent de la configuration et restent généralement modestes. Les réglages graphiques du jeu ont un effet bien plus important. Éclair améliore surtout la régularité et la réactivité, et réduit l'activité en arrière-plan.

**Les réglages restent-ils après un redémarrage ?**
Les paramètres Windows (pages Jeu, Confidentialité, Démarrage, plan d'alimentation) restent en place. Les modes de priorité, la purge mémoire et le minuteur nécessitent qu'Éclair soit en cours d'exécution ; activez le mode Persistant pour qu'il démarre automatiquement.

**Pourquoi Éclair demande-t-il les droits administrateur ?**
La modification des paramètres machine, la purge de la mémoire, le changement de priorité de certains programmes et la création de points de restauration sont réservés aux administrateurs par Windows.

**Puis-je utiliser Éclair avec un logiciel antitriche ?**
Éclair ne lit ni ne modifie la mémoire des jeux ; il change uniquement des priorités et des paramètres Windows. Les règles des éditeurs de jeux pouvant varier, en cas de doute, n'ajoutez pas le jeu à la liste des applications prioritaires.

**Où sont stockés les réglages d'Éclair ?**
Dans `%APPDATA%\Eclair\config.txt`. Le fichier `priorites.txt`, présent uniquement pendant qu'un mode modifie des priorités, sert à la reprise après un arrêt inattendu.
