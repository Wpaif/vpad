import { opacityOf } from "./layoutModel";
import type { LayoutStore } from "./layoutStore.svelte";

interface Options {
  input: HTMLInputElement;
  output?: HTMLElement | null;
  store: LayoutStore;
  /** chamado quando o controle muda o rascunho */
  onChange?: () => void;
}

/**
 * Controle deslizante da opacidade dos botões (só em modo edição). `sync` o relê do store,
 * para Restaurar, Cancelar e uma nova edição mostrarem o valor certo.
 */
export function mountOpacitySlider({ input, output, store, onChange }: Options): { sync: () => void } {
  const sync = () => {
    const percent = Math.round(opacityOf(store.current) * 100);
    input.value = String(percent);
    if (output) output.textContent = `${percent}%`; // textContent: só o número
  };

  input.addEventListener("input", () => {
    if (store.editing) {
      store.setOpacity(Number(input.value) / 100);
      onChange?.();
    }
    sync(); // fora da edição o controle volta ao valor salvo
  });

  sync();
  return { sync };
}
