import { tick } from "svelte";
import { limitScale, limitShift, minScaleFor, scaleRatio, snapShift, type Rect } from "./layoutGeometry";
import { adjustOf, MAX_SCALE, type GroupId } from "./layoutModel";
import type { LayoutStore } from "./layoutStore.svelte";

/** Distância (px) em que o grupo gruda no alinhamento de outro. */
const SNAP_PX = 8;
/** Meia alça (px): a distância mínima entre o centro da alça e a borda da tela. */
const HANDLE_HALF = 14;

interface Options {
  pad: HTMLElement;
  store: LayoutStore;
  /** tamanho da área útil, base do deslocamento (o mesmo de --app-w/--app-h) */
  appSize: () => { w: number; h: number };
  /** limites em que um grupo precisa caber (a tela menos as áreas seguras) */
  bounds: () => Rect;
  /** linhas de alinhamento a desenhar; null apaga */
  onGuides?: (x: number | null, y: number | null) => void;
}

const toRect = (el: Element): Rect => {
  const r = el.getBoundingClientRect();
  return { left: r.left, top: r.top, right: r.right, bottom: r.bottom };
};

/**
 * Arrastar grupos no modo edição. Cada arrasto parte das posições do toque inicial e do
 * retângulo do grupo naquele instante, então não depende de reler o DOM durante o movimento.
 * Cada grupo também ganha uma alça (fora do pad, para não ser escalada junto) que o redimensiona
 * em torno do centro. `refresh` reposiciona as alças depois de mudanças do modo edição.
 */
export interface LayoutEditor {
  stop: () => void;
  refresh: () => Promise<void>;
}

