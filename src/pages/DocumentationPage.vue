<script setup lang="ts">
import { ref, computed, type Component } from "vue";
import NCard from "@/components/ui/NCard.vue";
import NSearchBar from "@/components/ui/NSearchBar.vue";
import {
  BookOpen, ChevronDown, ChevronRight,
  Rocket, Stethoscope, Download, Wrench,
  Save, Bot, Keyboard, Info,
  Monitor, Globe, Shield, HardDrive, Terminal,
  BarChart3, Lightbulb, AlertTriangle, Zap,
} from "lucide-vue-next";

interface DocSection {
  id: string;
  title: string;
  icon: Component;
  content: { type: "text" | "list" | "kbd" | "warning" | "tip"; value: string | string[] }[];
}

const search = ref("");
const appVersion = __APP_VERSION__;

const sections: DocSection[] = [
  {
    id: "premiers-pas",
    title: "Premiers pas",
    icon: Rocket,
    content: [
      { type: "text", value: "Bienvenue dans NiTriTe, votre outil de maintenance systeme tout-en-un." },
      { type: "text", value: "Au lancement, le Tableau de bord affiche un apercu de l'etat de votre systeme : CPU, RAM, disque et reseau." },
      { type: "list", value: [
        "Utilisez le menu lateral pour naviguer entre les differentes sections",
        "Cliquez sur le nom d'une categorie pour replier/deplier le groupe",
        "Le bouton en haut de la sidebar permet de la replier completement",
        "Utilisez Ctrl+K pour ouvrir la recherche rapide de pages",
      ] },
      { type: "tip", value: "Commencez par un Diagnostic complet pour obtenir un etat des lieux detaille de votre materiel." },
    ],
  },
  {
    id: "dashboard",
    title: "Tableau de bord",
    icon: BarChart3,
    content: [
      { type: "text", value: "Le Tableau de bord est le point d'entree de l'application. Il affiche les metriques systeme essentielles en temps reel." },
      { type: "list", value: [
        "Score de sante global (0-100) calcule a partir du CPU, RAM et disque",
        "Graphiques en temps reel de l'utilisation des ressources",
        "Alertes automatiques si un seuil critique est depasse",
        "Acces rapide aux actions les plus courantes (diagnostic, nettoyage, terminal)",
      ] },
      { type: "text", value: "Un score superieur a 80 indique un systeme en bonne sante. En dessous de 50, des actions correctives sont recommandees." },
    ],
  },
  {
    id: "diagnostic",
    title: "Diagnostic systeme",
    icon: Stethoscope,
    content: [
      { type: "text", value: "La page Diagnostic analyse en profondeur votre materiel via WMI (Windows Management Instrumentation)." },
      { type: "list", value: [
        "Processeur : modele, frequence, nombre de coeurs/threads, architecture",
        "Memoire RAM : capacite totale, type (DDR4/DDR5), frequence, slots",
        "Stockage : disques, partitions, espace libre, type (SSD/HDD)",
        "GPU : carte graphique, VRAM, pilote installe",
        "Carte mere : fabricant, modele, version BIOS",
        "Reseau : adaptateurs, IP, MAC, debit",
      ] },
      { type: "tip", value: "Exportez les resultats au format JSON pour archivage ou partage avec un technicien." },
    ],
  },
  {
    id: "monitoring",
    title: "Monitoring temps reel",
    icon: Monitor,
    content: [
      { type: "text", value: "Le Monitoring affiche les metriques systeme en temps reel avec un rafraichissement configurable." },
      { type: "list", value: [
        "Utilisation CPU et RAM en pourcentage avec graphique",
        "Top processus consommateurs tries par usage CPU/RAM",
        "Debit reseau entrant/sortant en temps reel",
        "Espace disque utilise sur chaque partition",
      ] },
      { type: "text", value: "L'intervalle de rafraichissement peut etre ajuste dans les Parametres (500ms a 5000ms)." },
      { type: "warning", value: "Un intervalle trop court (< 500ms) peut impacter les performances sur les machines anciennes." },
    ],
  },
  {
    id: "reseau",
    title: "Reseau",
    icon: Globe,
    content: [
      { type: "text", value: "La page Reseau fournit une vue complete de votre configuration reseau." },
      { type: "list", value: [
        "Vue d'ensemble : nom d'hote, domaine, adaptateurs actifs",
        "Adresses IP (IPv4/IPv6), masque, passerelle",
        "Connexions actives : protocole, port local/distant, etat",
        "Test de ping vers un hote avec mesure de latence",
      ] },
    ],
  },
  {
    id: "installation",
    title: "Installation d'applications",
    icon: Download,
    content: [
      { type: "text", value: "NiTriTe propose plus de 700 applications organisees en 29 categories, installables via WinGet." },
      { type: "list", value: [
        "Recherchez par nom ou filtrez par categorie (Navigateurs, Bureautique, Dev...)",
        "Cliquez sur 'Installer' pour lancer l'installation silencieuse",
        "La progression est affichee en temps reel dans les logs",
        "Master Install : selectionnez plusieurs apps et installez-les en un clic",
      ] },
      { type: "warning", value: "WinGet doit etre installe sur le systeme. NiTriTe verifie automatiquement sa presence." },
      { type: "tip", value: "Utilisez Master Install pour configurer rapidement un nouveau PC de travail." },
    ],
  },
  {
    id: "portables",
    title: "Applications portables",
    icon: HardDrive,
    content: [
      { type: "text", value: "Les applications portables fonctionnent sans installation Windows." },
      { type: "list", value: [
        "Telechargez l'application depuis le lien fourni",
        "Extrayez les fichiers dans le dossier logiciel/<nom_app>/ a cote de NiTriTe",
        "Lancez l'application directement depuis la page Portables",
        "Ideal pour les cles USB de depannage",
      ] },
      { type: "text", value: "18 applications portables sont referencees : outils systeme (Sysinternals, CPU-Z, GPU-Z...), reseau (PuTTY, WinSCP...), multimedia (VLC) et utilitaires." },
    ],
  },
  {
    id: "maintenance",
    title: "Maintenance & Optimisation",
    icon: Wrench,
    content: [
      { type: "text", value: "Les outils de maintenance permettent de garder votre systeme propre et performant." },
      { type: "list", value: [
        "Nettoyage : fichiers temporaires, corbeille, cache navigateur",
        "Drivers : liste complete avec export CSV, detection des pilotes obsoletes",
        "Mises a jour : verification via WinGet des applications a mettre a jour",
        "Antivirus : lancement de scans Windows Defender (rapide ou complet)",
        "Outils Windows : plus de 500 outils systeme accessibles en un clic",
      ] },
    ],
  },
  {
    id: "sauvegarde",
    title: "Sauvegarde systeme",
    icon: Save,
    content: [
      { type: "text", value: "La page Sauvegarde permet de creer des copies de securite de la configuration systeme." },
      { type: "list", value: [
        "Applications installees (liste complete)",
        "Configuration des pilotes",
        "Parametres reseau (IP, DNS, passerelle)",
        "Programmes au demarrage",
        "Variables d'environnement",
        "Regles de pare-feu",
        "Favoris navigateur (Chrome, Edge, Brave)",
        "Cle de licence Windows",
      ] },
      { type: "text", value: "Les sauvegardes sont stockees dans le dossier backups/ au format JSON avec horodatage." },
    ],
  },
  {
    id: "terminal",
    title: "Terminal & Scripts",
    icon: Terminal,
    content: [
      { type: "text", value: "Le Terminal integre permet d'executer des commandes systeme sans ouvrir cmd.exe." },
      { type: "list", value: [
        "Historique de commandes navigable avec les fleches haut/bas",
        "Affichage stdout et stderr avec coloration",
        "133 scripts pre-integres organises en 11 categories",
      ] },
      { type: "text", value: "Categories de scripts : Diagnostic, Drivers, Nettoyage, Performance, Registre, Reparation, Reseau, Sauvegardes, Securite, Tweaks, Utilisateurs." },
      { type: "warning", value: "Certains scripts necessitent des droits administrateur pour fonctionner correctement." },
    ],
  },
  {
    id: "ia",
    title: "Agent IA (Ollama)",
    icon: Bot,
    content: [
      { type: "text", value: "L'Agent IA est un assistant conversationnel base sur Ollama qui diagnostique et resout les problemes." },
      { type: "list", value: [
        "Decrivez votre probleme en langage naturel",
        "L'IA propose des solutions adaptees a votre contexte systeme",
        "Execution de commandes systeme securisees avec confirmation",
        "Necessite Ollama installe localement (http://localhost:11434)",
      ] },
      { type: "tip", value: "Configurez l'URL et le modele Ollama dans les Parametres." },
    ],
  },
  {
    id: "securite",
    title: "Securite",
    icon: Shield,
    content: [
      { type: "text", value: "NiTriTe integre des outils de securite pour proteger votre systeme." },
      { type: "list", value: [
        "Scan antivirus via Windows Defender (rapide ou complet)",
        "Verification du statut de la protection en temps reel",
        "Base de connaissances avec guides de securite",
        "Scripts de diagnostic securite (audit pare-feu, ports ouverts...)",
      ] },
    ],
  },
  {
    id: "ia-portable",
    title: "IA locale portable (llama.cpp)",
    icon: Bot,
    content: [
      { type: "text", value: "L'assistant IA de NiTriTe fonctionne en mode 100% portable : aucune installation sur le PC cible. Tout tourne depuis le dossier de l'application." },
      { type: "list", value: [
        "Étape 1 — Téléchargez le moteur IA (llama-server.exe) depuis l'onglet Assistant IA",
        "Étape 2 — Téléchargez un modèle GGUF depuis le catalogue (18 modèles disponibles)",
        "Étape 3 — Démarrez le serveur et posez vos questions",
        "Le moteur et les modèles sont stockés dans logiciel/AI/ et models/",
        "Aucune trace laissée sur le système cible",
      ] },
      { type: "tip", value: "Modèle recommandé : Phi-3 Mini 3.8B (2.2 GB, excellent rapport qualité/vitesse)." },
      { type: "warning", value: "Le chargement du modèle peut prendre 30-60 secondes selon votre disque et la taille du modèle." },
    ],
  },
  {
    id: "raccourcis",
    title: "Raccourcis clavier",
    icon: Keyboard,
    content: [
      { type: "kbd", value: [
        "Ctrl+K|Recherche rapide de pages",
        "Ctrl+B|Replier/deployer la sidebar",
        "Echap|Fermer la fenetre modale active",
        "↑ ↓|Naviguer dans la recherche/historique terminal",
        "Entree|Valider la selection/commande",
      ] },
    ],
  },
];

