import { defineStore } from "pinia";
import { ref, computed } from "vue";

export interface CustomSavedTheme {
  id: string;
  name: string;
  vars: Record<string, string>;
  createdAt: number;
}

// Variables exposées dans l'éditeur avec leurs valeurs par défaut (nitrite-dark)
export const THEME_VAR_GROUPS = [
  {
    label: "Arrière-plans",
    icon: "layers",
    vars: [
      { key: "--bg-primary",   label: "Fond principal",    type: "color" },
      { key: "--bg-secondary", label: "Fond secondaire",   type: "color" },
      { key: "--bg-tertiary",  label: "Fond tertiaire",    type: "color" },
      { key: "--bg-elevated",  label: "Fond surélevé",     type: "color" },
    ],
  },
  {
    label: "Couleur Accent",
    icon: "zap",
    vars: [
      { key: "--accent-primary", label: "Accent principal", type: "color" },
      { key: "--accent-hover",   label: "Accent survol",    type: "color" },
    ],
  },
  {
    label: "Texte",
    icon: "type",
    vars: [
      { key: "--text-primary",   label: "Texte principal",   type: "color" },
      { key: "--text-secondary", label: "Texte secondaire",  type: "color" },
      { key: "--text-muted",     label: "Texte atténué",     type: "color" },
    ],
  },
  {
    label: "Bordures",
    icon: "square",
    vars: [
      { key: "--border",       label: "Bordure",         type: "color" },
      { key: "--border-hover", label: "Bordure survol",  type: "color" },
    ],
  },
  {
    label: "Statuts",
    icon: "activity",
    vars: [
      { key: "--success", label: "Succès", type: "color" },
      { key: "--warning", label: "Avertissement", type: "color" },
      { key: "--danger",  label: "Danger",  type: "color" },
      { key: "--info",    label: "Info",    type: "color" },
    ],
  },
  {
    label: "Rayons",
    icon: "rounded-corner",
    vars: [
      { key: "--radius-sm", label: "Rayon SM", type: "radius", min: 0, max: 20 },
      { key: "--radius-md", label: "Rayon MD", type: "radius", min: 0, max: 24 },
      { key: "--radius-lg", label: "Rayon LG", type: "radius", min: 0, max: 32 },
      { key: "--radius-xl", label: "Rayon XL", type: "radius", min: 0, max: 40 },
    ],
  },
] as const;

