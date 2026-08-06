/**
 * Les propriétés WMI de type CIM_DATETIME (ex: `Win32_ShadowCopy.InstallDate`,
 * `Get-ComputerRestorePoint`'s `CreationTime`) reviennent au format DMTF brut
 * "YYYYMMDDHHMMSS.mmmmmm+UUU" — `new Date()` ne sait PAS parser ce format et
 * retourne silencieusement un `Invalid Date` (aucune exception levée, un
 * `try/catch` ne l'intercepte donc jamais). Confirmé en direct sur cette
 * machine : `Get-WmiObject Win32_ShadowCopy` → InstallDate
 * "20260803162919.545193+120" → `new Date(...).toLocaleString("fr-FR")`
 * affichait littéralement le texte "Invalid Date" à l'utilisateur.
 */
export function formatWmiDateTime(raw: string): string {
  if (!raw) return "—";
  const m = raw.match(/^(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})/);
  if (m) {
    const [, y, mo, d, h, mi] = m;
    return `${d}/${mo}/${y} ${h}:${mi}`;
  }
  // Repli : la valeur n'est pas au format DMTF (ex: déjà une chaîne ISO) —
  // tenter le parsing standard plutôt que de rejeter en bloc.
  const parsed = new Date(raw);
  return isNaN(parsed.getTime()) ? raw : parsed.toLocaleString("fr-FR");
}
