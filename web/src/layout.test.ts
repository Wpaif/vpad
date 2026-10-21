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

describe("tela de acesso recusado no HTML", () => {
  it("existe, começa escondida, fora do controle e tem o botão de tentar de novo", () => {
    const gate = document.getElementById("gate")!;
    expect(gate.hidden).toBe(true);
    expect(gate.getAttribute("role")).toBe("alertdialog");
    expect(document.getElementById("pad")!.contains(gate)).toBe(false);
    expect(gate.querySelector("[data-role=message]")).not.toBeNull();
    expect(document.getElementById("gate-retry")).not.toBeNull();
  });
});

describe("dica de instalação", () => {
  it("manda abrir pelo link do QR code, para o atalho levar o código de acesso", () => {
    expect(document.getElementById("install-hint")!.textContent).toMatch(/QR/);
  });
});