export const PRESET_THEMES: { id: string; label: string; accent: string; vars: Record<string, string> }[] = [
  {
    id: "nitrite-dark", label: "Nitrite Dark", accent: "#f97316",
    vars: { "--bg-primary": "#09090b", "--bg-secondary": "#111114", "--bg-tertiary": "#1c1c20", "--bg-elevated": "#2a2a30", "--accent-primary": "#f97316", "--accent-hover": "#fb923c", "--text-primary": "#fafafa", "--text-secondary": "#a1a1aa", "--text-muted": "#71717a", "--border": "#2e2e33", "--border-hover": "#44444c", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "cyber-blue", label: "Cyber Blue", accent: "#3b82f6",
    vars: { "--bg-primary": "#0a0a1a", "--bg-secondary": "#111128", "--bg-tertiary": "#1a1a3e", "--bg-elevated": "#2a2a5e", "--accent-primary": "#3b82f6", "--accent-hover": "#60a5fa", "--text-primary": "#f0f4ff", "--text-secondary": "#94a3b8", "--text-muted": "#64748b", "--border": "#1e2040", "--border-hover": "#2a2a5e", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#38bdf8", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "matrix-green", label: "Matrix Green", accent: "#22c55e",
    vars: { "--bg-primary": "#030a03", "--bg-secondary": "#071207", "--bg-tertiary": "#0f1f0f", "--bg-elevated": "#1a331a", "--accent-primary": "#22c55e", "--accent-hover": "#4ade80", "--text-primary": "#e8ffe8", "--text-secondary": "#86efac", "--text-muted": "#4ade80", "--border": "#0f1f0f", "--border-hover": "#1a331a", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "4px", "--radius-md": "6px", "--radius-lg": "8px", "--radius-xl": "10px" },
  },
  {
    id: "purple-haze", label: "Purple Haze", accent: "#a855f7",
    vars: { "--bg-primary": "#0a0510", "--bg-secondary": "#150d20", "--bg-tertiary": "#201530", "--bg-elevated": "#302248", "--accent-primary": "#a855f7", "--accent-hover": "#c084fc", "--text-primary": "#faf5ff", "--text-secondary": "#c4b5fd", "--text-muted": "#7c3aed", "--border": "#201530", "--border-hover": "#302248", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "10px", "--radius-lg": "14px", "--radius-xl": "20px" },
  },
  {
    id: "red-alert", label: "Red Alert", accent: "#ef4444",
    vars: { "--bg-primary": "#0a0505", "--bg-secondary": "#1a0d0d", "--bg-tertiary": "#2a1515", "--bg-elevated": "#3d2020", "--accent-primary": "#ef4444", "--accent-hover": "#f87171", "--text-primary": "#fff5f5", "--text-secondary": "#fca5a5", "--text-muted": "#dc2626", "--border": "#2a1515", "--border-hover": "#3d2020", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "arctic-light", label: "Arctic Light", accent: "#0ea5e9",
    vars: { "--bg-primary": "#f8fafc", "--bg-secondary": "#f1f5f9", "--bg-tertiary": "#e2e8f0", "--bg-elevated": "#cbd5e1", "--accent-primary": "#0ea5e9", "--accent-hover": "#38bdf8", "--text-primary": "#0f172a", "--text-secondary": "#475569", "--text-muted": "#94a3b8", "--border": "#e2e8f0", "--border-hover": "#cbd5e1", "--success": "#16a34a", "--warning": "#ca8a04", "--danger": "#dc2626", "--info": "#0284c7", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "midnight-gold", label: "Midnight Gold", accent: "#eab308",
    vars: { "--bg-primary": "#0a0a05", "--bg-secondary": "#15140d", "--bg-tertiary": "#201e15", "--bg-elevated": "#302d20", "--accent-primary": "#eab308", "--accent-hover": "#facc15", "--text-primary": "#fffbeb", "--text-secondary": "#fde68a", "--text-muted": "#b45309", "--border": "#201e15", "--border-hover": "#302d20", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "neon-synthwave", label: "Neon Synthwave", accent: "#f0abfc",
    vars: { "--bg-primary": "#0d0015", "--bg-secondary": "#160025", "--bg-tertiary": "#22003a", "--bg-elevated": "#350058", "--accent-primary": "#f0abfc", "--accent-hover": "#f5c6fe", "--text-primary": "#fdf4ff", "--text-secondary": "#e879f9", "--text-muted": "#c026d3", "--border": "#22003a", "--border-hover": "#350058", "--success": "#4ade80", "--warning": "#fbbf24", "--danger": "#f43f5e", "--info": "#818cf8", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "ocean-deep", label: "Ocean Deep", accent: "#06b6d4",
    vars: { "--bg-primary": "#020f1a", "--bg-secondary": "#041a2e", "--bg-tertiary": "#062840", "--bg-elevated": "#0a3d5e", "--accent-primary": "#06b6d4", "--accent-hover": "#22d3ee", "--text-primary": "#ecfeff", "--text-secondary": "#67e8f9", "--text-muted": "#0e7490", "--border": "#062840", "--border-hover": "#0a3d5e", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#38bdf8", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "rose-quartz", label: "Rose Quartz", accent: "#f43f5e",
    vars: { "--bg-primary": "#1a0a10", "--bg-secondary": "#28101a", "--bg-tertiary": "#381525", "--bg-elevated": "#4d1d33", "--accent-primary": "#f43f5e", "--accent-hover": "#fb7185", "--text-primary": "#fff1f2", "--text-secondary": "#fda4af", "--text-muted": "#be123c", "--border": "#381525", "--border-hover": "#4d1d33", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "10px", "--radius-lg": "16px", "--radius-xl": "22px" },
  },
  {
    id: "void-dark", label: "Void Dark (AMOLED)", accent: "#6366f1",
    vars: { "--bg-primary": "#000000", "--bg-secondary": "#080808", "--bg-tertiary": "#111111", "--bg-elevated": "#1c1c1c", "--accent-primary": "#6366f1", "--accent-hover": "#818cf8", "--text-primary": "#ffffff", "--text-secondary": "#a8a8b8", "--text-muted": "#666680", "--border": "#1a1a1a", "--border-hover": "#2a2a2a", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "4px", "--radius-md": "6px", "--radius-lg": "8px", "--radius-xl": "12px" },
  },
  {
    id: "forest-green", label: "Forest Green", accent: "#16a34a",
    vars: { "--bg-primary": "#040f06", "--bg-secondary": "#081a0c", "--bg-tertiary": "#0f2614", "--bg-elevated": "#183a1f", "--accent-primary": "#16a34a", "--accent-hover": "#22c55e", "--text-primary": "#f0fdf4", "--text-secondary": "#86efac", "--text-muted": "#15803d", "--border": "#0f2614", "--border-hover": "#183a1f", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "copper-rust", label: "Copper Rust", accent: "#d97706",
    vars: { "--bg-primary": "#120a05", "--bg-secondary": "#1e1008", "--bg-tertiary": "#2c180d", "--bg-elevated": "#3d2314", "--accent-primary": "#d97706", "--accent-hover": "#f59e0b", "--text-primary": "#fffbeb", "--text-secondary": "#fcd34d", "--text-muted": "#92400e", "--border": "#2c180d", "--border-hover": "#3d2314", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "slate-steel", label: "Slate Steel", accent: "#64748b",
    vars: { "--bg-primary": "#0a0e18", "--bg-secondary": "#111622", "--bg-tertiary": "#1a2030", "--bg-elevated": "#252e42", "--accent-primary": "#64748b", "--accent-hover": "#94a3b8", "--text-primary": "#f1f5f9", "--text-secondary": "#94a3b8", "--text-muted": "#475569", "--border": "#1a2030", "--border-hover": "#252e42", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "4px", "--radius-md": "6px", "--radius-lg": "10px", "--radius-xl": "14px" },
  },
  // 10 nouveaux thèmes
  {
    id: "inferno", label: "Inferno", accent: "#ff4500",
    vars: { "--bg-primary": "#0c0200", "--bg-secondary": "#180500", "--bg-tertiary": "#270800", "--bg-elevated": "#3d1000", "--accent-primary": "#ff4500", "--accent-hover": "#ff6a30", "--text-primary": "#fff5ee", "--text-secondary": "#fca472", "--text-muted": "#c2410c", "--border": "#270800", "--border-hover": "#3d1000", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "aurora", label: "Aurora Borealis", accent: "#00d4aa",
    vars: { "--bg-primary": "#020d18", "--bg-secondary": "#051825", "--bg-tertiary": "#0a2535", "--bg-elevated": "#0f3548", "--accent-primary": "#00d4aa", "--accent-hover": "#00f0c0", "--text-primary": "#edfff9", "--text-secondary": "#67e8c9", "--text-muted": "#0e7a60", "--border": "#0a2535", "--border-hover": "#0f3548", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "10px", "--radius-lg": "14px", "--radius-xl": "20px" },
  },
  {
    id: "moonlight", label: "Moonlight", accent: "#7dd3fc",
    vars: { "--bg-primary": "#0c1220", "--bg-secondary": "#111a2e", "--bg-tertiary": "#18243e", "--bg-elevated": "#1e2e50", "--accent-primary": "#7dd3fc", "--accent-hover": "#bae6fd", "--text-primary": "#f0f8ff", "--text-secondary": "#93c5fd", "--text-muted": "#4a7fc1", "--border": "#18243e", "--border-hover": "#1e2e50", "--success": "#34d399", "--warning": "#fbbf24", "--danger": "#f87171", "--info": "#60a5fa", "--radius-sm": "8px", "--radius-md": "12px", "--radius-lg": "16px", "--radius-xl": "22px" },
  },
  {
    id: "ember-glow", label: "Ember Glow", accent: "#fb923c",
    vars: { "--bg-primary": "#100800", "--bg-secondary": "#1c1000", "--bg-tertiary": "#2a1800", "--bg-elevated": "#3d2400", "--accent-primary": "#fb923c", "--accent-hover": "#fdba74", "--text-primary": "#fff7ed", "--text-secondary": "#fdba74", "--text-muted": "#9a3412", "--border": "#2a1800", "--border-hover": "#3d2400", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "10px", "--radius-lg": "14px", "--radius-xl": "20px" },
  },
  {
    id: "cobalt-night", label: "Cobalt Night", accent: "#2563eb",
    vars: { "--bg-primary": "#05091a", "--bg-secondary": "#080e28", "--bg-tertiary": "#0d1438", "--bg-elevated": "#141c50", "--accent-primary": "#2563eb", "--accent-hover": "#3b82f6", "--text-primary": "#eff6ff", "--text-secondary": "#93c5fd", "--text-muted": "#1d4ed8", "--border": "#0d1438", "--border-hover": "#141c50", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#38bdf8", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "volcanic", label: "Volcanic", accent: "#f97316",
    vars: { "--bg-primary": "#0a0806", "--bg-secondary": "#150f0a", "--bg-tertiary": "#221710", "--bg-elevated": "#301f14", "--accent-primary": "#f97316", "--accent-hover": "#fb923c", "--text-primary": "#faf9f8", "--text-secondary": "#a8978a", "--text-muted": "#786050", "--border": "#301f14", "--border-hover": "#402818", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "4px", "--radius-md": "6px", "--radius-lg": "8px", "--radius-xl": "12px" },
  },
  {
    id: "sakura", label: "Sakura (Clair)", accent: "#ec4899",
    vars: { "--bg-primary": "#fff8fb", "--bg-secondary": "#fdf2f8", "--bg-tertiary": "#fce7f3", "--bg-elevated": "#fbcfe8", "--accent-primary": "#ec4899", "--accent-hover": "#f472b6", "--text-primary": "#1e0a14", "--text-secondary": "#701a45", "--text-muted": "#9d174d", "--border": "#fce7f3", "--border-hover": "#fbcfe8", "--success": "#16a34a", "--warning": "#ca8a04", "--danger": "#dc2626", "--info": "#0284c7", "--radius-sm": "8px", "--radius-md": "12px", "--radius-lg": "18px", "--radius-xl": "24px" },
  },
  {
    id: "jade-temple", label: "Jade Temple", accent: "#10b981",
    vars: { "--bg-primary": "#03100a", "--bg-secondary": "#071a10", "--bg-tertiary": "#0e2818", "--bg-elevated": "#143820", "--accent-primary": "#10b981", "--accent-hover": "#34d399", "--text-primary": "#ecfdf5", "--text-secondary": "#6ee7b7", "--text-muted": "#065f46", "--border": "#0e2818", "--border-hover": "#143820", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "10px", "--radius-lg": "14px", "--radius-xl": "20px" },
  },
  {
    id: "hacker", label: "Hacker Terminal", accent: "#00ff41",
    vars: { "--bg-primary": "#000000", "--bg-secondary": "#001500", "--bg-tertiary": "#002800", "--bg-elevated": "#003d00", "--accent-primary": "#00ff41", "--accent-hover": "#39ff5a", "--text-primary": "#00ff41", "--text-secondary": "#00b32b", "--text-muted": "#007a1e", "--border": "#003000", "--border-hover": "#005000", "--success": "#00ff41", "--warning": "#ffff00", "--danger": "#ff0040", "--info": "#00aaff", "--radius-sm": "0px", "--radius-md": "2px", "--radius-lg": "4px", "--radius-xl": "6px" },
  },
  {
    id: "ice-storm", label: "Ice Storm (Clair)", accent: "#0284c7",
    vars: { "--bg-primary": "#f0f9ff", "--bg-secondary": "#e0f2fe", "--bg-tertiary": "#bae6fd", "--bg-elevated": "#7dd3fc", "--accent-primary": "#0284c7", "--accent-hover": "#0369a1", "--text-primary": "#0c1a2e", "--text-secondary": "#1e4c7a", "--text-muted": "#64748b", "--border": "#bae6fd", "--border-hover": "#7dd3fc", "--success": "#15803d", "--warning": "#b45309", "--danger": "#b91c1c", "--info": "#0284c7", "--radius-sm": "6px", "--radius-md": "10px", "--radius-lg": "16px", "--radius-xl": "22px" },
  },

  // ── WINDOWS 11 STYLE ─────────────────────────────────────────────────────
  {
    id: "win11-default", label: "Win11 Défaut (Clair)", accent: "#0067c0",
    vars: { "--bg-primary": "#f3f3f3", "--bg-secondary": "#ebebeb", "--bg-tertiary": "#e0e0e0", "--bg-elevated": "#d5d5d5", "--accent-primary": "#0067c0", "--accent-hover": "#0078d4", "--text-primary": "#1a1a1a", "--text-secondary": "#4a4a4a", "--text-muted": "#767676", "--border": "#d0d0d0", "--border-hover": "#b8b8b8", "--success": "#107c10", "--warning": "#ca5010", "--danger": "#c42b1c", "--info": "#0067c0", "--radius-sm": "4px", "--radius-md": "6px", "--radius-lg": "8px", "--radius-xl": "12px" },
  },
  {
    id: "win11-dark", label: "Win11 Sombre", accent: "#0078d4",
    vars: { "--bg-primary": "#202020", "--bg-secondary": "#2b2b2b", "--bg-tertiary": "#383838", "--bg-elevated": "#454545", "--accent-primary": "#0078d4", "--accent-hover": "#1084d8", "--text-primary": "#ffffff", "--text-secondary": "#c8c8c8", "--text-muted": "#8a8a8a", "--border": "#393939", "--border-hover": "#4f4f4f", "--success": "#6ccb5f", "--warning": "#fce100", "--danger": "#ff99a4", "--info": "#60cdff", "--radius-sm": "4px", "--radius-md": "6px", "--radius-lg": "8px", "--radius-xl": "12px" },
  },
  {
    id: "win11-glow", label: "Win11 Glow", accent: "#a78bfa",
    vars: { "--bg-primary": "#0d0b1a", "--bg-secondary": "#14112a", "--bg-tertiary": "#1e1a3a", "--bg-elevated": "#2a254e", "--accent-primary": "#a78bfa", "--accent-hover": "#c4b5fd", "--text-primary": "#f5f3ff", "--text-secondary": "#ddd6fe", "--text-muted": "#7c3aed", "--border": "#1e1a3a", "--border-hover": "#2a254e", "--success": "#4ade80", "--warning": "#fbbf24", "--danger": "#f87171", "--info": "#60a5fa", "--radius-sm": "6px", "--radius-md": "10px", "--radius-lg": "16px", "--radius-xl": "24px" },
  },
  {
    id: "win11-sunrise", label: "Win11 Sunrise", accent: "#ff8c00",
    vars: { "--bg-primary": "#120b00", "--bg-secondary": "#1e1100", "--bg-tertiary": "#2e1a00", "--bg-elevated": "#422600", "--accent-primary": "#ff8c00", "--accent-hover": "#ffa733", "--text-primary": "#fff8f0", "--text-secondary": "#ffd59e", "--text-muted": "#b56500", "--border": "#2e1a00", "--border-hover": "#422600", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#38bdf8", "--radius-sm": "6px", "--radius-md": "10px", "--radius-lg": "14px", "--radius-xl": "20px" },
  },
  {
    id: "win11-flow", label: "Win11 Flow", accent: "#6366f1",
    vars: { "--bg-primary": "#08071a", "--bg-secondary": "#100e2a", "--bg-tertiary": "#18163c", "--bg-elevated": "#22204e", "--accent-primary": "#6366f1", "--accent-hover": "#818cf8", "--text-primary": "#eef2ff", "--text-secondary": "#a5b4fc", "--text-muted": "#4f46e5", "--border": "#18163c", "--border-hover": "#22204e", "--success": "#4ade80", "--warning": "#fbbf24", "--danger": "#f87171", "--info": "#38bdf8", "--radius-sm": "8px", "--radius-md": "12px", "--radius-lg": "18px", "--radius-xl": "26px" },
  },
  {
    id: "win11-captured", label: "Win11 Captured Motion", accent: "#00b4d8",
    vars: { "--bg-primary": "#020d12", "--bg-secondary": "#051820", "--bg-tertiary": "#092430", "--bg-elevated": "#0d3040", "--accent-primary": "#00b4d8", "--accent-hover": "#00d4f5", "--text-primary": "#ecfeff", "--text-secondary": "#67e8f9", "--text-muted": "#0e7490", "--border": "#092430", "--border-hover": "#0d3040", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#38bdf8", "--radius-sm": "6px", "--radius-md": "10px", "--radius-lg": "14px", "--radius-xl": "20px" },
  },
  {
    id: "win11-vibes", label: "Win11 Vibes", accent: "#d946ef",
    vars: { "--bg-primary": "#0f0618", "--bg-secondary": "#1a0828", "--bg-tertiary": "#270e3a", "--bg-elevated": "#38154e", "--accent-primary": "#d946ef", "--accent-hover": "#e879f9", "--text-primary": "#fdf4ff", "--text-secondary": "#f0abfc", "--text-muted": "#a21caf", "--border": "#270e3a", "--border-hover": "#38154e", "--success": "#4ade80", "--warning": "#fbbf24", "--danger": "#f43f5e", "--info": "#818cf8", "--radius-sm": "8px", "--radius-md": "12px", "--radius-lg": "18px", "--radius-xl": "26px" },
  },
  {
    id: "win11-storm", label: "Win11 Storm", accent: "#4a90d9",
    vars: { "--bg-primary": "#0c1018", "--bg-secondary": "#131925", "--bg-tertiary": "#1c2535", "--bg-elevated": "#263348", "--accent-primary": "#4a90d9", "--accent-hover": "#6aaee8", "--text-primary": "#edf2f8", "--text-secondary": "#9baec8", "--text-muted": "#5a7a9e", "--border": "#1c2535", "--border-hover": "#263348", "--success": "#48bb78", "--warning": "#ed8936", "--danger": "#fc8181", "--info": "#63b3ed", "--radius-sm": "4px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "win11-galaxy", label: "Win11 Galaxy", accent: "#818cf8",
    vars: { "--bg-primary": "#050510", "--bg-secondary": "#0b0b20", "--bg-tertiary": "#121230", "--bg-elevated": "#1a1a48", "--accent-primary": "#818cf8", "--accent-hover": "#a5b4fc", "--text-primary": "#e0e7ff", "--text-secondary": "#a5b4fc", "--text-muted": "#4338ca", "--border": "#121230", "--border-hover": "#1a1a48", "--success": "#34d399", "--warning": "#fcd34d", "--danger": "#f87171", "--info": "#60a5fa", "--radius-sm": "6px", "--radius-md": "10px", "--radius-lg": "16px", "--radius-xl": "22px" },
  },
  {
    id: "win11-nightfall", label: "Win11 Nightfall", accent: "#3b82f6",
    vars: { "--bg-primary": "#020612", "--bg-secondary": "#06101e", "--bg-tertiary": "#0d1a2e", "--bg-elevated": "#142440", "--accent-primary": "#3b82f6", "--accent-hover": "#60a5fa", "--text-primary": "#f0f6ff", "--text-secondary": "#93c5fd", "--text-muted": "#1d4ed8", "--border": "#0d1a2e", "--border-hover": "#142440", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#38bdf8", "--radius-sm": "4px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "18px" },
  },
  {
    id: "win11-frost", label: "Win11 Frost (Clair)", accent: "#0369a1",
    vars: { "--bg-primary": "#f8fafb", "--bg-secondary": "#eef3f7", "--bg-tertiary": "#dde8ef", "--bg-elevated": "#c8dce8", "--accent-primary": "#0369a1", "--accent-hover": "#0284c7", "--text-primary": "#0c1f2e", "--text-secondary": "#2a4a60", "--text-muted": "#6b8fa8", "--border": "#c8dce8", "--border-hover": "#a8c5d8", "--success": "#166534", "--warning": "#92400e", "--danger": "#991b1b", "--info": "#0369a1", "--radius-sm": "8px", "--radius-md": "12px", "--radius-lg": "18px", "--radius-xl": "26px" },
  },
  {
    id: "win11-desert", label: "Win11 Desert (Clair)", accent: "#b45309",
    vars: { "--bg-primary": "#fdf8f0", "--bg-secondary": "#faf0e0", "--bg-tertiary": "#f5e6cc", "--bg-elevated": "#edd8b0", "--accent-primary": "#b45309", "--accent-hover": "#d97706", "--text-primary": "#1c0f04", "--text-secondary": "#6b3a14", "--text-muted": "#92400e", "--border": "#e8d0a8", "--border-hover": "#d8b880", "--success": "#166534", "--warning": "#92400e", "--danger": "#991b1b", "--info": "#1d4ed8", "--radius-sm": "4px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "18px" },
  },
  {
    id: "win11-rust-canyon", label: "Win11 Rust Canyon", accent: "#dc2626",
    vars: { "--bg-primary": "#110505", "--bg-secondary": "#1c0b0b", "--bg-tertiary": "#2c1212", "--bg-elevated": "#3d1a1a", "--accent-primary": "#dc2626", "--accent-hover": "#ef4444", "--text-primary": "#fef2f2", "--text-secondary": "#fca5a5", "--text-muted": "#991b1b", "--border": "#2c1212", "--border-hover": "#3d1a1a", "--success": "#22c55e", "--warning": "#f97316", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "4px", "--radius-md": "6px", "--radius-lg": "10px", "--radius-xl": "14px" },
  },
  {
    id: "win11-limelight", label: "Win11 Limelight", accent: "#65a30d",
    vars: { "--bg-primary": "#060f02", "--bg-secondary": "#0c1a04", "--bg-tertiary": "#142808", "--bg-elevated": "#1f3c0e", "--accent-primary": "#65a30d", "--accent-hover": "#84cc16", "--text-primary": "#f7fee7", "--text-secondary": "#bef264", "--text-muted": "#4d7c0f", "--border": "#142808", "--border-hover": "#1f3c0e", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#3b82f6", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "win11-cosmic", label: "Win11 Cosmic Teal", accent: "#0d9488",
    vars: { "--bg-primary": "#030e0d", "--bg-secondary": "#071816", "--bg-tertiary": "#0e2522", "--bg-elevated": "#163630", "--accent-primary": "#0d9488", "--accent-hover": "#14b8a6", "--text-primary": "#f0fdfa", "--text-secondary": "#5eead4", "--text-muted": "#0f766e", "--border": "#0e2522", "--border-hover": "#163630", "--success": "#22c55e", "--warning": "#eab308", "--danger": "#ef4444", "--info": "#38bdf8", "--radius-sm": "8px", "--radius-md": "12px", "--radius-lg": "16px", "--radius-xl": "22px" },
  },

  // ── POPULAIRES / APPS CONNUES ─────────────────────────────────────────────
  {
    id: "vscode-dark", label: "VS Code Dark", accent: "#007acc",
    vars: { "--bg-primary": "#1e1e1e", "--bg-secondary": "#252526", "--bg-tertiary": "#2d2d30", "--bg-elevated": "#3c3c3c", "--accent-primary": "#007acc", "--accent-hover": "#1a8cd8", "--text-primary": "#d4d4d4", "--text-secondary": "#9cdcfe", "--text-muted": "#6a9955", "--border": "#3c3c3c", "--border-hover": "#555555", "--success": "#4ec9b0", "--warning": "#dcdcaa", "--danger": "#f44747", "--info": "#569cd6", "--radius-sm": "3px", "--radius-md": "4px", "--radius-lg": "6px", "--radius-xl": "8px" },
  },
  {
    id: "github-dark", label: "GitHub Dark", accent: "#58a6ff",
    vars: { "--bg-primary": "#0d1117", "--bg-secondary": "#161b22", "--bg-tertiary": "#21262d", "--bg-elevated": "#30363d", "--accent-primary": "#58a6ff", "--accent-hover": "#79b8ff", "--text-primary": "#e6edf3", "--text-secondary": "#8b949e", "--text-muted": "#484f58", "--border": "#30363d", "--border-hover": "#484f58", "--success": "#3fb950", "--warning": "#d29922", "--danger": "#f85149", "--info": "#58a6ff", "--radius-sm": "6px", "--radius-md": "6px", "--radius-lg": "8px", "--radius-xl": "12px" },
  },
  {
    id: "discord-dark", label: "Discord Dark", accent: "#5865f2",
    vars: { "--bg-primary": "#202225", "--bg-secondary": "#2f3136", "--bg-tertiary": "#36393f", "--bg-elevated": "#40444b", "--accent-primary": "#5865f2", "--accent-hover": "#7289da", "--text-primary": "#dcddde", "--text-secondary": "#b9bbbe", "--text-muted": "#72767d", "--border": "#40444b", "--border-hover": "#72767d", "--success": "#3ba55d", "--warning": "#faa61a", "--danger": "#ed4245", "--info": "#00b0f4", "--radius-sm": "4px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "notion-light", label: "Notion (Clair)", accent: "#2eaadc",
    vars: { "--bg-primary": "#ffffff", "--bg-secondary": "#f7f6f3", "--bg-tertiary": "#efede8", "--bg-elevated": "#e3e0d8", "--accent-primary": "#2eaadc", "--accent-hover": "#40b4e5", "--text-primary": "#37352f", "--text-secondary": "#787774", "--text-muted": "#9b9a97", "--border": "#e9e9e7", "--border-hover": "#d3d1ca", "--success": "#0f7b6c", "--warning": "#dfab01", "--danger": "#eb5757", "--info": "#2eaadc", "--radius-sm": "3px", "--radius-md": "4px", "--radius-lg": "6px", "--radius-xl": "8px" },
  },
  {
    id: "terminal-green", label: "Terminal Vert", accent: "#39ff14",
    vars: { "--bg-primary": "#0a0f0a", "--bg-secondary": "#0f160f", "--bg-tertiary": "#162016", "--bg-elevated": "#1e2e1e", "--accent-primary": "#39ff14", "--accent-hover": "#57ff36", "--text-primary": "#c8ffc8", "--text-secondary": "#7dbf7d", "--text-muted": "#3d7a3d", "--border": "#162016", "--border-hover": "#1e2e1e", "--success": "#39ff14", "--warning": "#ffcc00", "--danger": "#ff4444", "--info": "#00ccff", "--radius-sm": "0px", "--radius-md": "2px", "--radius-lg": "4px", "--radius-xl": "6px" },
  },
  {
    id: "dracula", label: "Dracula", accent: "#bd93f9",
    vars: { "--bg-primary": "#282a36", "--bg-secondary": "#1e1f29", "--bg-tertiary": "#343746", "--bg-elevated": "#44475a", "--accent-primary": "#bd93f9", "--accent-hover": "#d0b0ff", "--text-primary": "#f8f8f2", "--text-secondary": "#6272a4", "--text-muted": "#44475a", "--border": "#44475a", "--border-hover": "#6272a4", "--success": "#50fa7b", "--warning": "#ffb86c", "--danger": "#ff5555", "--info": "#8be9fd", "--radius-sm": "4px", "--radius-md": "6px", "--radius-lg": "10px", "--radius-xl": "14px" },
  },
  {
    id: "monokai-pro", label: "Monokai Pro", accent: "#ff6188",
    vars: { "--bg-primary": "#2d2a2e", "--bg-secondary": "#221f22", "--bg-tertiary": "#3a373a", "--bg-elevated": "#4a4748", "--accent-primary": "#ff6188", "--accent-hover": "#ff82a0", "--text-primary": "#fcfcfa", "--text-secondary": "#c1c0c0", "--text-muted": "#727072", "--border": "#3a373a", "--border-hover": "#5a585a", "--success": "#a9dc76", "--warning": "#ffd866", "--danger": "#ff6188", "--info": "#78dce8", "--radius-sm": "3px", "--radius-md": "5px", "--radius-lg": "8px", "--radius-xl": "12px" },
  },
  {
    id: "nord", label: "Nord", accent: "#88c0d0",
    vars: { "--bg-primary": "#2e3440", "--bg-secondary": "#3b4252", "--bg-tertiary": "#434c5e", "--bg-elevated": "#4c566a", "--accent-primary": "#88c0d0", "--accent-hover": "#8fbcbb", "--text-primary": "#eceff4", "--text-secondary": "#d8dee9", "--text-muted": "#81a1c1", "--border": "#434c5e", "--border-hover": "#4c566a", "--success": "#a3be8c", "--warning": "#ebcb8b", "--danger": "#bf616a", "--info": "#81a1c1", "--radius-sm": "4px", "--radius-md": "6px", "--radius-lg": "10px", "--radius-xl": "14px" },
  },
  {
    id: "solarized-dark", label: "Solarized Dark", accent: "#268bd2",
    vars: { "--bg-primary": "#002b36", "--bg-secondary": "#073642", "--bg-tertiary": "#0d4455", "--bg-elevated": "#115a6e", "--accent-primary": "#268bd2", "--accent-hover": "#2aa0f0", "--text-primary": "#fdf6e3", "--text-secondary": "#93a1a1", "--text-muted": "#586e75", "--border": "#0d4455", "--border-hover": "#115a6e", "--success": "#859900", "--warning": "#b58900", "--danger": "#dc322f", "--info": "#268bd2", "--radius-sm": "4px", "--radius-md": "6px", "--radius-lg": "8px", "--radius-xl": "12px" },
  },
  {
    id: "catppuccin", label: "Catppuccin Mocha", accent: "#cba6f7",
    vars: { "--bg-primary": "#1e1e2e", "--bg-secondary": "#181825", "--bg-tertiary": "#313244", "--bg-elevated": "#45475a", "--accent-primary": "#cba6f7", "--accent-hover": "#d8b4fe", "--text-primary": "#cdd6f4", "--text-secondary": "#bac2de", "--text-muted": "#6c7086", "--border": "#313244", "--border-hover": "#45475a", "--success": "#a6e3a1", "--warning": "#fab387", "--danger": "#f38ba8", "--info": "#89b4fa", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "one-dark", label: "One Dark Pro", accent: "#e06c75",
    vars: { "--bg-primary": "#282c34", "--bg-secondary": "#21252b", "--bg-tertiary": "#2c313c", "--bg-elevated": "#3e4451", "--accent-primary": "#e06c75", "--accent-hover": "#e88c95", "--text-primary": "#abb2bf", "--text-secondary": "#9da5b4", "--text-muted": "#5c6370", "--border": "#3e4451", "--border-hover": "#5c6370", "--success": "#98c379", "--warning": "#e5c07b", "--danger": "#e06c75", "--info": "#61afef", "--radius-sm": "4px", "--radius-md": "6px", "--radius-lg": "10px", "--radius-xl": "14px" },
  },
  {
    id: "tokyo-night", label: "Tokyo Night", accent: "#7aa2f7",
    vars: { "--bg-primary": "#1a1b26", "--bg-secondary": "#16161e", "--bg-tertiary": "#1f2335", "--bg-elevated": "#292e42", "--accent-primary": "#7aa2f7", "--accent-hover": "#9ab7ff", "--text-primary": "#c0caf5", "--text-secondary": "#a9b1d6", "--text-muted": "#565f89", "--border": "#1f2335", "--border-hover": "#292e42", "--success": "#9ece6a", "--warning": "#e0af68", "--danger": "#f7768e", "--info": "#7dcfff", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "18px" },
  },
  {
    id: "gruvbox", label: "Gruvbox Dark", accent: "#d65d0e",
    vars: { "--bg-primary": "#282828", "--bg-secondary": "#1d2021", "--bg-tertiary": "#3c3836", "--bg-elevated": "#504945", "--accent-primary": "#d65d0e", "--accent-hover": "#fe8019", "--text-primary": "#ebdbb2", "--text-secondary": "#bdae93", "--text-muted": "#7c6f64", "--border": "#3c3836", "--border-hover": "#504945", "--success": "#98971a", "--warning": "#d79921", "--danger": "#cc241d", "--info": "#458588", "--radius-sm": "2px", "--radius-md": "4px", "--radius-lg": "6px", "--radius-xl": "8px" },
  },
  {
    id: "material-ocean", label: "Material Ocean", accent: "#82aaff",
    vars: { "--bg-primary": "#0f111a", "--bg-secondary": "#090b10", "--bg-tertiary": "#1a1c25", "--bg-elevated": "#1f2233", "--accent-primary": "#82aaff", "--accent-hover": "#9ec0ff", "--text-primary": "#eeffff", "--text-secondary": "#8796b0", "--text-muted": "#464b5d", "--border": "#1a1c25", "--border-hover": "#1f2233", "--success": "#c3e88d", "--warning": "#ffcb6b", "--danger": "#ff5370", "--info": "#89ddff", "--radius-sm": "4px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px" },
  },
  {
    id: "ayu-dark", label: "Ayu Dark", accent: "#ff8f40",
    vars: { "--bg-primary": "#0a0e14", "--bg-secondary": "#0d1017", "--bg-tertiary": "#131721", "--bg-elevated": "#1a2132", "--accent-primary": "#ff8f40", "--accent-hover": "#ffaa66", "--text-primary": "#b3b1ad", "--text-secondary": "#6c7680", "--text-muted": "#3d4354", "--border": "#131721", "--border-hover": "#1a2132", "--success": "#91b362", "--warning": "#f2ae49", "--danger": "#ea6c73", "--info": "#36a3d9", "--radius-sm": "3px", "--radius-md": "5px", "--radius-lg": "8px", "--radius-xl": "12px" },
  },
];

export const PRESET_THEME_GROUPS = [
  {
    label: "🎨 Nitrite Originals",
    ids: ["nitrite-dark","cyber-blue","matrix-green","purple-haze","red-alert","arctic-light","midnight-gold","neon-synthwave","ocean-deep","rose-quartz","void-dark","forest-green","copper-rust","slate-steel"],
  },
  {
    label: "🔥 Ambiance",
    ids: ["inferno","ember-glow","volcanic","aurora","moonlight","cobalt-night","jade-temple","hacker","ice-storm","sakura"],
  },
  {
    label: "🪟 Windows 11",
    ids: ["win11-default","win11-dark","win11-glow","win11-sunrise","win11-flow","win11-captured","win11-vibes","win11-storm","win11-galaxy","win11-nightfall","win11-frost","win11-desert","win11-rust-canyon","win11-limelight","win11-cosmic"],
  },
  {
    label: "💻 Apps & Éditeurs",
    ids: ["vscode-dark","github-dark","discord-dark","notion-light","terminal-green","dracula","monokai-pro","nord","solarized-dark","catppuccin","one-dark","tokyo-night","gruvbox","material-ocean","ayu-dark"],
  },
] as const;

export const useThemeEditorStore = defineStore("themeEditor", () => {
  const editingVars = ref<Record<string, string>>({ ...PRESET_THEMES[0].vars });
  const themeName = ref("Mon Thème");
  const savedThemes = ref<CustomSavedTheme[]>([]);
  const globalPreviewActive = ref(false);

  function loadSavedThemes() {
    try {
      const raw = localStorage.getItem("nitrite-custom-themes");
      if (raw) savedThemes.value = JSON.parse(raw);
    } catch {}
  }

  function loadPreset(presetId: string) {
    const preset = PRESET_THEMES.find(p => p.id === presetId);
    if (preset) {
      editingVars.value = { ...preset.vars };
      themeName.value = preset.label;
    }
  }

  function setVar(key: string, val: string) {
    editingVars.value = { ...editingVars.value, [key]: val };
    if (globalPreviewActive.value) applyToDocument();
  }

  function applyToDocument() {
    for (const [k, v] of Object.entries(editingVars.value)) {
      document.documentElement.style.setProperty(k, v);
      // Compute derived vars
      if (k === "--accent-primary") {
        const rgb = hexToRgb(v);
        if (rgb) {
          document.documentElement.style.setProperty("--accent-muted", `rgba(${rgb}, 0.12)`);
          document.documentElement.style.setProperty("--accent-glow", `0 0 24px rgba(${rgb}, 0.4)`);
          document.documentElement.style.setProperty("--accent-glow-sm", `0 0 10px rgba(${rgb}, 0.3)`);
        }
      }
      if (k === "--success") {
        const rgb = hexToRgb(v);
        if (rgb) document.documentElement.style.setProperty("--success-muted", `rgba(${rgb}, 0.12)`);
      }
      if (k === "--warning") {
        const rgb = hexToRgb(v);
        if (rgb) document.documentElement.style.setProperty("--warning-muted", `rgba(${rgb}, 0.12)`);
      }
      if (k === "--danger") {
        const rgb = hexToRgb(v);
        if (rgb) document.documentElement.style.setProperty("--danger-muted", `rgba(${rgb}, 0.12)`);
      }
      if (k === "--info") {
        const rgb = hexToRgb(v);
        if (rgb) document.documentElement.style.setProperty("--info-muted", `rgba(${rgb}, 0.12)`);
      }
    }
  }

  function clearDocumentVars() {
    for (const key of Object.keys(editingVars.value)) {
      document.documentElement.style.removeProperty(key);
    }
    for (const k of ["--accent-muted","--accent-glow","--accent-glow-sm","--success-muted","--warning-muted","--danger-muted","--info-muted"]) {
      document.documentElement.style.removeProperty(k);
    }
  }

  function toggleGlobalPreview(active: boolean) {
    globalPreviewActive.value = active;
    if (active) applyToDocument();
    else clearDocumentVars();
  }

  function saveTheme() {
    const id = `custom_${Date.now()}`;
    const theme: CustomSavedTheme = { id, name: themeName.value, vars: { ...editingVars.value }, createdAt: Date.now() };
    savedThemes.value = [theme, ...savedThemes.value];
    localStorage.setItem("nitrite-custom-themes", JSON.stringify(savedThemes.value));
    return theme;
  }

  function loadSavedTheme(id: string) {
    const t = savedThemes.value.find(x => x.id === id);
    if (t) { editingVars.value = { ...t.vars }; themeName.value = t.name; }
  }

  function deleteSavedTheme(id: string) {
    savedThemes.value = savedThemes.value.filter(x => x.id !== id);
    localStorage.setItem("nitrite-custom-themes", JSON.stringify(savedThemes.value));
  }

  function exportTheme(): string {
    return JSON.stringify({ name: themeName.value, vars: editingVars.value }, null, 2);
  }

  function importTheme(json: string): { ok: boolean; error?: string } {
    try {
      const parsed = JSON.parse(json);
      if (!parsed || typeof parsed !== "object") return { ok: false, error: "Format invalide" };
      if (!parsed.vars || typeof parsed.vars !== "object") return { ok: false, error: "Champ 'vars' manquant" };
      const FORBIDDEN = /<\/?(style|script|link)/i;
      const safeVars: Record<string, string> = {};
      for (const [k, v] of Object.entries(parsed.vars as Record<string, unknown>)) {
        if (!String(k).startsWith("--")) continue;
        const val = String(v);
        if (FORBIDDEN.test(val) || val.includes("javascript:") || val.includes("expression(")) continue;
        safeVars[k] = val;
      }
      if (Object.keys(safeVars).length === 0) return { ok: false, error: "Aucune variable CSS valide trouvée" };
      editingVars.value = safeVars;
      themeName.value = typeof parsed.name === "string" ? parsed.name.slice(0, 80) : "Thème importé";
      return { ok: true };
    } catch {
      return { ok: false, error: "JSON invalide" };
    }
  }

  // Computed preview style object for scoped previews
  const previewStyle = computed(() => {
    const style: Record<string, string> = {};
    for (const [k, v] of Object.entries(editingVars.value)) {
      style[k] = v;
    }
    // Derived
    const acc = editingVars.value["--accent-primary"] ?? "#f97316";
    const rgb = hexToRgb(acc);
    if (rgb) {
      style["--accent-muted"] = `rgba(${rgb}, 0.12)`;
      style["--accent-subtle"] = `rgba(${rgb}, 0.06)`;
      style["--accent-glow"] = `0 0 24px rgba(${rgb}, 0.4)`;
    }
    return style;
  });

  return {
    editingVars, themeName, savedThemes, globalPreviewActive,
    loadSavedThemes, loadPreset, setVar, applyToDocument, clearDocumentVars,
    toggleGlobalPreview, saveTheme, loadSavedTheme, deleteSavedTheme,
    exportTheme, importTheme, previewStyle,
  };
});

function hexToRgb(hex: string): string | null {
  const m = hex.replace("#", "").match(/.{2}/g);
  if (!m || m.length < 3) return null;
  return `${parseInt(m[0], 16)}, ${parseInt(m[1], 16)}, ${parseInt(m[2], 16)}`;
}
