import { invoke, invokeRaw } from "@/utils/invoke";
import { useNotificationStore } from "@/stores/notifications";

export { invoke, invokeRaw, useNotificationStore };

export function kbStr(v: number): string {
  return v >= 1024 ? `${(v / 1024).toFixed(0)} MB` : `${v} KB`;
}

/**
 * Échappe une valeur pour l'insérer dans une cellule de tableau ou un span de
 * code inline Markdown : \ en premier (pour ne pas ré-échapper le \| inséré
 * ensuite), | (délimiteur de colonne), retour à la ligne → espace (empêche
 * une valeur d'injecter un titre/bloc supplémentaire), backtick → apostrophe
 * (empêche de casser un span ``` ` ```), et <> (la plupart des convertisseurs
 * Markdown→HTML conformes CommonMark laissent passer le HTML brut inline tel
 * quel par conception — sans ça, un processus/fichier nommé avec un tag HTML
 * survivrait et s'exécuterait si ce rapport de scan, qui liste précisément des
 * entités système potentiellement malveillantes, est un jour rendu en HTML).
 */
export function mdCell(s: unknown): string {
  return String(s ?? "")
    .replace(/\\/g, "\\\\")
    .replace(/\r?\n/g, " ")
    .replace(/\|/g, "\\|")
    .replace(/`/g, "'")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

export function fullRegPath(location: string, name?: string): string {
  let p = location
    .replace(/^HKCU(\\|$)/, "HKEY_CURRENT_USER$1")
    .replace(/^HKLM(\\|$)/, "HKEY_LOCAL_MACHINE$1")
    .replace(/^HKCR(\\|$)/, "HKEY_CLASSES_ROOT$1")
    .replace(/^HKU(\\|$)/, "HKEY_USERS$1");
  if (name) p = p + (p.endsWith("\\") ? "" : "\\") + name;
  return p;
}

export interface Solution {
  problem: string;
  action: string;
  repairKey?: string;
  severity: "critical" | "warning" | "info";
}
