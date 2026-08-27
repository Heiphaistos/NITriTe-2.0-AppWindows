/**
 * Seuils d'alerte CPU/RAM/disque. Defini ici plutot que dans
 * `AlertThresholdsModal.vue` : `tsc --noEmit` (CI) ne sait pas lire un SFC,
 * le shim `*.vue` ne declare qu'un export par defaut, donc importer un type
 * depuis un `.vue` cassait la CI (TS2614). Bonus : supprime le cycle
 * modal -> utils -> modal.
 */
export interface AlertThresholds {
  cpu_warn: number;
  cpu_crit: number;
  ram_warn: number;
  ram_crit: number;
  disk_warn: number;
  disk_crit: number;
}

const MIN = 50;
const MAX = 100;

function clampPercent(n: number, fallback: number): number {
  if (!Number.isFinite(n)) return fallback;
  return Math.min(MAX, Math.max(MIN, Math.round(n)));
}

/**
 * Force min<=valeur<=max sur chaque seuil, et seuil critique >= seuil
 * d'avertissement pour chaque paire (cpu/ram/disk) — rien dans l'UI
 * (`AlertThresholdsModal.vue`, inputs `type="number"` avec `min`/`max`
 * seulement décoratifs, jamais appliqués par `v-model.number`) n'empêchait
 * jusqu'ici de saisir un seuil critique inférieur au seuil d'avertissement.
 * Conséquence silencieuse : `DashboardPage.vue::computeHealthScore()` et
 * `checkThresholdAlerts()` testent toujours "critique" avant "avertissement"
 * (`if (x >= crit) ... else if (x >= warn) ...`), donc si crit < warn le
 * palier "avertissement" devient inatteignable — chaque dépassement du seuil
 * (plus bas) est immédiatement classé "critique", sans jamais passer par
 * l'étage intermédiaire pourtant configuré par l'utilisateur.
 */
export function normalizeThresholds(input: AlertThresholds, fallback: AlertThresholds): AlertThresholds {
  const out: AlertThresholds = {
    cpu_warn: clampPercent(input.cpu_warn, fallback.cpu_warn),
    cpu_crit: clampPercent(input.cpu_crit, fallback.cpu_crit),
    ram_warn: clampPercent(input.ram_warn, fallback.ram_warn),
    ram_crit: clampPercent(input.ram_crit, fallback.ram_crit),
    disk_warn: clampPercent(input.disk_warn, fallback.disk_warn),
    disk_crit: clampPercent(input.disk_crit, fallback.disk_crit),
  };
  if (out.cpu_crit < out.cpu_warn) [out.cpu_warn, out.cpu_crit] = [out.cpu_crit, out.cpu_warn];
  if (out.ram_crit < out.ram_warn) [out.ram_warn, out.ram_crit] = [out.ram_crit, out.ram_warn];
  if (out.disk_crit < out.disk_warn) [out.disk_warn, out.disk_crit] = [out.disk_crit, out.disk_warn];
  return out;
}