const expanded = ref<Set<string>>(new Set(["premiers-pas"]));

function toggle(id: string) {
  if (expanded.value.has(id)) {
    expanded.value.delete(id);
  } else {
    expanded.value.add(id);
  }
}

function expandAll() {
  sections.forEach((s) => expanded.value.add(s.id));
}

function collapseAll() {
  expanded.value.clear();
}

const filteredSections = computed(() => {
  if (!search.value) return sections;
  const q = search.value.toLowerCase();
  return sections.filter((s) => {
    if (s.title.toLowerCase().includes(q)) return true;
    return s.content.some((c) => {
      if (typeof c.value === "string") return c.value.toLowerCase().includes(q);
      return c.value.some((v) => v.toLowerCase().includes(q));
    });
  });
});
</script>

<template>
  <div class="docs-page">
    <div class="page-header">
      <div>
        <h1><BookOpen :size="22" /> Documentation</h1>
        <p class="page-subtitle">Guide complet d'utilisation de NiTriTe — {{ sections.length }} sections</p>
      </div>
      <div class="header-actions">
        <button class="action-btn" @click="expandAll">Tout deployer</button>
        <button class="action-btn" @click="collapseAll">Tout replier</button>
      </div>
    </div>

    <NSearchBar v-model="search" placeholder="Rechercher dans la documentation..." />

    <div class="accordion-list">
      <NCard v-for="section in filteredSections" :key="section.id" padding="none">
        <button class="accordion-header" @click="toggle(section.id)">
          <div class="accordion-left">
            <component :is="section.icon" :size="16" style="color: var(--accent-primary)" />
            <span>{{ section.title }}</span>
          </div>
          <ChevronDown v-if="expanded.has(section.id)" :size="16" />
          <ChevronRight v-else :size="16" />
        </button>
        <div v-if="expanded.has(section.id)" class="accordion-body">
          <template v-for="(block, i) in section.content" :key="i">
            <p v-if="block.type === 'text'" class="doc-text">{{ block.value }}</p>

            <ul v-else-if="block.type === 'list'" class="doc-list">
              <li v-for="(item, j) in (block.value as string[])" :key="j">{{ item }}</li>
            </ul>

            <div v-else-if="block.type === 'kbd'" class="kbd-grid">
              <div v-for="(item, j) in (block.value as string[])" :key="j" class="kbd-row">
                <kbd>{{ item.split('|')[0] }}</kbd>
                <span>{{ item.split('|')[1] }}</span>
              </div>
            </div>

            <div v-else-if="block.type === 'warning'" class="doc-warning">
              <AlertTriangle :size="14" />
              <span>{{ block.value }}</span>
            </div>

            <div v-else-if="block.type === 'tip'" class="doc-tip">
              <Lightbulb :size="14" />
              <span>{{ block.value }}</span>
            </div>
          </template>
        </div>
      </NCard>
    </div>

    <div v-if="filteredSections.length === 0" class="empty-state">
      Aucune section ne correspond a "{{ search }}"
    </div>

    <!-- Quick tips -->
    <NCard>
      <template #header>
        <div class="section-header"><Zap :size="16" /><span>Astuces rapides</span></div>
      </template>
      <div class="tips-list">
        <div class="tip-item">Lancez un diagnostic complet apres chaque mise a jour majeure de Windows.</div>
        <div class="tip-item">Utilisez Master Install pour configurer rapidement un nouveau PC.</div>
        <div class="tip-item">Exportez regulierement un rapport systeme pour suivre l'evolution de votre materiel.</div>
        <div class="tip-item">Le Terminal integre evite de basculer vers cmd.exe ou PowerShell.</div>
        <div class="tip-item">Programmez vos sauvegardes config avant toute intervention lourde (reinstallation, changement materiel).</div>
        <div class="tip-item">Utilisez les applications portables sur une cle USB pour depanner des PC sans internet.</div>
      </div>
    </NCard>

    <!-- About -->
    <NCard>
      <template #header>
        <div class="section-header"><Info :size="16" /><span>A propos de NiTriTe</span></div>
      </template>
      <div class="about-info">
        <div class="about-row"><span>Application</span><span>NiTriTe</span></div>
        <div class="about-row"><span>Version</span><span>{{ appVersion }}</span></div>
        <div class="about-row"><span>Stack</span><span>Tauri v2 + Rust + Vue 3 + TypeScript</span></div>
        <div class="about-row"><span>IA</span><span>llama.cpp portable + Ollama</span></div>
        <div class="about-row"><span>Onglets diagnostic</span><span>33 onglets, 7 catégories</span></div>
        <div class="about-row"><span>Auteur</span><span>Momo</span></div>
        <div class="about-row"><span>Licence</span><span>Usage interne</span></div>
      </div>
    </NCard>
  </div>
