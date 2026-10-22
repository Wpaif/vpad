import { mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import Pad from "./Pad.svelte";
import { defaultLayout, setHidden, withControl, withGroup, type Layout } from "./layoutModel";

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

describe("Pad com layout", () => {
  const montar = (layout: Layout) => {
    unmount(app);
    document.body.innerHTML = `<div id="app"></div>`;
    app = mount(Pad, { target: document.getElementById("app")!, props: { layout } });
    pad = document.getElementById("pad")!;
  };

  it("sem layout nada recebe transform nem classe oculto", () => {
    expect(pad.querySelectorAll(".oculto").length).toBe(0);
    for (const g of pad.querySelectorAll<HTMLElement>("[data-grupo]")) expect(g.style.transform).toBe("");
  });

  it("controle oculto ganha a classe e os outros não", () => {
    montar(setHidden(defaultLayout(), "A", true));
    expect(pad.querySelector("[data-btn=A]")!.classList.contains("oculto")).toBe(true);
    expect(pad.querySelector("[data-btn=B]")!.classList.contains("oculto")).toBe(false);
  });

  it("analógico oculto marca o próprio elemento do analógico", () => {
    montar(setHidden(defaultLayout(), "StickLeft", true));
    expect(pad.querySelector("[data-stick=left]")!.classList.contains("oculto")).toBe(true);
  });

  it("grupo ajustado recebe translate e scale em função de --app-w/--app-h", () => {
    montar(withGroup(defaultLayout(), "face", { dx: 0.1, dy: -0.05, scale: 1.5 }));
    const t = pad.querySelector<HTMLElement>("[data-grupo=face]")!.style.transform;
    expect(t).toContain("translate(");
    expect(t).toContain("--app-w");
    expect(t).toContain("--app-h");
    expect(t).toContain("scale(1.5)");
  });
});

describe("Pad em edição", () => {
  it("só marca data-editando quando editing é verdadeiro", () => {
    expect(pad.hasAttribute("data-editando")).toBe(false);
    unmount(app);
    document.body.innerHTML = `<div id="app"></div>`;
    app = mount(Pad, { target: document.getElementById("app")!, props: { editing: true } });
    pad = document.getElementById("pad")!;
    expect(pad.hasAttribute("data-editando")).toBe(true);
  });
});

describe("Pad com controle solto", () => {
  const montar = (layout: Layout) => {
    unmount(app);
    document.body.innerHTML = `<div id="app"></div>`;
    app = mount(Pad, { target: document.getElementById("app")!, props: { layout } });
    pad = document.getElementById("pad")!;
  };
  const btn = (id: string) => pad.querySelector<HTMLElement>(`[data-btn=${id}]`)!;

  it("sem deslocamento individual nenhum controle recebe translate", () => {
    for (const b of pad.querySelectorAll<HTMLElement>("[data-btn], [data-trigger]")) {
      expect(b.style.getPropertyValue("translate")).toBe("");
    }
  });

  it("controle deslocado usa a propriedade translate, que não atrapalha o transform do toque", () => {
    montar(withControl(defaultLayout(), "A", { dx: 0.1, dy: -0.05 }));
    const t = btn("A").style.getPropertyValue("translate");
    expect(t).toContain("--app-w");
    expect(t).toContain("--app-h");
    expect(btn("A").style.transform).toBe("");
    expect(btn("B").style.getPropertyValue("translate")).toBe("");
  });

  it("vale também para gatilhos (LT/RT)", () => {
    montar(withControl(defaultLayout(), "LT", { dx: 0.02 }));
    expect(pad.querySelector<HTMLElement>("[data-trigger=LT]")!.style.getPropertyValue("translate")).toContain("--app-w");
  });
});
