import { t } from "./i18n";
import type { SyncState } from "./layoutSync";

const KEYS = {
  synced: "layoutSync.synced",
  pending: "layoutSync.pending",
  rejected: "layoutSync.rejected",
} as const;

/** Mostra no painel se o layout já está guardado no PC. */
export function renderSyncNote(el: HTMLElement, state: SyncState): void {
  el.textContent = state === "none" ? "" : t(KEYS[state]); // textContent: nada vira HTML
  el.dataset.state = state;
}
