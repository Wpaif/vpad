import type { Store } from "./auth";

export const PING_KEY = "vpad:ping-curto";
export const PING_NORMAL_MS = 1000;
/**
 * Um ping a cada 100 ms mantém o rádio Wi-Fi do celular acordado: parado, ele dorme entre os
 * avisos do roteador e o primeiro toque depois de uma pausa chega 50–140 ms atrasado.
 */
export const PING_SHORT_MS = 100;

export const pingIntervalFor = (short: boolean) => (short ? PING_SHORT_MS : PING_NORMAL_MS);

export function loadShortPing(storage: Store): boolean {
  try {
    return storage.getItem(PING_KEY) === "1";
  } catch {
    return false;
  }
}

/** Desligado é o padrão, então desligar só apaga a chave. */
export function saveShortPing(storage: Store, on: boolean): void {
  try {
    if (on) storage.setItem(PING_KEY, "1");
    else storage.removeItem(PING_KEY);
  } catch {
    // sem armazenamento: vale só até recarregar
  }
}
