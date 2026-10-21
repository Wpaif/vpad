import { mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import Pad from "./Pad.svelte";

let app: ReturnType<typeof mount>;
let pad: HTMLElement;

beforeEach(() => {
  document.body.innerHTML = `<div id="app"></div>`;
  app = mount(Pad, { target: document.getElementById("app")! });
  pad = document.getElementById("pad")!;
});
afterEach(() => unmount(app));

const grupo = (id: string) => pad.querySelector<HTMLElement>(`[data-grupo="${id}"]`)!;
/** Quantos elementos que casam com `seletor` há no grupo (o próprio grupo conta, como no analógico). */
const contar = (g: HTMLElement, seletor: string) => (g.matches(seletor) ? 1 : 0) + g.querySelectorAll(seletor).length;

const GRUPOS = {
  "ombros-esq": ["[data-trigger=LT]", "[data-btn=LB]"],
  "ombros-dir": ["[data-btn=RB]", "[data-trigger=RT]"],
  "analogico-esq": ["[data-stick=left]"],
  "analogico-dir": ["[data-stick=right]"],
  direcional: ["[data-btn=Up]", "[data-btn=Down]", "[data-btn=Left]", "[data-btn=Right]"],
  face: ["[data-btn=A]", "[data-btn=B]", "[data-btn=X]", "[data-btn=Y]"],
  menu: ["[data-btn=Select]", "[data-btn=Start]"],
};

describe("Pad", () => {
  it("renderiza o controle com o id 'pad' e as três zonas de sempre", () => {
    expect(pad).not.toBeNull();
    expect(pad.getAttribute("aria-label")).toBe("Controle virtual");
    expect(pad.querySelectorAll(".zone").length).toBe(3);
    expect(pad.querySelector(".zone.left")).not.toBeNull();
    expect(pad.querySelector(".zone.center")).not.toBeNull();
    expect(pad.querySelector(".zone.right")).not.toBeNull();
  });

  it("tem sete grupos, cada um uma única vez", () => {
    for (const id of Object.keys(GRUPOS)) {
      expect(pad.querySelectorAll(`[data-grupo="${id}"]`).length, id).toBe(1);
    }
    expect(pad.querySelectorAll("[data-grupo]").length).toBe(7);
  });

  it("cada controle fica dentro do grupo certo", () => {
    for (const [id, controles] of Object.entries(GRUPOS)) {
      for (const seletor of controles) {
        expect(contar(grupo(id), seletor), `${seletor} em ${id}`).toBe(1);
      }
    }
  });

  it("nenhum controle fica fora de um grupo", () => {
    const todos = pad.querySelectorAll("[data-btn],[data-trigger],[data-stick]");
    expect(todos.length).toBe(Object.values(GRUPOS).flat().length);
    for (const el of todos) expect(el.closest("[data-grupo]"), el.outerHTML.slice(0, 50)).not.toBeNull();
  });

  it("mantém os ids do protocolo nos botões", () => {
    const ids = [...pad.querySelectorAll<HTMLElement>("[data-btn]")].map((e) => e.dataset.btn).sort();
    expect(ids).toEqual(["A", "B", "Down", "LB", "Left", "RB", "Right", "Select", "Start", "Up", "X", "Y"]);
    const gatilhos = [...pad.querySelectorAll<HTMLElement>("[data-trigger]")].map((e) => e.dataset.trigger).sort();
    expect(gatilhos).toEqual(["LT", "RT"]);
  });

  it("os botões de face mantêm a posição de cada um", () => {
    expect(grupo("face").querySelector(".top")!.getAttribute("data-btn")).toBe("Y");
    expect(grupo("face").querySelector(".left")!.getAttribute("data-btn")).toBe("X");
    expect(grupo("face").querySelector(".right")!.getAttribute("data-btn")).toBe("B");
    expect(grupo("face").querySelector(".bottom")!.getAttribute("data-btn")).toBe("A");
  });

  it("cada analógico tem o botão visual que se mexe", () => {
    for (const lado of ["left", "right"]) {
      expect(pad.querySelector(`[data-stick=${lado}] .knob`)).not.toBeNull();
    }
  });

  it("as setas do direcional têm nome acessível", () => {
    const nomes = ["Up", "Down", "Left", "Right"].map((d) => pad.querySelector(`[data-btn=${d}]`)!.getAttribute("aria-label"));
    expect(nomes).toEqual(["Cima", "Baixo", "Esquerda", "Direita"]);
  });

  it("o texto padrão dos botões é o do Xbox (os rótulos por perfil são aplicados depois)", () => {
    const texto = (s: string) => pad.querySelector(s)!.textContent!.trim();
    expect(texto("[data-btn=A]")).toBe("A");
    expect(texto("[data-trigger=LT]")).toBe("LT");
    expect(texto("[data-btn=Select]")).toBe("Select");
    expect(texto("[data-btn=Start]")).toBe("Start");
  });
});
