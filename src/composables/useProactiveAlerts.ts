import { ref, onUnmounted, getCurrentInstance } from 'vue';
import { invoke } from "@/utils/invoke";
import type { SysInfo, SmartDiskInfo } from "@/types/diagnostic";

interface AlertThresholds {
  cpuTempCritical: number;   // °C
  gpuTempCritical: number;   // °C
  diskUsageWarn: number;     // %
  diskUsageCritical: number; // %
  ramUsageWarn: number;      // %
}

const DEFAULT_THRESHOLDS: AlertThresholds = {
  cpuTempCritical: 90,
  gpuTempCritical: 85,
  diskUsageWarn: 85,
  diskUsageCritical: 95,
  ramUsageWarn: 90,
};

export const activeAlerts = ref<Array<{
  id: string;
  type: 'temp' | 'disk' | 'ram' | 'smart';
  severity: 'warning' | 'critical';
  message: string;
  timestamp: Date;
  dismissed: boolean;
}>>([]);

let monitorInterval: ReturnType<typeof setInterval> | null = null;
let instanceCount = 0;

function addAlert(id: string, type: 'temp' | 'disk' | 'ram' | 'smart', severity: 'warning' | 'critical', message: string) {
  const existing = activeAlerts.value.find(a => a.id === id && !a.dismissed);
  if (existing) return; // déjà présent
  activeAlerts.value.push({ id, type, severity, message, timestamp: new Date(), dismissed: false });
  // Notification système (si permission)
  if ('Notification' in window && Notification.permission === 'granted') {
    new Notification(`Nitrite — ${severity === 'critical' ? '🔴' : '🟡'} ${message}`);
  }
}

// Retire les alertes d'un type dont l'id n'est plus dans currentIds : sans ça,
// une alerte restait affichée indéfiniment (bannière globale sur toute l'app,
// voir NAlertBanner.vue) même longtemps après le retour à la normale — un pic
// de température transitoire de quelques secondes laissait un faux "Surchauffe
// critique !" épinglé jusqu'au dismiss manuel, et empêchait toute nouvelle
// notification pour une récidive du même capteur tant que l'ancienne alerte
// n'était pas fermée. N'est appelé qu'après un sondage RÉUSSI (currentIds
// reflète l'état réel) — un échec de sondage ne doit jamais être interprété
// comme "condition résolue".
function clearResolvedAlerts(type: 'temp' | 'disk' | 'ram' | 'smart', currentIds: Set<string>) {
  activeAlerts.value = activeAlerts.value.filter(a => a.type !== type || currentIds.has(a.id));
}

export function dismissAlert(id: string) {
  const alert = activeAlerts.value.find(a => a.id === id);
  if (alert) alert.dismissed = true;
}

export function dismissAll() {
  activeAlerts.value.forEach(a => { a.dismissed = true; });
}

export function useProactiveAlerts(thresholds: AlertThresholds = DEFAULT_THRESHOLDS) {
  async function checkOnce() {
    try {
      // Vérifier températures
      const temps = await invoke<Array<{ sensor_name: string; temp_celsius: number; source: string }>>('get_temperatures');
      const currentTempIds = new Set<string>();
      for (const t of temps) {
        if (t.temp_celsius <= 0) continue;
        if (t.sensor_name.toLowerCase().includes('cpu') || t.source.toLowerCase().includes('cpu')) {
          if (t.temp_celsius >= thresholds.cpuTempCritical) {
            const id = `cpu-temp-${t.sensor_name}`;
            addAlert(id, 'temp', 'critical', `CPU ${t.sensor_name}: ${t.temp_celsius}°C — Surchauffe critique!`);
            currentTempIds.add(id);
          }
        }
        if (t.sensor_name.toLowerCase().includes('gpu')) {
          if (t.temp_celsius >= thresholds.gpuTempCritical) {
            const id = `gpu-temp-${t.sensor_name}`;
            addAlert(id, 'temp', 'critical', `GPU ${t.sensor_name}: ${t.temp_celsius}°C — Surchauffe!`);
            currentTempIds.add(id);
          }
        }
      }
      clearResolvedAlerts('temp', currentTempIds);
    } catch { /* sondage échoué : état inconnu, ne pas toucher aux alertes existantes */ }

    try {
      // Vérifier utilisation disques/RAM
      const sysInfo = await invoke<SysInfo>('get_system_info').catch(() => null);
      if (sysInfo) {
        const currentRamIds = new Set<string>();
        if (sysInfo.ram?.usage_percent >= thresholds.ramUsageWarn) {
          const sev = sysInfo.ram.usage_percent >= 95 ? 'critical' : 'warning';
          addAlert('ram-usage', 'ram', sev, `RAM: ${sysInfo.ram.usage_percent.toFixed(0)}% utilisée`);
          currentRamIds.add('ram-usage');
        }
        clearResolvedAlerts('ram', currentRamIds);

        const currentDiskIds = new Set<string>();
        if (sysInfo.disks) {
          for (const d of sysInfo.disks) {
            for (const p of (d.partitions || [])) {
              const label = p.mount_point || d.model || 'Disque';
              const id = `disk-${label}`;
              if (p.usage_percent >= thresholds.diskUsageCritical) {
                addAlert(id, 'disk', 'critical', `Disque ${label}: ${p.usage_percent.toFixed(0)}% plein!`);
                currentDiskIds.add(id);
              } else if (p.usage_percent >= thresholds.diskUsageWarn) {
                addAlert(id, 'disk', 'warning', `Disque ${label}: ${p.usage_percent.toFixed(0)}% utilisé`);
                currentDiskIds.add(id);
              }
            }
          }
        }
        clearResolvedAlerts('disk', currentDiskIds);
      }
      // sysInfo null (get_system_info a échoué) : état inconnu, ne rien effacer
    } catch {}

    try {
      // Vérifier SMART
      const smart = await invoke<SmartDiskInfo[]>('get_smart_info');
      const currentSmartIds = new Set<string>();
      for (const s of smart) {
        if (s.reallocated_sectors > 0) {
          const id = `smart-realloc-${s.name}`;
          addAlert(id, 'smart', 'critical',
            `${s.name}: ${s.reallocated_sectors} secteur(s) réalloué(s) — Défaillance imminente!`);
          currentSmartIds.add(id);
        }
        const healthy = ['ok', 'good', 'passed', 'sain', 'healthy'];
        if (s.health_status && !healthy.some(h => s.health_status.toLowerCase().includes(h))) {
          const id = `smart-health-${s.name}`;
          addAlert(id, 'smart', 'critical', `${s.name}: État SMART dégradé (${s.health_status})`);
          currentSmartIds.add(id);
        }
      }
      clearResolvedAlerts('smart', currentSmartIds);
    } catch { /* sondage échoué : état inconnu, ne pas toucher aux alertes existantes */ }
  }

  function start(intervalMs = 60000) {
    instanceCount++;
    if (instanceCount > 1) return; // timer déjà actif — on incrémente juste le compteur
    checkOnce();
    monitorInterval = setInterval(checkOnce, intervalMs);
    if ('Notification' in window && Notification.permission === 'default') {
      Notification.requestPermission();
    }
  }

  function stop() {
    instanceCount = Math.max(0, instanceCount - 1);
    if (instanceCount === 0 && monitorInterval) {
      clearInterval(monitorInterval);
      monitorInterval = null;
    }
  }

  if (getCurrentInstance()) onUnmounted(() => stop());

  return { start, stop, activeAlerts, dismissAlert, dismissAll, checkOnce };
}
