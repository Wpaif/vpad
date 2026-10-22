import { CONTROL_IDS, isHidden, type ControlId } from "./layoutModel";
import type { LayoutStore } from "./layoutStore.svelte";

const NAMES: Record<ControlId, string> = {
  A: "A", B: "B", X: "X", Y: "Y",
  LB: "LB", RB: "RB", LT: "LT", RT: "RT",
  Select: "Select", Start: "Start",
  Up: "Cima", Down: "Baixo", Left: "Esquerda", Right: "Direita",
  StickLeft: "Analógico esq.", StickRight: "Analógico dir.",
};

interface Options {
  panel: HTMLElement;
  opener: HTMLElement;
  store: LayoutStore;
  /** chamado quando um interruptor muda o rascunho */
  onChange?: () => void;
}

/**
 * Lista de controles para ocultar um a um (só em modo edição). Cada interruptor diz se o
 * controle está visível; `sync` o relê do store e fecha a lista quando a edição acaba.
 */
export function mountHideList({ panel, opener, store, onChange }: Options): { sync: () => void } {
  let open = false;
  const chips = new Map<ControlId, HTMLElement>();

  for (const id of CONTROL_IDS) {
    const chip = document.createElement("button");
    chip.type = "button";
    chip.dataset.control = id;
    chip.setAttribute("role", "switch");
    chip.textContent = NAMES[id];
    chip.addEventListener("click", () => {
      if (!store.editing) return;
      store.hide(id, !isHidden(store.current, id));
      sync();
      onChange?.();
    });
    panel.append(chip);
    chips.set(id, chip);
  }

  const sync = () => {
    if (!store.editing) open = false;
    panel.hidden = !open;
    for (const [id, chip] of chips) chip.setAttribute("aria-checked", String(!isHidden(store.current, id)));
  };

  opener.addEventListener("click", () => {
    open = store.editing && !open;
    sync();
  });

  sync();
  return { sync };
}
