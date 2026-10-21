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
