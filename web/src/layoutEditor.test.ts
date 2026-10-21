import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { adjustOf } from "./layoutModel";
import { LAYOUT_KEY } from "./layoutStorage";
import { createLayoutStore, type LayoutStore } from "./layoutStore.svelte";
import { mountLayoutEditor } from "./layoutEditor";
import type { Rect } from "./layoutGeometry";

const rect = (left: number, top: number, w: number, h: number): Rect => ({ left, top, right: left + w, bottom: top + h });
const BOUNDS = rect(0, 0, 800, 400);

let store: LayoutStore;
let pad: HTMLElement;
let stop: () => void;
let guides: Array<{ x: number | null; y: number | null }>;

const grupo = (id: string) => pad.querySelector<HTMLElement>(`[data-grupo=${id}]`)!;
function ptr(el: Element, type: string, id: number, x: number, y: number) {
  el.dispatchEvent(new PointerEvent(type, { pointerId: id, clientX: x, clientY: y, bubbles: true }));
}
const faceAdjust = () => adjustOf(store.current, "face");

beforeEach(() => {
  localStorage.clear();
  document.body.innerHTML = `
    <main id="pad">
      <div data-grupo="face"><button data-btn="A"></button></div>
      <div data-grupo="menu"></div>
    </main>`;
  pad = document.getElementById("pad")!;
  const rects: Record<string, Rect> = { face: rect(100, 100, 100, 100), menu: rect(300, 100, 100, 50) };
  for (const [id, r] of Object.entries(rects)) {
    grupo(id).getBoundingClientRect = () => ({ ...r, width: r.right - r.left, height: r.bottom - r.top }) as DOMRect;
  }
  store = createLayoutStore();
  guides = [];
  stop = mountLayoutEditor({
    pad,
    store,
    appSize: () => ({ w: 800, h: 400 }),
    bounds: () => BOUNDS,
    onGuides: (x, y) => guides.push({ x, y }),
  });
});
afterEach(() => stop());

describe("layoutEditor", () => {
  it("fora do modo edição o arrastar não faz nada", () => {
    ptr(grupo("face"), "pointerdown", 1, 150, 150);
    ptr(grupo("face"), "pointermove", 1, 250, 190);
    expect(faceAdjust()).toEqual({ dx: 0, dy: 0, scale: 1 });
  });

  it("arrastar um grupo move o rascunho, em fração da área útil", () => {
    store.edit();
    ptr(grupo("face"), "pointerdown", 1, 150, 150);
    ptr(grupo("face"), "pointermove", 1, 200, 190); // +50, +40
    expect(faceAdjust().dx).toBeCloseTo(0.0625, 4);
    expect(faceAdjust().dy).toBeCloseTo(0.1, 4);
  });

  it("tocar num botão dentro do grupo também arrasta o grupo", () => {
    store.edit();
    const botao = grupo("face").querySelector("button")!;
    ptr(botao, "pointerdown", 1, 150, 150);
    ptr(botao, "pointermove", 1, 200, 190);
    expect(faceAdjust().dx).toBeCloseTo(0.0625, 4);
  });

  it("não deixa o grupo sair da tela", () => {
    store.edit();
    ptr(grupo("face"), "pointerdown", 1, 150, 150);
    ptr(grupo("face"), "pointermove", 1, -350, 150); // pediria -500
    expect(faceAdjust().dx).toBeCloseTo(-100 / 800, 4);
  });

  it("gruda na borda de outro grupo e avisa a guia", () => {
    store.edit();
    ptr(grupo("face"), "pointerdown", 1, 150, 150);
    ptr(grupo("face"), "pointermove", 1, 247, 190); // direita do face em 297 (menu começa em 300); y +40 fica longe de qualquer alinhamento
    expect(faceAdjust().dx).toBeCloseTo(100 / 800, 4);
    expect(guides.at(-1)).toEqual({ x: 300, y: null });
  });

  it("ignora um segundo dedo enquanto um grupo está sendo arrastado", () => {
    store.edit();
    ptr(grupo("face"), "pointerdown", 1, 150, 150);
    ptr(grupo("menu"), "pointerdown", 2, 350, 120);
    ptr(grupo("menu"), "pointermove", 2, 380, 140);
    expect(adjustOf(store.current, "menu")).toEqual({ dx: 0, dy: 0, scale: 1 });
  });

  it("soltar termina o arrasto, apaga as guias e não salva", () => {
    store.edit();
    ptr(grupo("face"), "pointerdown", 1, 150, 150);
    ptr(grupo("face"), "pointermove", 1, 200, 190);
    ptr(grupo("face"), "pointerup", 1, 200, 190);
    const depois = faceAdjust().dx;
    ptr(grupo("face"), "pointermove", 1, 300, 300);
    expect(faceAdjust().dx).toBe(depois);
    expect(guides.at(-1)).toEqual({ x: null, y: null });
    expect(store.editing).toBe(true);
    expect(localStorage.getItem(LAYOUT_KEY)).toBeNull();
  });

  it("depois de parar (stop) não responde mais", () => {
    store.edit();
    stop();
    ptr(grupo("face"), "pointerdown", 1, 150, 150);
    ptr(grupo("face"), "pointermove", 1, 200, 190);
    expect(faceAdjust()).toEqual({ dx: 0, dy: 0, scale: 1 });
    stop = vi.fn();
  });
});
