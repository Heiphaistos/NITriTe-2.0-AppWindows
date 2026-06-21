/**
 * Wrapper autour de invoke() de Tauri avec timeout configurable.
 * Évite les freezes UI si une commande Rust ne répond pas (WMI hang, disque mort, etc.)
 */
import { invoke as tauriInvoke } from "@tauri-apps/api/core";

const DEFAULT_TIMEOUT_MS = 30_000;

export class InvokeTimeoutError extends Error {
  constructor(cmd: string, ms: number) {
    super(`Commande "${cmd}" timeout après ${ms}ms`);
    this.name = "InvokeTimeoutError";
  }
}

/**
 * invoke() avec timeout. Lance une erreur `InvokeTimeoutError` si le délai est dépassé.
 * @param cmd   - nom de la commande Tauri
 * @param args  - arguments (optionnel)
 * @param ms    - timeout en millisecondes (défaut: 15s)
 */
export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
  ms = DEFAULT_TIMEOUT_MS,
): Promise<T> {
  let timerId: ReturnType<typeof setTimeout> | undefined;

  const timeout = new Promise<never>((_, reject) => {
    timerId = setTimeout(() => reject(new InvokeTimeoutError(cmd, ms)), ms);
  });

  try {
    const result = await Promise.race([tauriInvoke<T>(cmd, args), timeout]);
    return result;
  } finally {
    clearTimeout(timerId);
  }
}

/**
 * invoke() sans timeout — à utiliser uniquement pour les commandes longues intentionnellement
 * (scan total, export, etc.)
 */
export const invokeRaw = tauriInvoke;
