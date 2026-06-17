# NiTriTe 2.0 — Documentation Utilisateur

## Démonstration

<video src="https://media.heiphaistos.org/videos/nitrite.mp4" controls width="100%" preload="none"></video>
## Table des matières

1. [Présentation](#présentation)
2. [Installation & Démarrage](#installation--démarrage)
3. [Système](#système)
   - [Tableau de bord](#-tableau-de-bord)
   - [Diagnostic](#-diagnostic)
   - [Optimisations & Mode Turbo](#-optimisations--mode-turbo)
4. [Logiciels](#logiciels)
   - [Applications](#-applications)
   - [Outils Système](#-outils-système)
   - [Master Install](#-master-install)
   - [Apps Portables](#-apps-portables)
   - [OS & USB Tools](#-os--usb-tools)
5. [Performance](#performance)
   - [Températures & Capteurs](#-températures--capteurs)
   - [Benchmark](#-benchmark)
   - [Historique Performance](#-historique-performance)
6. [Avancé](#avancé-beta)
   - [Clonage Système](#-clonage-système)
   - [Récupération de Données](#-récupération-de-données)
   - [Visualiseur Disque](#-visualiseur-disque)
   - [Doublons](#-détecteur-de-doublons)
   - [Gros Fichiers](#-gros-fichiers)
   - [Hash Checker](#-hash-checker)
   - [Boot Manager](#-boot-manager)
   - [Éditeur Hosts](#-éditeur-hosts)
   - [Analyse BSOD](#-analyse-bsod)
   - [WSL Linux](#-wsl-linux)
   - [Points de Restauration](#-points-de-restauration)
   - [Docker Manager](#-docker-manager)
7. [Maintenance](#maintenance)
   - [Mises à jour](#-mises-à-jour)
   - [Drivers](#-drivers)
   - [Désinstallateur](#-désinstallateur)
   - [Nettoyeur Avancé](#-nettoyeur-avancé)
   - [Sauvegarde](#-sauvegarde)
   - [Scan Antivirus](#-scan-antivirus)
   - [Dépendances](#-dépendances)
8. [Réseau & Terminal](#réseau--terminal)
   - [Réseau](#-réseau)
   - [DNS Switcher](#-dns-switcher)
   - [WiFi Analyzer](#-wifi-analyzer)
   - [Scanner de Ports](#-scanner-de-ports)
   - [Bluetooth](#-bluetooth)
   - [Terminal](#-terminal)
   - [Scripts & Snippets](#-scripts--snippets)
9. [Intelligence](#intelligence)
   - [Agent IA](#-agent-ia)
   - [Base de Connaissances](#-base-de-connaissances)
10. [Configuration](#configuration)
    - [Paramètres](#-paramètres)
    - [Profils](#-profils)
    - [Éditeur de Thème](#-éditeur-de-thème)
    - [Logs](#-logs)
11. [Mode WinPE](#-mode-winpe)
12. [Raccourcis Clavier](#raccourcis-clavier)
13. [Stack Technique](#stack-technique)

---

## Présentation

**NiTriTe 2.0** est un outil de diagnostic et maintenance Windows complet, conçu pour les techniciens informatiques, administrateurs système et utilisateurs avancés. Il regroupe en une seule application native plus de **50 fonctionnalités** couvrant :

- Monitoring temps réel du matériel
- Diagnostic système approfondi (CPU, RAM, disques, GPU, réseau)
- Installation et désinstallation de logiciels
- Nettoyage, optimisation et sauvegarde
- Réparation système depuis WinPE (Windows Preinstallation Environment)
- Analyse réseau et sécurité
- Récupération de données et clonage disque

L'application fonctionne **entièrement en local**, sans envoi de données vers des serveurs externes.

---

## Installation & Démarrage

### Prérequis
- **Windows 10** (build 1903+) ou **Windows 11**
- **Architecture 64-bit** (x86_64)
- **Droits administrateur** recommandés pour toutes les fonctionnalités

### Installation
1. Télécharger l'installeur `Nitrite_6.0.0_x64-setup.exe`
2. Exécuter en tant qu'administrateur
3. Suivre l'assistant d'installation
4. Lancer **NiTriTe** depuis le raccourci Bureau ou le menu Démarrer

### Démarrage rapide
Au premier lancement, le **Tableau de bord** s'affiche automatiquement avec le monitoring temps réel. Utilisez la barre de navigation à gauche pour accéder aux différentes fonctionnalités.

> **Conseil :** Appuyez sur `Ctrl+K` pour ouvrir la recherche rapide et accéder instantanément à n'importe quelle fonctionnalité.

---

## Système

### 🏠 Tableau de bord

**Route :** `/` — Page d'accueil de l'application.

Le tableau de bord fournit une vue d'ensemble en temps réel de l'état du système :

| Métrique | Description |
|---|---|
| **CPU** | Utilisation globale + usage par cœur |
| **RAM** | Mémoire utilisée / totale + type (DDR4, DDR5) |
| **Disque** | Taux d'occupation de la partition C: |
| **Réseau** | Débit download/upload en temps réel |

**Fonctionnalités :**
- **Score de santé système** (0–100) avec code couleur (vert/orange/rouge)
- **Graphiques Sparkline** historique 60 secondes pour CPU, RAM, Disque, Réseau
- **Barre d'informations système** : OS, Machine, Architecture, CPU (cœurs/threads/GHz), RAM type, Uptime, Nombre de processus
- **Moteur de suggestions** : recommandations automatiques basées sur les métriques (RAM élevée → Turbo Mode, Disque plein → Nettoyeur, température critique → Températures)
- **Top processus** avec kill direct depuis le tableau de bord
- **Monitoring GPU** : utilisation, VRAM, température
- **Températures** CPU, GPU et disques avec barres de progression colorées
- **Débits disque** : lecture/écriture en KB/s
- **Alertes actives** : notifications si CPU/RAM/Disque dépassent les seuils configurés
- **Actions rapides** : accès en 1 clic vers Diagnostic, Nettoyage, Mises à jour, Sauvegarde, Antivirus

**Configuration des seuils :** Cliquez sur ⚙ **Seuils** pour personnaliser les niveaux d'alerte (CPU warn/critique, RAM warn/critique, Disque warn/critique).

---

### 🩺 Diagnostic

**Route :** `/diagnostic` — Analyse complète du système.

Le diagnostic effectue une collecte exhaustive des informations matérielles et logicielles, organisées en onglets :

| Onglet | Contenu |
|---|---|
| **Vue d'ensemble** | Score global, résumé CPU/RAM/Disque/GPU |
| **Processus** | Liste des processus actifs (PID, CPU%, RAM, chemin) |
| **Services** | Services Windows (état, type de démarrage, activation/désactivation) |
| **Démarrage** | Programmes au démarrage (registre + dossier Startup) |
| **Tâches planifiées** | Tâches Windows Scheduler actives et inactives |
| **Logiciels** | Applications installées avec éditeur, version, taille |
| **Variables Env.** | Variables d'environnement système et utilisateur |
| **Stockage** | Disques physiques, partitions, volumes (SMART, état de santé) |
| **Réseau** | Interfaces, adresses IP, passerelle, DNS, débit |
| **GPU** | Informations carte graphique, utilisation, VRAM, température |
| **Écrans** | Moniteurs connectés, résolution, fréquence de rafraîchissement |
| **Audio** | Périphériques audio, pilotes, état |
| **USB** | Périphériques USB connectés |
| **Batterie** | État batterie, capacité, cycles de charge (si applicable) |
| **Énergie** | Plan d'alimentation actif, paramètres veille/hibernation |
| **Imprimantes** | Imprimantes installées et leur état |
| **Drivers** | Pilotes système avec état, version, fournisseur |
| **Sécurité** | État Windows Defender, pare-feu, UAC, Secure Boot |
| **Bluetooth** | Adaptateurs et périphériques Bluetooth |

**Export :** Les résultats de diagnostic peuvent être exportés en rapport texte/HTML.

---

### ⚡ Optimisations & Mode Turbo

**Route :** `/optimizations` — Profils d'optimisation en 1 clic.

#### Modes prédéfinis
| Mode | Description | Actions |
|---|---|---|
| **🔥 Turbo** | Performances maximales (usage général) | Plan haute perf, vide DNS et RAM, GPU Scheduling, termine processus inutiles |
| **🎮 Gaming** | Optimisé FPS | Haute performance, GPU HW Scheduling, Game Mode ON, Xbox Game Bar OFF |
| **💼 Travail** | Équilibre perf/batterie | Plan équilibré, effets visuels réduits, presse-papiers nettoyé |
| **🌿 Économie** | Batterie maximale | Plan économie d'énergie, luminosité réduite |

**Avant/Après :** Affiche la comparaison RAM utilisée avant et après application du mode.

#### Optimisations individuelles
Appliquez des optimisations à la carte :

| Action | Effet |
|---|---|
| Nettoyer fichiers temporaires | Vide `%TEMP%` et `C:\Windows\Temp` |
| Vider le cache DNS | `ipconfig /flushdns` |
| Vider journaux d'événements | Efface logs Application/System/Security |
| Optimiser démarrage SSD | Désactive Superfetch/SysMain |
| Limiter la télémétrie | Réduit la collecte de données Microsoft |
| Effets visuels Performance | Désactive animations Windows |
| Optimiser disques (TRIM) | Lance TRIM sur les volumes SSD |
| Vider le presse-papiers | Efface le contenu du presse-papiers |

**Historique des gains :** Conserve un journal des optimisations passées avec la RAM libérée à chaque session (30 dernières sessions).

---

## Logiciels

### 📦 Applications

**Route :** `/applications` — Installation d'applications tierces.

Catalogue d'applications courantes téléchargeables et installables silencieusement, organisées par catégorie (Navigateurs, Développement, Multimédia, Utilitaires, etc.).

---

### 🔧 Outils Système

**Route :** `/tools` — Scripts `.bat` système prêts à l'emploi.

Collection de scripts d'administration Windows exécutables directement depuis l'interface : nettoyage, réparation, configuration réseau, gestion des services, etc.

---

### 📥 Master Install

**Route :** `/master-install` — Installation groupée de logiciels.

Sélectionnez une liste d'applications et installez-les toutes en une seule fois silencieusement. Idéal pour configurer rapidement un nouveau poste.

---

### 📱 Apps Portables

**Route :** `/portables` — Applications portables intégrées.

Accès direct aux outils portables embarqués dans NiTriTe, sans installation requise :

| Outil | Utilité |
|---|---|
| Autoruns64 | Gestion des programmes au démarrage |
| CPU-Z | Informations matérielles détaillées |
| CrystalDiskInfo | Santé des disques (SMART) |
| HWiNFO | Monitoring matériel avancé |
| HWMonitor | Températures, tensions, ventilateurs |
| ProcessExplorer | Gestionnaire de processus avancé |
| AdwCleaner | Suppression d'adwares et PUPs |
| DDU | Désinstallation complète de pilotes GPU |
| BCUninstaller | Désinstallateur avancé |
| GetDataBack | Récupération de données |
| WiseCare365 | Nettoyage et optimisation |
| WiseDiskCleaner | Nettoyeur de disque |
| Spybot | Anti-spyware |
| HardDiskSentinel | Surveillance disques durs |

---

### 💿 OS & USB Tools

**Route :** `/os-downloads` — Téléchargement d'images système.

Liens vers les ISO officielles des systèmes d'exploitation (Windows 10, Windows 11, distributions Linux) et outils de création de clés USB bootables (Rufus, Ventoy, etc.).

---

## Performance

### 🌡 Températures & Capteurs

**Route :** `/temperatures` — Monitoring matériel en temps réel.

Affiche toutes les données des capteurs matériels, organisées par composant :

| Type de capteur | Exemples |
|---|---|
| **Températures** | CPU par cœur, GPU, M.2, disques durs |
| **Ventilateurs** | Vitesse en RPM (CPU fan, chassis fan) |
| **Charge** | Utilisation CPU total et par cœur, GPU |
| **Fréquences** | Fréquence d'horloge CPU et GPU (MHz/GHz) |
| **Tensions** | Tensions CPU, DRAM, VCore (Volts) |
| **Consommation** | Puissance CPU, GPU, package (Watts) |

> **LibreHardwareMonitor :** Pour les données complètes (températures par cœur, ventilateurs RPM, tensions), installez **LibreHardwareMonitor** et lancez-le en administrateur. NiTriTe le détecte automatiquement.

**Code couleur températures :**
- 🟢 < 50°C — Normal
- 🟡 50–70°C — Chaud
- 🟠 70–85°C — Très chaud
- 🔴 > 85°C — Critique

Actualisation automatique toutes les **3 secondes**.

---

### 📊 Benchmark

**Route :** `/benchmark` — Tests de performance.

Benchmarks natifs (sans dépendances tierces) :

| Test | Ce qu'il mesure |
|---|---|
| **CPU Mono-cœur** | Performance d'un seul cœur (chiffrement, calcul) |
| **CPU Multi-cœur** | Performance parallèle (tous les cœurs) |
| **RAM Bande passante** | Débit mémoire en GB/s |
| **Disque Séquentiel** | Vitesse lecture/écriture séquentielle |
| **Compression** | Vitesse de compression données |

Comparaison avec des scores de référence pour évaluer les performances.

---

### 📈 Historique Performance

**Route :** `/perf-history` — Suivi longitudinal des performances.

Enregistre et visualise l'évolution des métriques système dans le temps (CPU, RAM, disque, réseau) pour identifier les dégradations progressives de performance.

---

## Avancé (BETA)

### 💾 Clonage Système

**Route :** `/clone` — Clonage disque complet.

Clone un disque/partition vers un autre support via **wbadmin** (Windows Backup) ou **Robocopy**. Utile pour migrer vers un nouveau SSD ou créer une image de secours.

---

### 🗄 Récupération de Données

**Route :** `/data-recovery` — Récupération de fichiers supprimés.

Recherche et récupère les fichiers supprimés depuis :
- La **Corbeille** Windows
- Les **Shadow Copies** (VSS — Volume Shadow Service)
- Les partitions formatées (scan avancé)

Fonctionnalité de **comparaison Shadow Copy** : visualise les différences entre une version précédente et l'état actuel d'un fichier ou dossier.

---

### 🥧 Visualiseur Disque

**Route :** `/disk-visualizer` — Cartographie visuelle de l'espace disque.

Affiche l'espace disque sous forme graphique interactive (treemap), permettant d'identifier rapidement les dossiers et fichiers les plus volumineux.

---

### 🔍 Détecteur de Doublons

**Route :** `/duplicate-finder` — Détection et suppression de fichiers en double.

Analyse un dossier (ou le disque entier) pour trouver les fichiers identiques (par hash MD5/SHA), affiche les doublons groupés et permet leur suppression sélective.

---

### 📁 Gros Fichiers

**Route :** `/big-files` — Recherche des fichiers volumineux.

Scanne un dossier et liste les fichiers dépassant un seuil configurable (50 Mo, 100 Mo, 500 Mo, 1 Go), triés par taille décroissante. Lien direct pour ouvrir le dossier contenant.

---

### # Hash Checker

**Route :** `/hash-checker` — Vérification d'intégrité de fichiers.

Calcule et compare les empreintes cryptographiques d'un fichier :
- **MD5, SHA-1, SHA-256, SHA-512**

Utile pour vérifier l'intégrité d'un ISO téléchargé ou d'un fichier reçu.

---

### 🖥 Boot Manager

**Route :** `/boot-manager` — Gestion du démarrage Windows.

Consulte et modifie la configuration du démarrage Windows (BCD — Boot Configuration Data) :
- Liste les entrées de démarrage
- Modifie le timeout du menu
- Ajoute/supprime des entrées de démarrage

---

### 🌐 Éditeur Hosts

**Route :** `/hosts-editor` — Édition du fichier hosts.

Éditeur visuel du fichier `C:\Windows\System32\drivers\etc\hosts` :
- Ajouter/modifier/supprimer des entrées
- Activer/désactiver des entrées sans les supprimer
- Validation du format IP + domaine
- Sauvegarde automatique avant modification

---

### 💥 Analyse BSOD

**Route :** `/bsod-analyzer` — Analyse des écrans bleus de la mort.

Analyse les fichiers de dump mémoire (`C:\Windows\Minidump\`) pour identifier la cause des BSOD :
- Code d'erreur (Bugcheck)
- Module/Driver responsable
- Date et heure de chaque crash
- Description de l'erreur et pistes de résolution

---

### 🐧 WSL Linux

**Route :** `/wsl` — Gestion du sous-système Linux.

Gère les distributions WSL (Windows Subsystem for Linux) :
- Lister les distributions installées et leur état
- Démarrer/arrêter une distribution
- Ouvrir un terminal WSL
- Exporter/importer des distributions

---

### 🛡 Points de Restauration

**Route :** `/restore-points` — Gestion des points de restauration.

- Liste tous les points de restauration système disponibles
- Créer un nouveau point de restauration
- Restaurer le système à un point antérieur
- Supprimer des points de restauration obsolètes

---

### 🐳 Docker Manager

**Route :** `/docker` — Interface Docker Desktop.

Gestion des conteneurs et images Docker sans ligne de commande :
- Liste des conteneurs (actifs/arrêtés)
- Démarrer/arrêter/supprimer un conteneur
- Liste des images locales
- Statistiques d'utilisation ressources

---

## Maintenance

### 🔄 Mises à jour

**Route :** `/updates` — Gestionnaire de mises à jour multi-sources.

Vérifie et installe les mises à jour depuis plusieurs gestionnaires de paquets :

| Source | Fonctionnalités |
|---|---|
| **WinGet** | Liste les apps avec mises à jour disponibles, mise à jour individuelle ou globale |
| **Chocolatey** | Vérifie les upgrades Choco disponibles, mise à jour groupée |
| **Scoop** | Gestion des paquets Scoop |
| **Windows Update** | Déclenchement des mises à jour Windows via PowerShell |
| **Automatisation** | Planification de mises à jour automatiques (daily/weekly) |

**Options :**
- Exclure des packages de la mise à jour (persisté en local)
- Créer un point de restauration avant mise à jour
- Rollback de la dernière mise à jour Windows

---

### 🖥 Drivers

**Route :** `/drivers` — Gestion des pilotes.

- Liste tous les pilotes installés avec version, date, fournisseur
- Identification des pilotes obsolètes ou manquants
- Lancement de Windows Update pour les pilotes
- Accès au Gestionnaire de périphériques

---

### 🗑 Désinstallateur

**Route :** `/uninstaller` — Désinstallation avancée.

Désinstallateur plus puissant que le Panneau de configuration Windows :

**Onglet Logiciels :**
- Recherche et filtre les applications installées
- Désinstallation silencieuse (détecte NSIS, Inno Setup, MSI, etc.)
- Suppression des résidus post-désinstallation (registre, fichiers orphelins)
- Désinstallation groupée

**Onglet DLL Scanner :**
- Scanne les DLL présentes dans System32 et dossiers d'applications
- Catégorise : Système (Microsoft), Tiers (System32), Applications
- Identification des DLL orphelines
- Suppression sélective avec avertissement

---

### ✨ Nettoyeur Avancé

**Route :** `/cleaner` — Libération d'espace disque.

#### Mode Nettoyage
Analyse et supprime les fichiers inutiles :

| Catégorie | Cibles |
|---|---|
| **Temp** | `%TEMP%`, `C:\Windows\Temp` |
| **Cache système** | Prefetch, WER, thumbnails |
| **Navigateurs** | Cache Chrome, Edge, Firefox |
| **Windows Update** | Cache `SoftwareDistribution\Download` |
| **Logs** | Journaux d'événements, dumps mémoire |
| **Corbeille** | Vider la corbeille |

#### Mode Quarantaine 🆕
Avant de supprimer définitivement, déplacez les fichiers vers une zone de quarantaine (`%LOCALAPPDATA%\NiTriTe\quarantine\`) pour vérification. Restaurez ou supprimez définitivement depuis le panneau Quarantaine.

#### Gros fichiers
Scan manuel de `C:\Users` pour trouver les fichiers dépassant un seuil (50 Mo–1 Go). Ouvrir le dossier directement.

---

### 💾 Sauvegarde

**Route :** `/backup` — Sauvegarde de la configuration système.

Sauvegardez des informations critiques du système dans un fichier archive :

| Catégorie | Éléments disponibles |
|---|---|
| **Logiciels & Système** | Apps installées, export WinGet JSON, drivers, features Windows, polices |
| **Réseau & Sécurité** | Config IP/DNS, profils WiFi, règles pare-feu, partages réseau, fichier hosts |
| **Licences** ⚠️ | Clé Windows, clés BitLocker, clé Office *(données sensibles — exportées en clair)* |
| **Navigateurs** | Favoris Chrome, Edge, Brave |
| **Démarrage & Tâches** | Programmes démarrage, tâches planifiées, export registre Run |
| **Développeur** | Variables env., clés SSH, packages Python (pip), extensions VSCode, config WSL, profil PowerShell |
| **Matériel** | Rapport composants PC, plans d'alimentation, imprimantes |
| **Divers** | Tailles dossiers, fichiers Bureau, processus suspects |

**Formats d'export :** `.txt`, `.html`, `.md`, `.json`
**Emplacement :** `~/Documents/NiTriTe/backups/` (personnalisable)
**Presets :** Essentiel · Dev · Complet

> ⚠️ **Sécurité :** Les éléments marqués **Sensible** (🔒) exportent des données confidentielles en clair. Conservez le fichier de sauvegarde en lieu sûr.

---

### 🛡 Scan Antivirus

**Route :** `/scanvirus` — Interface Windows Defender.

NiTriTe s'interface directement avec **Windows Defender** via PowerShell :

| Action | Description |
|---|---|
| **Scan Rapide** | Analyse les zones vulnérables (~5 min) |
| **Scan Complet** | Analyse tous les fichiers (~1h+) |
| **Scan Hors-ligne** | Redémarre en mode sans échec pour scanner les rootkits |
| **Scan Personnalisé** | Analyse un dossier/lecteur spécifique |
| **Quarantaine** | Consulte les menaces mises en quarantaine par Defender |
| **Mise à jour définitions** | `Update-MpSignature` pour les signatures de virus |
| **Planification** | Crée une tâche planifiée pour un scan hebdomadaire automatique |

Affiche l'état de la **protection temps réel** et l'**historique des scans** (10 derniers).

---

### 📦 Dépendances

**Route :** `/dependencies` — Gestionnaire de runtimes.

Vérifie et installe les dépendances système courantes :
- **.NET Framework / .NET Runtime** (toutes versions)
- **Visual C++ Redistributable** (2005 → 2022)
- **DirectX**
- **WebView2 Runtime**
- **Java JRE**
- **PowerShell 7**

---

## Réseau & Terminal

### 📡 Réseau

**Route :** `/network` — Informations et configuration réseau.

- Interfaces réseau actives (IP, masque, passerelle, DNS, MAC)
- Test de connectivité (ping)
- Statistiques de connexion (débit, paquets)
- Commandes réseau rapides (ipconfig, netstat, tracert)
- Test de débit (upload/download)

---

### 🌐 DNS Switcher

**Route :** `/dns-switcher` — Changement de serveurs DNS en 1 clic.

Préréglages DNS populaires :

| Fournisseur | DNS Primaire | DNS Secondaire |
|---|---|---|
| Google | 8.8.8.8 | 8.8.4.4 |
| Cloudflare | 1.1.1.1 | 1.0.0.1 |
| OpenDNS | 208.67.222.222 | 208.67.220.220 |
| Quad9 | 9.9.9.9 | 149.112.112.112 |
| NextDNS | Configurable | — |

Applique le DNS sur l'interface réseau active. Restauration des DNS automatiques en 1 clic.

---

### 📶 WiFi Analyzer

**Route :** `/wifi-analyzer` — Analyse des réseaux WiFi.

- Liste tous les réseaux WiFi à portée
- Affiche canal, signal (dBm), sécurité, fréquence (2.4/5 GHz)
- Identifie les interférences de canaux
- Graphique de signal en temps réel
- Recommandations de canal optimal

---

### 🔌 Scanner de Ports

**Route :** `/port-scanner` — Scan de ports TCP/UDP.

- Scan d'une IP ou plage d'IPs
- Ports communs prédéfinis (HTTP, HTTPS, SSH, RDP, FTP, etc.)
- Scan personnalisé par plage de ports
- Identification du service associé à chaque port ouvert
- Mode scan automatique planifié

---

### 🔵 Bluetooth

**Route :** `/bluetooth` — Gestion Bluetooth.

- État de l'adaptateur Bluetooth
- Liste des périphériques associés (type, état de connexion, batterie si disponible)
- Activer/désactiver le Bluetooth
- Informations sur l'adaptateur (version, fabricant)

---

### ⬛ Terminal

**Route :** `/terminal` — Terminal intégré.

Terminal PowerShell/CMD directement dans l'interface de NiTriTe :
- Historique des commandes
- Auto-complétion
- Support des commandes système complètes
- Sortie colorée

---

### 📝 Scripts & Snippets

**Route :** `/scripts` — Gestionnaire de scripts.

Créez, organisez et exécutez des scripts directement depuis NiTriTe :

**Langages supportés :** PowerShell, CMD, Python, JavaScript, SQL, Bash, Autre

**Fonctionnalités :**
- Éditeur de code intégré avec coloration syntaxique
- Tags et description pour retrouver facilement un script
- Filtre par langage
- **Exécution directe** (PowerShell et CMD)
- Copie en 1 clic dans le presse-papiers
- Les scripts PowerShell/CMD sont exécutables depuis NiTriTe sans accès au terminal
- Sauvegarde locale persistante

---

## Intelligence

### 🤖 Agent IA

**Route :** `/ai-agents` — Assistant IA intégré.

Interface de chat avec un assistant IA pour l'aide au diagnostic système, l'interprétation des erreurs, et les recommandations de maintenance. Support de modèles locaux (LM Studio) et API externes.

---

### 📚 Base de Connaissances

**Route :** `/knowledge-base` — Documentation interne.

Base de données d'articles techniques sur Windows :
- Résolution d'erreurs courantes
- Guides de maintenance
- Codes d'erreur Windows (BSOD, Event Log)
- Bonnes pratiques système

---

## Configuration

### ⚙ Paramètres

**Route :** `/settings` — Configuration de l'application.

Personnalisation du comportement de NiTriTe :
- Langue de l'interface
- Intervalle de polling (monitoring)
- Notifications système
- Emplacement par défaut des sauvegardes
- Comportement au démarrage

---

### 👤 Profils

**Route :** `/profiles` — Gestion des profils utilisateur.

Sauvegardez et chargez des configurations NiTriTe pour différents contextes d'utilisation (Technicien, Administrateur, Utilisateur standard).

---

### 🎨 Éditeur de Thème

**Route :** `/theme-editor` — Personnalisation visuelle.

Personnalisez l'apparence complète de l'application :
- Thèmes prédéfinis (Dark, Light, Nitrite Orange, etc.)
- Éditeur de couleurs avancé (accent, arrière-plans, textes, bordures)
- Présets de disposition (sidebar position, densité)
- Prévisualisation en temps réel
- Export/import de thèmes

---

### 📜 Logs

**Route :** `/logs` — Journal d'activité de l'application.

- Historique de toutes les opérations effectuées dans NiTriTe
- Niveaux : INFO, WARNING, ERROR
- Filtres par date, niveau, catégorie
- Mode "follow" (affichage en temps réel)
- Export des logs

---

## 🔵 Mode WinPE

**Route :** `/winpe` — Environnement de réparation Windows.

Le Mode WinPE permet d'effectuer des réparations système avancées depuis un **environnement Windows Preinstallation**. Ce mode est conçu pour être utilisé depuis une clé USB bootable NiTriTe ou directement sur un système démarré.

### Onglet Réparation
Outils de réparation du démarrage et du système :

| Action | Commande |
|---|---|
| Réparer le MBR | `bootrec /fixmbr` |
| Réparer le secteur de démarrage | `bootrec /fixboot` |
| Reconstruire le BCD | `bootrec /rebuildbcd` |
| Scanner les OS | `bootrec /scanos` |
| Vérifier le disque | `chkdsk C: /f /r` |
| SFC hors ligne | `sfc /scannow /offbootdir=C:\ /offwindir=C:\Windows` |
| DISM hors ligne | Réparation de l'image Windows depuis WinPE |
| Réinitialisation MDP | `net user [compte] [mdp]` |
| Déverrouiller BitLocker | `manage-bde -unlock` avec clé de récupération |

### Onglet Réseau
- 15 commandes réseau rapides (ipconfig, netsh, ping, DNS, routes)
- Test ping vers une IP/domaine personnalisé
- Configuration IP statique ou DHCP
- Réinitialisation configuration réseau

### Onglet Disques
- Commandes Diskpart complètes (list disk/volume, clean, format, assign)
- Informations SMART via PowerShell
- Formatage de volume (NTFS/FAT32/exFAT)
- Clonage avec Robocopy
- Nettoyage disque (ATTENTION : irréversible)

### Onglet Système
- Liste des processus et services actifs
- Journaux d'événements Windows
- Analyse des dumps BSOD
- Tuer un processus par PID
- Démarrer/arrêter un service par nom
- Lancement d'outils graphiques (regedit, explorer, taskmgr, msconfig...)

### Onglet Registre
- Montage de ruches hors ligne (SAM, SYSTEM, SOFTWARE, DEFAULT)
- Export de clés de registre (`reg export`)
- Lecture/modification de valeurs individuelles
- 16 opérations registre courantes :
  - Désactiver UAC (hors ligne)
  - Activer RDP (hors ligne)
  - Corriger LSA/SecureBoot
  - Supprimer entrées de démarrage suspectes
  - Sauvegarder/restaurer des ruches

### Onglet Outils
- Téléchargement d'outils de récupération tiers (Hiren's BootCD PE, SystemRescue, GParted, etc.)
- Commandes rapides WinPE en 1 clic

---

## Raccourcis Clavier

| Raccourci | Action |
|---|---|
| `Ctrl+K` | Recherche rapide (accès à toutes les fonctionnalités) |
| `Ctrl+,` | Ouvrir les paramètres |
| `Escape` | Fermer le modal/panneau actif |
| `F5` | Actualiser la page en cours |

---

## Stack Technique

| Composant | Technologie | Version |
|---|---|---|
| **Framework Desktop** | Tauri | v2.x |
| **Backend** | Rust | stable |
| **Frontend** | Vue.js | 3.5 |
| **Typage** | TypeScript | 5.7 |
| **Build** | Vite | 6.x |
| **UI Components** | Lucide Icons + composants maison | — |
| **State** | Pinia | 3.x |
| **Routing** | Vue Router | 4.x |
| **Charts** | Chart.js + vue-chartjs | 4.x |
| **Recherche** | Fuse.js | 7.x |

---

## Licence & Crédits

NiTriTe 2.0 est un outil propriétaire destiné à un usage personnel et professionnel.
Développé avec ❤️ pour les techniciens informatiques et les utilisateurs avancés Windows.

---

*Documentation générée pour NiTriTe v6.0.0 — Mars 2026*