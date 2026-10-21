import { readFileSync } from "node:fs";
import { beforeEach, describe, expect, it } from "vitest";
import { applyKeyboardLabels, applyProfileLabels } from "./labels";

let pad: HTMLElement;

beforeEach(() => {
  document.documentElement.innerHTML = readFileSync("index.html", "utf8");
  pad = document.getElementById("pad")!;
});

const el = (sel: string) => pad.querySelector<HTMLElement>(sel)!;
const text = (sel: string) => el(sel).textContent;
const glyph = (sel: string) => el(sel).querySelector("svg")?.dataset.glyph ?? null;
const aria = (sel: string) => el(sel).getAttribute("aria-label");

describe("Xbox 360", () => {
  beforeEach(() => applyProfileLabels(pad, "xbox360"));

  it("usa A/B/X/Y e os nomes de ombro e menu do Xbox", () => {
    expect(["A", "B", "X", "Y"].map((b) => text(`[data-btn=${b}]`))).toEqual(["A", "B", "X", "Y"]);
    expect(text("[data-btn=LB]")).toBe("LB");
    expect(text("[data-btn=RB]")).toBe("RB");
    expect(text("[data-trigger=LT]")).toBe("LT");
    expect(text("[data-trigger=RT]")).toBe("RT");
    expect(text("[data-btn=Select]")).toBe("Back");
    expect(text("[data-btn=Start]")).toBe("Start");
  });
});

describe("DualShock 4", () => {
  beforeEach(() => applyProfileLabels(pad, "ds4"));

  it("troca os botões de face pelos símbolos, na mesma posição", () => {
    expect(glyph("[data-btn=A]")).toBe("cross"); // embaixo
    expect(glyph("[data-btn=B]")).toBe("circle"); // à direita
    expect(glyph("[data-btn=X]")).toBe("square"); // à esquerda
    expect(glyph("[data-btn=Y]")).toBe("triangle"); // em cima
  });

  it("os símbolos têm nome acessível em português", () => {
    expect(aria("[data-btn=A]")).toBe("Cruz");
    expect(aria("[data-btn=B]")).toBe("Círculo");
    expect(aria("[data-btn=X]")).toBe("Quadrado");
    expect(aria("[data-btn=Y]")).toBe("Triângulo");
  });

  it("ombros L1/R1, gatilhos L2/R2, Share e Options", () => {
    expect(text("[data-btn=LB]")).toBe("L1");
    expect(text("[data-btn=RB]")).toBe("R1");
    expect(text("[data-trigger=LT]")).toBe("L2");
    expect(text("[data-trigger=RT]")).toBe("R2");
    expect(text("[data-btn=Select]")).toBe("Share");
    expect(text("[data-btn=Start]")).toBe("Options");
  });
});

describe("Switch Pro", () => {
  beforeEach(() => applyProfileLabels(pad, "switch-pro"));

  it("segue a disposição da Nintendo: B embaixo, A à direita, Y à esquerda, X em cima", () => {
    expect(text("[data-btn=A]")).toBe("B"); // posição de baixo
    expect(text("[data-btn=B]")).toBe("A"); // posição da direita
    expect(text("[data-btn=X]")).toBe("Y"); // posição da esquerda
    expect(text("[data-btn=Y]")).toBe("X"); // posição de cima
  });

  it("ombros L/R, gatilhos ZL/ZR, menos e mais", () => {
    expect(text("[data-btn=LB]")).toBe("L");
    expect(text("[data-btn=RB]")).toBe("R");
    expect(text("[data-trigger=LT]")).toBe("ZL");
    expect(text("[data-trigger=RT]")).toBe("ZR");
    expect(text("[data-btn=Select]")).toBe("−");
    expect(text("[data-btn=Start]")).toBe("+");
    expect(aria("[data-btn=Select]")).toBe("Menos");
    expect(aria("[data-btn=Start]")).toBe("Mais");
  });
});

describe("símbolos de um caractere", () => {
  it("− e + ganham data-symbol para o CSS aumentá-los", () => {
    applyProfileLabels(pad, "switch-pro");
    expect(el("[data-btn=Select]").hasAttribute("data-symbol")).toBe(true);
    expect(el("[data-btn=Start]").hasAttribute("data-symbol")).toBe(true);
  });

  it("rótulos de palavra não ganham", () => {
    applyProfileLabels(pad, "xbox360");
    expect(el("[data-btn=Select]").hasAttribute("data-symbol")).toBe(false);
    applyProfileLabels(pad, "switch-pro");
    applyProfileLabels(pad, "ds4");
    expect(el("[data-btn=Select]").hasAttribute("data-symbol")).toBe(false);
  });
});

