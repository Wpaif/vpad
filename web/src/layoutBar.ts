import type { LayoutStore } from "./layoutStore.svelte";

interface Options {
  bar: HTMLElement;
  editButton: HTMLElement;
  store: LayoutStore;
  /** chamado depois de qualquer mudança de modo (abrir, salvar, cancelar), para a tela se atualizar */
  onChange?: () => void;
}

/** Barra do modo edição: o botão do painel abre, e a barra salva, cancela ou restaura o padrão. */
export function mountLayoutBar({ bar, editButton, store, onChange }: Options): void {
  const note = bar.querySelector<HTMLElement>("[data-note]");
  const say = (text: string) => {
    if (note) note.textContent = text; // textContent: nada vira HTML
  };
  const sync = () => {
    bar.hidden = !store.editing;
    onChange?.();
  };

  editButton.addEventListener("click", () => {
    say("");
    store.edit();
    sync();
  });
  bar.querySelector("[data-act=reset]")?.addEventListener("click", () => {
    store.restoreDefault();
    sync();
  });
  bar.querySelector("[data-act=cancel]")?.addEventListener("click", () => {
    say("");
    store.cancel();
    sync();
  });
  bar.querySelector("[data-act=save]")?.addEventListener("click", () => {
    if (store.save()) say("");
    else say("Não foi possível salvar neste navegador.");
    sync();
  });
}
