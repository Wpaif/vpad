import { defaultLayout, isDefault, parseLayout, serializeLayout, type Layout } from "./layoutModel";

export const LAYOUT_KEY = "vpad:layout";

/** Lê o layout salvo; qualquer falha (sem armazenamento, JSON ruim, versão futura) dá o padrão. */
export function loadLayout(): Layout {
  try {
    const raw = localStorage.getItem(LAYOUT_KEY);
    if (raw === null) return defaultLayout();
    return parseLayout(JSON.parse(raw)).layout;
  } catch {
    return defaultLayout();
  }
}

/** Grava o layout (o padrão apaga a chave). Devolve se deu certo. */
export function saveLayout(layout: Layout): boolean {
  try {
    if (isDefault(layout)) localStorage.removeItem(LAYOUT_KEY);
    else localStorage.setItem(LAYOUT_KEY, serializeLayout(layout));
    return true;
  } catch {
    return false;
  }
}