</template>

<style scoped>
.docs-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 12px;
}
.page-header h1 {
  font-size: 22px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 10px;
}
.page-subtitle { color: var(--text-muted); font-size: 13px; margin-top: 2px; }
.header-actions { display: flex; gap: 8px; }
.action-btn {
  padding: 6px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  color: var(--text-muted);
  font-family: inherit;
  font-size: 12px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.action-btn:hover { background: var(--bg-tertiary); color: var(--text-primary); }

.section-header { display: flex; align-items: center; gap: 8px; }

.accordion-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.accordion-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 14px 16px;
  border: none;
  background: none;
  cursor: pointer;
  font-family: inherit;
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  transition: background var(--transition-fast);
}
.accordion-header:hover { background: var(--bg-tertiary); }
.accordion-left { display: flex; align-items: center; gap: 10px; }

.accordion-body {
  padding: 0 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  border-top: 1px solid var(--border);
  padding-top: 12px;
  margin: 0 16px 16px;
}

.doc-text {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.7;
}

.doc-list {
  margin: 0 0 0 20px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.doc-list li {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.6;
}

.kbd-grid {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.kbd-row {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13px;
  color: var(--text-secondary);
}
.kbd-row kbd {
  padding: 3px 8px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--bg-tertiary);
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
  color: var(--accent-primary);
  min-width: 80px;
  text-align: center;
}

.doc-warning {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 14px;
  background: rgba(234, 179, 8, 0.08);
  border: 1px solid rgba(234, 179, 8, 0.2);
  border-radius: var(--radius-md);
  font-size: 13px;
  color: var(--warning);
}
.doc-tip {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 14px;
  background: var(--accent-muted);
  border: 1px solid rgba(var(--accent-primary-rgb, 249, 115, 22), 0.2);
  border-radius: var(--radius-md);
  font-size: 13px;
  color: var(--accent-primary);
}

.tips-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.tip-item {
  padding: 8px 12px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-md);
  font-size: 13px;
  color: var(--text-secondary);
  border-left: 3px solid var(--accent-primary);
}

.about-info {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.about-row {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}
.about-row span:first-child { color: var(--text-muted); }
.about-row span:last-child { color: var(--text-primary); font-weight: 500; }

.empty-state {
  text-align: center;
  padding: 40px;
  color: var(--text-muted);
  font-size: 14px;
}
</style>
