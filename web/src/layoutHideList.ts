import { t } from "./i18n";
import { CONTROL_IDS, isHidden, type ControlId } from "./layoutModel";
import type { Key } from "./locales/pt-BR";
import type { LayoutStore } from "./layoutStore.svelte";

/** Nomes que mudam com o idioma; os demais (A, LB, Select…) são o próprio id do controle. */
const KEYED: Partial<Record<ControlId, Key>> = {
  Up: "direction.up",
  Down: "direction.down",
  Left: "direction.left",
  Right: "direction.right",
  StickLeft: "hideList.stickLeft",
  StickRight: "hideList.stickRight",
};

const nameOf = (id: ControlId): string => {
  const key = KEYED[id];
  return key ? t(key) : id;
};

interface Options {
  panel: HTMLElement;
  opener: HTMLElement;
  store: LayoutStore;
  /** chamado quando um interruptor muda o rascunho */
  onChange?: () => void;
  /** nome que o controle tem agora (segue o perfil); null usa o nome padrão daqui */
  labelOf?: (id: ControlId) => string | null;
}

/**
 * Lista de controles para ocultar um a um (só em modo edição). Cada interruptor diz se o
 * controle está visível; `sync` o relê do store e fecha a lista quando a edição acaba.
 */
export function mountHideList({ panel, opener, store, onChange, labelOf }: Options): { sync: () => void } {
  let open = false;
  const chips = new Map<ControlId, HTMLElement>();

  for (const id of CONTROL_IDS) {
    const chip = document.createElement("button");
    chip.type = "button";
    chip.dataset.control = id;
    chip.setAttribute("role", "switch");
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
    for (const [id, chip] of chips) {
      const name = labelOf?.(id) ?? nameOf(id);
      chip.textContent = name;
      chip.setAttribute("aria-label", name);
      chip.setAttribute("aria-checked", String(!isHidden(store.current, id)));
    }
  };

  opener.addEventListener("click", () => {
    open = store.editing && !open;
    sync();
  });

  sync();
  return { sync };
}