describe("aplicar rótulos", () => {
  it("marca o perfil no controle para o CSS escolher as cores", () => {
    applyProfileLabels(pad, "ds4");
    expect(pad.dataset.profile).toBe("ds4");
  });

  it("nunca mexe nos ids do protocolo nem no direcional", () => {
    const antes = [...pad.querySelectorAll("[data-btn],[data-trigger]")].map(
      (e) => (e as HTMLElement).dataset.btn ?? (e as HTMLElement).dataset.trigger,
    );
    const dpad = el(".dpad").innerHTML;
    applyProfileLabels(pad, "switch-pro");
    const depois = [...pad.querySelectorAll("[data-btn],[data-trigger]")].map(
      (e) => (e as HTMLElement).dataset.btn ?? (e as HTMLElement).dataset.trigger,
    );
    expect(depois).toEqual(antes);
    expect(el(".dpad").innerHTML).toBe(dpad);
  });

  it("voltar a outro perfil limpa o símbolo anterior", () => {
    applyProfileLabels(pad, "ds4");
    applyProfileLabels(pad, "xbox360");
    expect(el("[data-btn=A]").querySelector("svg")).toBeNull();
    expect(text("[data-btn=A]")).toBe("A");
    expect(el("[data-btn=A]").hasAttribute("aria-label")).toBe(false);
  });

  it("perfil desconhecido não altera nada e devolve false", () => {
    applyProfileLabels(pad, "xbox360");
    expect(applyProfileLabels(pad, "wii")).toBe(false);
    expect(pad.dataset.profile).toBe("xbox360");
    expect(text("[data-btn=A]")).toBe("A");
  });

  it("devolve true quando aplica", () => {
    expect(applyProfileLabels(pad, "ds4")).toBe(true);
  });
});

describe("posição dos botões de face no HTML", () => {
  it("o botão de cada posição continua ligado ao id certo do protocolo", () => {
    expect(el(".face .top").dataset.btn).toBe("Y");
    expect(el(".face .left").dataset.btn).toBe("X");
    expect(el(".face .right").dataset.btn).toBe("B");
    expect(el(".face .bottom").dataset.btn).toBe("A");
  });
});

describe("modo teclado", () => {
  const teclas = [
    { id: "KeyK", rotulo: "K", grupo: "Letras" },
    { id: "Space", rotulo: "Espaço", grupo: "Especiais" },
    { id: "Tab", rotulo: "Tab", grupo: "Especiais" },
    { id: "ArrowUp", rotulo: "↑", grupo: "Setas" },
  ];
  const mapa = { A: "KeyK", RB: "Space", Select: "Tab", RT: "ArrowUp" };

  beforeEach(() => applyKeyboardLabels(pad, mapa, teclas));

  it("mostra em cada botão a tecla que ele aciona", () => {
    expect(text("[data-btn=A]")).toBe("K");
    expect(text("[data-btn=RB]")).toBe("Espaço");
    expect(text("[data-btn=Select]")).toBe("Tab");
    expect(text("[data-trigger=RT]")).toBe("↑");
  });

  it("controle sem tecla mostra um traço", () => {
    expect(text("[data-btn=B]")).toBe("–");
    expect(text("[data-btn=Start]")).toBe("–");
  });

  it("tecla fora do catálogo mostra o próprio id", () => {
    applyKeyboardLabels(pad, { A: "Desconhecida" }, teclas);
    expect(text("[data-btn=A]")).toBe("Desconhecida");
  });

  it("marca o perfil e os rótulos de tecla para o CSS", () => {
    expect(pad.dataset.profile).toBe("teclado");
    expect(el("[data-btn=A]").hasAttribute("data-keycap")).toBe(true);
  });

  it("o nome acessível diz a tecla", () => {
    expect(aria("[data-btn=RB]")).toBe("Tecla Espaço");
    expect(aria("[data-btn=B]")).toBe("Sem tecla");
  });

  it("voltar a um perfil de controle limpa as marcas de tecla", () => {
    applyProfileLabels(pad, "xbox360");
    expect(el("[data-btn=A]").hasAttribute("data-keycap")).toBe(false);
    expect(text("[data-btn=A]")).toBe("A");
  });

  it("não mexe nos ids do protocolo nem no direcional", () => {
    const dpad = el(".dpad").innerHTML;
    applyKeyboardLabels(pad, { Up: "KeyK" }, teclas);
    expect(el(".dpad").innerHTML).toBe(dpad);
    expect(el(".face .top").dataset.btn).toBe("Y");
  });
});
