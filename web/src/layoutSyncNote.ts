import type { SyncState } from "./layoutSync";

const TEXTS: Record<SyncState, string> = {
  none: "",
  synced: "Layout salvo também no PC.",
  pending: "Layout só neste aparelho por enquanto: sem conexão com o PC. Será enviado depois.",
  rejected: "O PC recusou este layout; ele vale só neste aparelho.",
};

/** Mostra no painel se o layout já está guardado no PC. */
export function renderSyncNote(el: HTMLElement, state: SyncState): void {
  el.textContent = TEXTS[state]; // textContent: nada vira HTML
  el.dataset.state = state;
}
