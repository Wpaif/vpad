import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { adjustOf, controlOffset } from "./layoutModel";
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
      <div data-grupo="face"><button data-btn="A"></button><button data-btn="B"></button></div>
      <div data-grupo="menu"><button data-btn="Select"></button><button data-btn="Start"></button></div>
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

describe("destravar grupo (mover botões soltos)", () => {
  const cadeado = (id: string) => document.querySelector<HTMLElement>(`[data-cadeado=${id}]`);
  const botao = (id: string) => pad.querySelector<HTMLElement>(`[data-btn=${id}]`)!;
  const setRect = (el: HTMLElement, r: Rect) => {
    el.getBoundingClientRect = () => ({ ...r, width: r.right - r.left, height: r.bottom - r.top }) as DOMRect;
  };

  beforeEach(() => {
    setRect(botao("A"), rect(110, 110, 60, 60));
    setRect(botao("B"), rect(600, 300, 60, 60));
    setRect(botao("Select"), rect(300, 100, 45, 50));
    setRect(botao("Start"), rect(355, 100, 45, 50));
  });

  const soltar = async (id: string) => {
    store.edit();
    await editor.refresh();
    cadeado(id)!.click();
  };

  it("fora do modo edição não há cadeados", async () => {
    await editor.refresh();
    expect(document.querySelectorAll("[data-cadeado]").length).toBe(0);
  });

  it("em edição há um cadeado por grupo com mais de um controle, começando travado", async () => {
    store.edit();
    await editor.refresh();
    expect(cadeado("face")).not.toBeNull();
    expect(cadeado("menu")).not.toBeNull();
    expect(cadeado("face")!.getAttribute("aria-pressed")).toBe("false");
    expect(grupo("face").hasAttribute("data-solto")).toBe(false);
  });

  it("grupo de um controle só (analógico) não tem cadeado", async () => {
    pad.insertAdjacentHTML("beforeend", `<div data-grupo="analogico-esq" data-stick="left"></div>`);
    store.edit();
    await editor.refresh();
    expect(cadeado("analogico-esq")).toBeNull();
  });

  it("o cadeado fica no canto de cima à direita do grupo (a alça é a de baixo)", async () => {
    store.edit();
    await editor.refresh();
    expect(cadeado("face")!.style.left).toBe("200px");
    expect(cadeado("face")!.style.top).toBe("100px");
  });

  it("o cadeado de um grupo colado na borda fica inteiro dentro da tela", async () => {
    setRect(grupo("face"), rect(700, 0, 100, 100));
    store.edit();
    await editor.refresh();
    expect(cadeado("face")!.style.left).toBe("786px");
    expect(cadeado("face")!.style.top).toBe("14px");
  });

  it("clicar destrava (e de novo trava), marcando o grupo e o botão", async () => {
    await soltar("face");
    expect(cadeado("face")!.getAttribute("aria-pressed")).toBe("true");
    expect(grupo("face").hasAttribute("data-solto")).toBe(true);
    cadeado("face")!.click();
    expect(cadeado("face")!.getAttribute("aria-pressed")).toBe("false");
    expect(grupo("face").hasAttribute("data-solto")).toBe(false);
  });

  it("grupo destravado: arrastar um botão move só ele", async () => {
    await soltar("face");
    ptr(botao("A"), "pointerdown", 1, 140, 140);
    ptr(botao("A"), "pointermove", 1, 180, 170); // +40, +30
    expect(controlOffset(store.current, "A").dx).toBeCloseTo(40 / 800, 4);
    expect(controlOffset(store.current, "A").dy).toBeCloseTo(30 / 400, 4);
    expect(controlOffset(store.current, "B")).toEqual({ dx: 0, dy: 0 });
    expect(faceAdjust()).toEqual({ dx: 0, dy: 0, scale: 1 });
  });

  it("grupo destravado: arrastar o vão do grupo não move nada", async () => {
    await soltar("face");
    ptr(grupo("face"), "pointerdown", 1, 190, 190);
    ptr(grupo("face"), "pointermove", 1, 240, 230);
    expect(faceAdjust()).toEqual({ dx: 0, dy: 0, scale: 1 });
    expect(controlOffset(store.current, "A")).toEqual({ dx: 0, dy: 0 });
  });

  it("grupo travado continua arrastando o grupo inteiro", () => {
    store.edit();
    ptr(botao("A"), "pointerdown", 1, 140, 140);
    ptr(botao("A"), "pointermove", 1, 190, 180);
    expect(faceAdjust().dx).toBeGreaterThan(0);
    expect(controlOffset(store.current, "A")).toEqual({ dx: 0, dy: 0 });
  });

  it("o deslocamento do botão vale na escala do grupo (o grupo ampliado o amplia)", async () => {
    store.edit();
    store.move("face", { scale: 2 });
    await editor.refresh();
    cadeado("face")!.click();
    ptr(botao("A"), "pointerdown", 1, 140, 140);
    ptr(botao("A"), "pointermove", 1, 180, 140); // +40 px de tela = 20 px locais
    expect(controlOffset(store.current, "A").dx).toBeCloseTo(40 / (800 * 2), 4);
  });

  it("o botão não sai da tela", async () => {
    await soltar("face");
    ptr(botao("A"), "pointerdown", 1, 140, 140);
    ptr(botao("A"), "pointermove", 1, -360, 140); // pediria -500; só cabem -110
    expect(controlOffset(store.current, "A").dx).toBeCloseTo(-110 / 800, 4);
  });

  it("o botão gruda no alinhamento de outro botão", async () => {
    await soltar("face");
    // A (110..170) com +132 fica em 242..302 → a direita encosta no Select (esq. 300): grudar em 300 (+130)
    ptr(botao("A"), "pointerdown", 1, 140, 140);
    ptr(botao("A"), "pointermove", 1, 272, 300); // y +160 longe de tudo
    expect(controlOffset(store.current, "A").dx).toBeCloseTo(130 / 800, 4);
    expect(guides.at(-1)!.x).toBe(300);
  });

  it("sair da edição trava tudo de novo", async () => {
    await soltar("face");
    store.cancel();
    await editor.refresh();
    expect(grupo("face").hasAttribute("data-solto")).toBe(false);
    expect(document.querySelectorAll("[data-cadeado]").length).toBe(0);
    store.edit();
    await editor.refresh();
    expect(cadeado("face")!.getAttribute("aria-pressed")).toBe("false");
  });

  it("parar o editor remove os cadeados e a marca dos grupos", async () => {
    await soltar("face");
    editor.stop();
    expect(document.querySelectorAll("[data-cadeado]").length).toBe(0);
    expect(grupo("face").hasAttribute("data-solto")).toBe(false);
    editor = { stop: vi.fn(), refresh: async () => {} };
  });
});
