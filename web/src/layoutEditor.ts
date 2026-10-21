import { limitShift, snapShift, type Rect } from "./layoutGeometry";
import { adjustOf, type GroupId } from "./layoutModel";
import type { LayoutStore } from "./layoutStore.svelte";

/** Distância (px) em que o grupo gruda no alinhamento de outro. */
const SNAP_PX = 8;

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
 * Devolve a função que desfaz tudo.
 */
export function mountLayoutEditor({ pad, store, appSize, bounds, onGuides }: Options): () => void {
  let drag: {
    pointerId: number;
    group: GroupId;
    startX: number;
    startY: number;
    start: { dx: number; dy: number };
    rect: Rect;
    targets: Rect[];
  } | null = null;

  const down = (e: PointerEvent) => {
    if (!store.editing || drag) return;
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
  };

  const end = (e: PointerEvent) => {
    if (!drag || e.pointerId !== drag.pointerId) return;
    drag = null;
    onGuides?.(null, null);
  };

  pad.addEventListener("pointerdown", down);
  pad.addEventListener("pointermove", move);
  pad.addEventListener("pointerup", end);
  pad.addEventListener("pointercancel", end);
  return () => {
    pad.removeEventListener("pointerdown", down);
    pad.removeEventListener("pointermove", move);
    pad.removeEventListener("pointerup", end);
    pad.removeEventListener("pointercancel", end);
    drag = null;
  };
}
