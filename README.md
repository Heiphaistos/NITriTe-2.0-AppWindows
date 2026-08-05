<div align="center">
  <h1>🔧 NiTriTe 8.144.0</h1>
  <p><strong>Suite de diagnostic, réparation, optimisation et administration Windows — 44 outils, interface Tauri v2 native.</strong></p>

  ![Version](https://img.shields.io/badge/version-8.144.0-blue)
  ![Stack](https://img.shields.io/badge/stack-Tauri%20v2%20%2B%20Rust%20%2B%20Vue%203-purple)
  ![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-informational)
  ![Language](https://img.shields.io/badge/language-Rust%20%2B%20TypeScript-orange)
  ![License](https://img.shields.io/badge/licence-MIT-green)
</div>

---

## 📋 Description

NiTriTe est un outil Windows tout-en-un conçu pour les techniciens et utilisateurs avancés. **44 pages** organisées en **9 catégories**, dont un tableau de diagnostic regroupant **33 sous-onglets** d'analyse système — le tout via une interface native moderne construite avec Tauri v2 (backend Rust, frontend Vue 3).

La 8.144.0 couvre le diagnostic matériel/logiciel complet, la réparation Windows (SFC/DISM/WinPE bootable), le clonage et la récupération de données (VSS), la gestion réseau et sécurité, l'automatisation via scripts et un assistant IA local (Ollama / llama.cpp portable), jusqu'au packaging d'une release portable autonome (exe + logiciels + drivers + scripts Windows).

---

## 📺 Démonstration

<video src="https://media.heiphaistos.org/videos/nitrite.mp4" controls width="100%" preload="none"></video>

---

## ✨ Fonctionnalités

### 🖥️ Système
- **Tableau de bord** : vue d'ensemble santé système, raccourcis, score global
- **Diagnostic** (33 sous-onglets, voir détail ci-dessous) : scan complet exportable TXT/HTML/MD/JSON
- **Optimisations** : debloat, réglages performance, tweaks Windows en un clic
- **Monitoring** : surveillance temps réel avec annotations et enregistrement de session

### 📦 Logiciels
- **Outils Système** : accès rapide utilitaires Windows natifs
- **Master Install** : installation en lot multi-source (winget/Chocolatey/Scoop) avec dry-run et résumé
- **Apps Portables** : bibliothèque d'applications portables catégorisées (bureautique, dev, média, réseau, système, utilitaires)
- **OS & USB Tools** : téléchargement ISO Windows, création clé USB bootable
- **Applications** : inventaire logiciels installés, désinstallation groupée

### ⚡ Performance
- **Températures** : sondes CPU/GPU/carte mère en temps réel
- **Benchmark** : test de performance CPU/RAM/disque avec historique de gains
- **Historique Performance** : suivi des métriques dans le temps, comparaison snapshots
- **Turbo Mode** : profil performance temporaire (stats avant/après)
- **Rapports statistiques** : seuils configurables, comparaison session précédente

### 🧪 Avancé (BETA)
- **Clonage Système** : `wbadmin` + `robocopy` avec gestion des codes de retour
- **Récupération de Données** : VSS (Shadow Copy), Corbeille, fichiers supprimés, dossiers utilisateur
- **Visualiseur Disque** : cartographie de l'occupation espace par dossier/type
- **Doublons** : détection et suppression de fichiers dupliqués (hash)
- **Gros Fichiers** : recherche des plus gros consommateurs d'espace
- **Hash Checker** : calcul/vérification SHA-256 de fichiers
- **Boot Manager** : gestion BCD, entrée de démarrage par défaut, timeout
- **Éditeur Hosts** : édition sécurisée du fichier hosts (validation IP/hostname)
- **Analyse BSOD** : parsing dump crash Windows, diagnostic cause probable
- **WSL Linux** : gestion des distributions Windows Subsystem for Linux
- **Points de Restauration** : création/restauration de points système
- **Docker Manager** : gestion conteneurs/images Docker Desktop

### 🔧 Maintenance
- **Mises à jour** : Windows Update + 4 gestionnaires de paquets (WinGet, Chocolatey, Scoop, Windows Update) avec détail des mises à jour disponibles
- **Drivers** : inventaire pilotes, détection critiques/problématiques (Error/Degraded), scanner de mise à jour dédié
- **Désinstallateur** : détection automatique NSIS (`/S`), Inno Setup (`/VERYSILENT`), winget — désinstall silencieux
- **Nettoyeur Avancé** : nettoyage fichiers temporaires, cache navigateurs, registre
- **Sauvegarde** : backup ciblé avec collecteurs et formatage de rapport
- **Scan Antivirus** : lancement scan Windows Defender / rapport menaces
- **Dépendances** : détection runtimes requis (VC++, .NET...), filtre requises/optionnelles, test post-installation

### 🌐 Réseau & Terminal
- **Réseau** : configuration interfaces, statistiques, Wi-Fi
- **DNS Switcher** : changement rapide de serveurs DNS
- **WiFi Analyzer** : scan réseaux à proximité, qualité signal
- **Scanner de Ports** : détection ports ouverts et services exposés
- **Bluetooth** : gestion périphériques appairés
- **Terminal** : terminal intégré (PowerShell/CMD)
- **Scripts & Snippets** : éditeur, bibliothèque de scripts (.bat/.cmd/.ps1) exécutables depuis l'app, moteur de validation

### 🧠 Intelligence
- **Agent IA** : assistant local via Ollama ou llama.cpp portable, appel d'outils (tool calling) sur les commandes Nitrite
- **Base de Connaissances** : articles et procédures de dépannage
- **Documentation** : aide intégrée

### 📊 Rapports
- **Logs** : consultation logs application (rotation, niveaux)
- **Éditeur de Thème** : personnalisation de l'interface

### ⚙️ Configuration
- **Paramètres** : préférences application, export/import config
- **Profils** : profils de configuration multiples

### 💽 WinPE
- **Mode WinPE** : ISO bootable Windows PE 11 pour réparation hors-OS (build via Windows ADK), 15+ commandes PE

---

### 🩺 Diagnostic — détail des 33 sous-onglets

| Bloc | Onglets |
|------|---------|
| Matériel | CPU, RAM, GPU, Stockage (Storage), Capteurs/Températures (Perf) |
| Système | Système (info générale), Historique (événements), Comptes (Accounts), Activation (licence Windows), Certificats numériques |
| Logiciel | Software (inventaire), Services, SysDrivers, Mises à jour (Updates), Cleaner |
| Réseau | Network, Firewall, Partages (Shares), Hosts, NetTools, Bluetooth |
| Sécurité | Security (registre clés de persistance suspectes) |
| Stockage avancé | Dossiers (Folders), Processus (Processes) |
| Réparation | Repair (SFC `/scannow`, DISM `RestoreHealth`), Boot, Analyse BSOD |
| Performance | Benchmark, Historique Perf. (PerfHistory) |
| Pilotes | Driver Updater |
| Autres | WSL, Scan (orchestrateur du scan complet + export) |

Export des résultats en **TXT / HTML / MD / JSON**, score de santé global, choix du périmètre de scan (rapide/complet).

---

## 🛠️ Stack technique

| Couche | Technologies |
|--------|-------------|
| Frontend | Vue 3 + TypeScript + Vite + Pinia + Tailwind CSS |
| Backend natif | Rust + Windows API (`std::process::Command`, `windows-rs`) |
| Framework desktop | Tauri v2 |
| IPC | Tauri commands (`invoke`), déduplication concurrence |
| IA | Ollama / llama.cpp portable, tool calling sur commandes natives |
| Installer | NSIS (bundle Tauri) + SFX 7-Zip (release portable tout-en-un) |
| Build | `build.bat` (kill → tsc → tauri build → packaging 4 modes) |

---

## 🚀 Installation

### Prérequis

- Windows 10 / 11 (x64)
- Rust stable (`rustup`) + Node.js 18+
- WebView2 Runtime (inclus dans Windows 11, auto-installé sinon)

### Installer (utilisateur final)

Télécharger et exécuter le setup NSIS :

```
Nitrite_8.144.0_x64-setup.exe
```

Ou la release portable tout-en-un (`Nitrite_v8.144.0_full.exe`) : app + logiciels portables + drivers + scripts Windows, sans installation.

### Build depuis les sources

```bat
REM Prérequis : npm install (première fois)
npm install

REM Build interactif — 4 modes de packaging au choix
build.bat
```

`build.bat` propose :
1. **EXE portable seul** (~15 Mo) — application uniquement
2. **Dossier portable complet** (~2,5 Go) — exe + `logiciel/` + `Drivers/` + `Script Windows/`
3. **SFX tout-en-un** — un seul `.exe` qui extrait tout et lance Nitrite
4. **ISO WinPE 11 bootable** — nécessite Windows ADK + WinPE Add-on

L'exécutable généré : `src-tauri\target\release\nitrite.exe`
L'installeur NSIS : `src-tauri\target\release\bundle\nsis\Nitrite_8.144.0_x64-setup.exe`

### Développement

```bat
npm run tauri dev
```

---

## 📂 Architecture

```
NiTriTe/
├── src/
│   ├── components/diagnostic/     # 33 composants Vue (DiagTab*.vue)
│   ├── data/
│   │   ├── navigation.ts          # 9 catégories, structure du menu
│   │   └── portable/              # catalogue apps portables par catégorie
│   ├── pages/                     # 44 pages (une par route)
│   └── router/                    # routes Vue Router
├── src-tauri/
│   ├── src/
│   │   ├── system/                 # ~50 modules (clone, boot_manager, drivers, security...)
│   │   ├── installer/              # winget, chocolatey, scoop, uninstaller, smart_install
│   │   ├── maintenance/            # cleanup, debloat, browser_cleanup, terminal
│   │   ├── backup/collector/       # collecteurs de sauvegarde + rendu rapport
│   │   ├── scripts/                # executor + validator (scripts .bat/.ps1)
│   │   └── ai/                     # Ollama / llama.cpp portable, tool calling
│   └── target/release/bundle/nsis/ # Installeur NSIS final
├── logiciel/                       # apps portables bundlées release complète (gitignored, ~2,5 Go)
├── Drivers/                        # runtimes (Visual C++) bundlés release complète
├── Script Windows/                 # scripts .bat/.ps1 accessibles depuis l'app
├── boot/                           # build-bootable.bat — génération ISO WinPE
├── build.bat                       # Script de build Windows (4 modes)
└── package.json
```

---

## 📝 Notes techniques

- **CREATE_NO_WINDOW** (`0x08000000`) appliqué sur tous les modules Rust — pas de flash CMD
- Robocopy : codes de retour `< 8` = succès (comportement normal)
- VSS paths : format `\\?\GLOBALROOT` + device_object
- Désinstall NSIS détecté via metadata `VersionInfo` (`Nullsoft`) → `/S`
- Désinstall Inno Setup détecté via `Inno|Jordan Russell` → `/VERYSILENT`
- `set_default_boot` : validation stricte GUID avant interpolation `bcdedit`
- Version applicative injectée au build depuis `package.json` (`__APP_VERSION__`, `vite.config.ts`)

---

## 📝 Licence

MIT — © 2026 Heiphaistos
