import { readFileSync } from "node:fs";
import { beforeEach, describe, expect, it } from "vitest";

beforeEach(() => {
  document.documentElement.innerHTML = readFileSync("index.html", "utf8");
});

describe("barra superior", () => {
  it("indicador de latência e botão de ocultar ficam juntos na mesma barra", () => {
    const bar = document.querySelector(".topbar");
    expect(bar).not.toBeNull();
    expect(bar!.contains(document.getElementById("status"))).toBe(true);
    expect(bar!.contains(document.getElementById("toggle"))).toBe(true);
  });

  it("a barra fica fora do controle, para continuar visível quando ele é ocultado", () => {
    expect(document.getElementById("pad")!.contains(document.querySelector(".topbar"))).toBe(false);
  });
});

describe("seletor de perfil", () => {
  it("fica dentro do painel que só abre ao tocar no indicador, sem ocupar espaço fixo", () => {
    const picker = document.getElementById("profile-picker");
    expect(picker).not.toBeNull();
    expect(document.getElementById("stats")!.contains(picker)).toBe(true);
  });
});

describe("editor de teclas", () => {
  it("o botão para abri-lo fica dentro do painel, sem ocupar espaço fixo", () => {
    const button = document.getElementById("open-key-editor");
    expect(button).not.toBeNull();
    expect(document.getElementById("stats")!.contains(button)).toBe(true);
  });

  it("o editor começa escondido e fica fora do controle", () => {
    const editor = document.getElementById("key-editor")!;
    expect(editor.hidden).toBe(true);
    expect(editor.getAttribute("role")).toBe("dialog");
    expect(document.getElementById("pad")!.contains(editor)).toBe(false);
  });
});

describe("tela de pareamento no HTML", () => {
  it("existe, começa escondida e fica fora do controle", () => {
    const pair = document.getElementById("pair")!;
    expect(pair.hidden).toBe(true);
    expect(pair.getAttribute("role")).toBe("dialog");
    expect(document.getElementById("pad")!.contains(pair)).toBe(false);
  });

  it("tem um campo numérico de seis dígitos e o botão de parear", () => {
    const input = document.querySelector<HTMLInputElement>("#pair input")!;
    expect(input.getAttribute("inputmode")).toBe("numeric");
    expect(input.getAttribute("maxlength")).toBe("6");
    expect(input.getAttribute("autocomplete")).toBe("one-time-code");
    expect(document.querySelector("#pair button[type=submit]")).not.toBeNull();
    expect(document.querySelector("#pair [role=alert]")).not.toBeNull();
  });

  it("o antigo bloqueio de 'outro aparelho está controlando' não existe mais", () => {
    expect(document.getElementById("gate")).toBeNull();
  });
});

describe("dica de instalação", () => {
  it("avisa que o app instalado precisa ser pareado de novo, com o código do terminal", () => {
    expect(document.getElementById("install-hint")!.textContent).toMatch(/parear|código/i);
  });
});
