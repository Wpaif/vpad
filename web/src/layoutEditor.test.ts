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
let editor: { stop: () => void; refresh: () => Promise<void> };
const stop = () => editor.stop();
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
  editor = mountLayoutEditor({
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
    editor = { stop: vi.fn(), refresh: async () => {} };
  });
});

describe("alças de redimensionar", () => {
  const alca = (id: string) => document.querySelector<HTMLElement>(`[data-alca=${id}]`);

  beforeEach(() => {
    // o botão A mede 60 px: o piso de toque (44 px) dá escala mínima 44/60
    grupo("face").querySelector("button")!.getBoundingClientRect = () => ({ ...rect(120, 120, 60, 60), width: 60, height: 60 }) as DOMRect;
  });

  it("não há alças fora do modo edição", async () => {
    await editor.refresh();
    expect(alca("face")).toBeNull();
  });

  it("no modo edição há uma alça por grupo, no canto de baixo à direita", async () => {
    store.edit();
    await editor.refresh();
    expect(document.querySelectorAll("[data-alca]").length).toBe(2);
    expect(alca("face")!.style.left).toBe("200px");
    expect(alca("face")!.style.top).toBe("200px");
  });

  it("a alça de um grupo colado na borda fica inteira dentro da tela", async () => {
    grupo("face").getBoundingClientRect = () => ({ ...rect(700, 300, 100, 100), width: 100, height: 100 }) as DOMRect;
    store.edit();
    await editor.refresh();
    expect(alca("face")!.style.left).toBe("786px"); // 800 - meia alça (14)
    expect(alca("face")!.style.top).toBe("386px"); // 400 - 14
  });

  it("as alças somem ao sair da edição", async () => {
    store.edit();
    await editor.refresh();
    store.cancel();
    await editor.refresh();
    expect(alca("face")).toBeNull();
  });

  it("arrastar a alça para longe do centro amplia o grupo", async () => {
    store.edit();
    await editor.refresh();
    ptr(alca("face")!, "pointerdown", 1, 200, 200); // centro (150,150): distância 70,7
    ptr(alca("face")!, "pointermove", 1, 220.71, 220.71); // distância ~100 → razão ~1,41
    expect(faceAdjust().scale).toBeCloseTo(1.414, 2);
  });

  it("o máximo é 2", async () => {
    store.edit();
    await editor.refresh();
    ptr(alca("face")!, "pointerdown", 1, 200, 200);
    ptr(alca("face")!, "pointermove", 1, 600, 600);
    expect(faceAdjust().scale).toBe(2);
  });

  it("encolher para quando o menor controle chega a 44 px", async () => {
    store.edit();
    await editor.refresh();
    ptr(alca("face")!, "pointerdown", 1, 200, 200);
    ptr(alca("face")!, "pointermove", 1, 151, 151); // pediria ~0,02
    expect(faceAdjust().scale).toBeCloseTo(44 / 60, 3);
  });

  it("redimensionar não mexe na posição", async () => {
    store.edit();
    await editor.refresh();
    ptr(alca("face")!, "pointerdown", 1, 200, 200);
    ptr(alca("face")!, "pointermove", 1, 230, 230);
    expect(faceAdjust().dx).toBe(0);
    expect(faceAdjust().dy).toBe(0);
  });

  it("soltar termina o redimensionamento", async () => {
    store.edit();
    await editor.refresh();
    ptr(alca("face")!, "pointerdown", 1, 200, 200);
    ptr(alca("face")!, "pointermove", 1, 230, 230);
    ptr(alca("face")!, "pointerup", 1, 230, 230);
    const depois = faceAdjust().scale;
    ptr(alca("face")!, "pointermove", 1, 400, 400);
    expect(faceAdjust().scale).toBe(depois);
  });

  it("depois de parar o editor, as alças são removidas", async () => {
    store.edit();
    await editor.refresh();
    editor.stop();
    expect(document.querySelectorAll("[data-alca]").length).toBe(0);
    editor = { stop: vi.fn(), refresh: async () => {} };
  });
});
