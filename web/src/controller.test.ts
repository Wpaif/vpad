import { beforeEach, describe, expect, it } from "vitest";
import { bindController } from "./controller";
import { Button, encodeButton } from "./protocol";

let sent: number[][];
const send = (m: Uint8Array) => sent.push([...m]);

function ptr(el: Element, type: string, id: number, x = 0, y = 0) {
  el.dispatchEvent(
    new PointerEvent(type, { pointerId: id, clientX: x, clientY: y, bubbles: true }),
  );
}

beforeEach(() => {
  sent = [];
  document.body.innerHTML = `
    <div id="root">
      <button data-btn="A"></button>
      <button data-btn="B"></button>
      <div data-stick="left"><div class="knob"></div></div>
      <button data-trigger="LT"></button>
    </div>`;
  const stick = document.querySelector<HTMLElement>("[data-stick]")!;
  stick.getBoundingClientRect = () =>
    ({ left: 0, top: 0, width: 200, height: 200, right: 200, bottom: 200 }) as DOMRect;
  bindController(document.getElementById("root")!, send);
});

const A = () => document.querySelector("[data-btn=A]")!;
const B = () => document.querySelector("[data-btn=B]")!;
const stick = () => document.querySelector("[data-stick]")!;

describe("botões", () => {
  it("envia press e release", () => {
    ptr(A(), "pointerdown", 1);
    ptr(A(), "pointerup", 1);
    expect(sent).toEqual([[1, 0, 1], [1, 0, 0]]);
  });

  it("pointercancel também solta o botão", () => {
    ptr(A(), "pointerdown", 1);
    ptr(A(), "pointercancel", 1);
    expect(sent.at(-1)).toEqual([1, 0, 0]);
  });

  it("multi-toque: dois botões ao mesmo tempo", () => {
    ptr(A(), "pointerdown", 1);
    ptr(B(), "pointerdown", 2);
    ptr(A(), "pointerup", 1);
    expect(sent).toEqual([[1, 0, 1], [1, 1, 1], [1, 0, 0]]);
  });

  it("marca o estado pressionado para o CSS", () => {
    ptr(A(), "pointerdown", 1);
    expect(A().hasAttribute("data-pressed")).toBe(true);
    ptr(A(), "pointerup", 1);
    expect(A().hasAttribute("data-pressed")).toBe(false);
  });

  it("não repete press para o mesmo ponteiro", () => {
    ptr(A(), "pointerdown", 1);
    ptr(A(), "pointerdown", 1);
    expect(sent.length).toBe(1);
  });
});

describe("analógico", () => {
  it("empurrar à direita envia LX positivo e posiciona o botão visual", () => {
    ptr(stick(), "pointerdown", 1, 100, 100);
    ptr(stick(), "pointermove", 1, 190, 100);
    const lx = sent.filter((m) => m[0] === 2 && m[1] === 0).at(-1)!;
    expect(new DataView(Uint8Array.from(lx).buffer).getInt16(2, true)).toBeGreaterThan(20000);
    const knob = document.querySelector<HTMLElement>(".knob")!;
    expect(knob.style.transform).toContain("translate");
  });

  it("soltar volta os dois eixos a zero", () => {
    ptr(stick(), "pointerdown", 1, 100, 100);
    ptr(stick(), "pointermove", 1, 190, 150);
    ptr(stick(), "pointerup", 1, 190, 150);
    expect(sent.slice(-2)).toEqual([[2, 0, 0, 0], [2, 1, 0, 0]]);
  });

  it("ignora ponteiros que não são o dono do analógico", () => {
    ptr(stick(), "pointerdown", 1, 100, 100);
    const n = sent.length;
    ptr(stick(), "pointermove", 2, 190, 100);
    expect(sent.length).toBe(n);
  });
});

describe("gatilhos", () => {
  it("gatilho digital envia 255 e depois 0", () => {
    const lt = document.querySelector("[data-trigger=LT]")!;
    ptr(lt, "pointerdown", 1);
    ptr(lt, "pointerup", 1);
    expect(sent).toEqual([[2, 4, 255, 0], [2, 4, 0, 0]]);
  });
});

describe("controle oculto", () => {
  it("botão, gatilho e analógico ocultos ignoram o toque", () => {
    const todos = ["[data-btn=A]", "[data-trigger=LT]", "[data-stick]"].map((q) => document.querySelector(q)!);
    todos.forEach((el) => el.classList.add("oculto"));
    todos.forEach((el, i) => {
      ptr(el, "pointerdown", i + 1, 100, 10);
      ptr(el, "pointerup", i + 1, 100, 10);
    });
    expect(sent).toEqual([]);
  });
});

describe("modo edição do layout", () => {
  it("com data-editando no pad, botão, gatilho e analógico não enviam nada", () => {
    document.getElementById("root")!.setAttribute("data-editando", "");
    for (const [i, q] of ["[data-btn=A]", "[data-trigger=LT]", "[data-stick]"].entries()) {
      const el = document.querySelector(q)!;
      ptr(el, "pointerdown", i + 1, 100, 10);
      ptr(el, "pointerup", i + 1, 100, 10);
    }
    expect(sent).toEqual([]);
  });
});

describe("analógico em grupo escalado", () => {
  it("o botão interno se move o mesmo tanto que o dedo, mesmo com o grupo ampliado", () => {
    const el = stick() as HTMLElement;
    // caixa de 200 px na tela, mas 100 px no layout: o grupo está em escala 2
    Object.defineProperty(el, "offsetWidth", { value: 100, configurable: true });
    ptr(el, "pointerdown", 1, 150, 100); // 50 px à direita do centro (100,100)
    const knob = el.querySelector<HTMLElement>(".knob")!;
    // 50 px de tela viram 25 px dentro do grupo escalado
    expect(knob.style.transform).toBe("translate(25px, 0px)");
  });
});

describe("direcional", () => {
  beforeEach(() => {
    sent = [];
    document.body.innerHTML = `
      <div id="root"><div data-grupo="direcional">
        <button data-btn="Up"></button><button data-btn="Right"></button>
      </div></div>`;
    const rect = (l: number) => ({ left: l, right: l + 50, top: 0, bottom: 50 }) as DOMRect;
    document.querySelector("[data-btn=Up]")!.getBoundingClientRect = () => rect(0);
    document.querySelector("[data-btn=Right]")!.getBoundingClientRect = () => rect(50);
    bindController(document.getElementById("root")!, send);
  });
  const group = () => document.querySelector("[data-grupo=direcional]")!;

  it("deslizar troca a seta sem levantar o dedo", () => {
    ptr(group(), "pointerdown", 1, 10, 10);
    ptr(group(), "pointermove", 1, 70, 10);
    ptr(group(), "pointerup", 1, 70, 10);
    expect(sent).toEqual([
      [...encodeButton(Button.Up, true)],
      [...encodeButton(Button.Up, false)],
      [...encodeButton(Button.Right, true)],
      [...encodeButton(Button.Right, false)],
    ]);
  });
});
