import {
  defaultLayout,
  resetGroup,
  setHidden,
  withControl,
  withGroup,
  withOpacity,
  type ControlAdjust,
  type ControlId,
  type GroupAdjust,
  type GroupId,
  type Layout,
} from "./layoutModel";
import { loadLayout, saveLayout } from "./layoutStorage";

/** Layout salvo mais o rascunho de edição; `current` é o que a tela mostra. */
export function createLayoutStore() {
  let saved = $state<Layout>(loadLayout());
  let draft = $state<Layout | null>(null);

  const change = (fn: (l: Layout) => Layout) => {
    if (draft) draft = fn(draft);
  };

  return {
    get current(): Layout {
      return draft ?? saved;
    },
    get editing(): boolean {
      return draft !== null;
    },
    /** O layout salvo, sem o rascunho (é o que vai para o PC). */
    get saved(): Layout {
      return $state.snapshot(saved) as Layout;
    },
    /** Troca o layout salvo (vindo do PC), guardando a cópia local. Não mexe numa edição em andamento. */
    adopt(next: Layout): boolean {
      if (draft) return false;
      saveLayout(next);
      saved = next;
      return true;
    },
    edit() {
      draft = $state.snapshot(saved) as Layout;
    },
    move(group: GroupId, patch: Partial<GroupAdjust>) {
      change((l) => withGroup(l, group, patch));
    },
    /** Move um controle sozinho (o grupo dele está destravado na tela). */
    moveControl(control: ControlId, patch: Partial<ControlAdjust>) {
      change((l) => withControl(l, control, patch));
    },
    /** Opacidade de todos os botões (0,2 a 1). */
    setOpacity(opacity: number) {
      change((l) => withOpacity(l, opacity));
    },
    resetGroup(group: GroupId) {
      change((l) => resetGroup(l, group));
    },
    hide(control: ControlId, hidden: boolean) {
      change((l) => setHidden(l, control, hidden));
    },
    restoreDefault() {
      change(() => defaultLayout());
    },
    /** Devolve se gravou; se o armazenamento falhar, o rascunho continua aberto. */
    save(): boolean {
      if (!draft) return true;
      const next = $state.snapshot(draft) as Layout;
      if (!saveLayout(next)) return false;
      saved = next;
      draft = null;
      return true;
    },
    cancel() {
      draft = null;
    },
  };
}

export type LayoutStore = ReturnType<typeof createLayoutStore>;