export function mountLayoutEditor({ pad, store, appSize, bounds, onGuides }: Options): LayoutEditor {
  let drag: {
    pointerId: number;
    group: GroupId;
    startX: number;
    startY: number;
    start: { dx: number; dy: number };
    rect: Rect;
    targets: Rect[];
  } | null = null;

  let resize: {
    pointerId: number;
    group: GroupId;
    startX: number;
    startY: number;
    startScale: number;
    center: { x: number; y: number };
    rect: Rect;
    min: number;
  } | null = null;

  const down = (e: PointerEvent) => {
    if (!store.editing || drag || resize) return;
    const el = (e.target as Element | null)?.closest<HTMLElement>("[data-grupo]");
    if (!el || !pad.contains(el)) return;
    const group = el.dataset.grupo as GroupId;
    const { dx, dy } = adjustOf(store.current, group);
    drag = {
      pointerId: e.pointerId,
      group,
      startX: e.clientX,
      startY: e.clientY,
      start: { dx, dy },
      rect: toRect(el),
      targets: [...pad.querySelectorAll<HTMLElement>("[data-grupo]")].filter((o) => o !== el).map(toRect),
    };
    try {
      el.setPointerCapture?.(e.pointerId);
    } catch {
      // ponteiro já liberado
    }
    e.preventDefault();
  };

  const move = (e: PointerEvent) => {
    if (!drag || e.pointerId !== drag.pointerId) return;
    const raw = { x: e.clientX - drag.startX, y: e.clientY - drag.startY };
    const snapped = snapShift(drag.rect, raw, drag.targets, SNAP_PX);
    const shift = limitShift(drag.rect, snapped, bounds());
    const { w, h } = appSize();
    store.move(drag.group, { dx: drag.start.dx + shift.x / w, dy: drag.start.dy + shift.y / h });
    // a guia só vale se o limite não empurrou o grupo para longe do alinhamento
    onGuides?.(shift.x === snapped.x ? snapped.guideX : null, shift.y === snapped.y ? snapped.guideY : null);
    void refresh();
  };

  const end = (e: PointerEvent) => {
    if (!drag || e.pointerId !== drag.pointerId) return;
    drag = null;
    onGuides?.(null, null);
  };

  // alças de redimensionar: um elemento por grupo, num contêiner fora do pad
  const handles = new Map<GroupId, HTMLElement>();
  const layer = document.createElement("div");
  layer.className = "alcas";
  document.body.append(layer);

  const groupEl = (id: GroupId) => pad.querySelector<HTMLElement>(`[data-grupo="${id}"]`);

  const refresh = async () => {
    await tick(); // o DOM só reflete o rascunho depois que o Svelte aplica a mudança
    if (!store.editing) {
      for (const h of handles.values()) h.remove();
      handles.clear();
      return;
    }
    for (const g of pad.querySelectorAll<HTMLElement>("[data-grupo]")) {
      const id = g.dataset.grupo as GroupId;
      let h = handles.get(id);
      if (!h) {
        h = document.createElement("div");
        h.className = "alca";
        h.dataset.alca = id;
        h.setAttribute("aria-hidden", "true");
        layer.append(h);
        handles.set(id, h);
      }
      const r = toRect(g);
      const { w, h: ah } = appSize();
      // a alça fica inteira na tela, mesmo com o grupo colado na borda
      h.style.left = `${Math.min(r.right, w - HANDLE_HALF)}px`;
      h.style.top = `${Math.min(r.bottom, ah - HANDLE_HALF)}px`;
    }
  };

  const downHandle = (e: PointerEvent) => {
    if (!store.editing || drag || resize) return;
    const h = (e.target as Element | null)?.closest<HTMLElement>("[data-alca]");
    const el = h && groupEl(h.dataset.alca as GroupId);
    if (!h || !el) return;
    const group = h.dataset.alca as GroupId;
    const startScale = adjustOf(store.current, group).scale;
    const rect = toRect(el);
    const controls = [el, ...el.querySelectorAll<HTMLElement>("*")].filter((c) =>
      c.matches("[data-btn], [data-trigger], [data-stick]"),
    );
    const sizes = controls.map((c) => {
      const r = c.getBoundingClientRect();
      return Math.min(r.width, r.height) / startScale;
    });
    resize = {
      pointerId: e.pointerId,
      group,
      startX: e.clientX,
      startY: e.clientY,
      startScale,
      center: { x: (rect.left + rect.right) / 2, y: (rect.top + rect.bottom) / 2 },
      rect,
      min: minScaleFor(sizes.length ? Math.min(...sizes) : 0),
    };
    try {
      h.setPointerCapture?.(e.pointerId);
    } catch {
      // ponteiro já liberado
    }
    e.preventDefault();
  };

  const moveHandle = (e: PointerEvent) => {
    if (!resize || e.pointerId !== resize.pointerId) return;
    const ratio = scaleRatio(resize.center, { x: resize.startX, y: resize.startY }, { x: e.clientX, y: e.clientY });
    const scale = limitScale(resize.startScale * ratio, resize.rect, resize.startScale, bounds(), resize.min, MAX_SCALE);
    store.move(resize.group, { scale });
    void refresh();
  };

  const endHandle = (e: PointerEvent) => {
    if (!resize || e.pointerId !== resize.pointerId) return;
    resize = null;
  };

  const onWindowResize = () => void refresh();

  pad.addEventListener("pointerdown", down);
  pad.addEventListener("pointermove", move);
  pad.addEventListener("pointerup", end);
  pad.addEventListener("pointercancel", end);
  layer.addEventListener("pointerdown", downHandle);
  layer.addEventListener("pointermove", moveHandle);
  layer.addEventListener("pointerup", endHandle);
  layer.addEventListener("pointercancel", endHandle);
  window.addEventListener("resize", onWindowResize);
  return {
    refresh,
    stop() {
      pad.removeEventListener("pointerdown", down);
      pad.removeEventListener("pointermove", move);
      pad.removeEventListener("pointerup", end);
      pad.removeEventListener("pointercancel", end);
      window.removeEventListener("resize", onWindowResize);
      layer.remove();
      handles.clear();
      drag = null;
      resize = null;
    },
  };
}
