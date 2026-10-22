import { beforeEach, describe, expect, it } from "vitest";
import { setLanguage } from "./i18n";
import { keyGroupName, keyLabel } from "./keyNames";
import { CATALOGS } from "./locales";

beforeEach(() => setLanguage("pt-BR"));

const GRUPOS = ["Letras", "Números", "Setas", "Especiais", "Funções", "Símbolos"];

describe("keyGroupName", () => {
  it("em português devolve o nome que o servidor mandou", () => {
    for (const g of GRUPOS) expect(keyGroupName(g)).toBe(g);
  });

  it.each(["en", "es", "zh-CN", "ja", "ko"] as const)("%s traduz os seis grupos do servidor", (idioma) => {
    setLanguage(idioma);
    const c = CATALOGS[idioma] as Record<string, string>;
    expect(GRUPOS.map(keyGroupName)).toEqual([
      c["keyGroup.letters"], c["keyGroup.numbers"], c["keyGroup.arrows"],
      c["keyGroup.special"], c["keyGroup.function"], c["keyGroup.symbols"],
    ]);
  });

  it("grupo que o servidor inventar no futuro aparece como veio", () => {
    setLanguage("ja");
    expect(keyGroupName("Mídia")).toBe("Mídia");
  });
});

describe("keyLabel", () => {
  it("só traduz o que é palavra (Espaço, Apagar); letras, setas e siglas ficam como vieram", () => {
    setLanguage("en");
    expect(keyLabel({ id: "Space", rotulo: "Espaço" })).toBe("Space");
    expect(keyLabel({ id: "Backspace", rotulo: "Apagar" })).toBe("Bksp");
    expect(keyLabel({ id: "KeyK", rotulo: "K" })).toBe("K");
    expect(keyLabel({ id: "ArrowUp", rotulo: "↑" })).toBe("↑");
    expect(keyLabel({ id: "ControlLeft", rotulo: "Ctrl" })).toBe("Ctrl");
    expect(keyLabel({ id: "Escape", rotulo: "Esc" })).toBe("Esc");
  });

  it("em português mantém o rótulo do servidor", () => {
    expect(keyLabel({ id: "Space", rotulo: "Espaço" })).toBe("Espaço");
    expect(keyLabel({ id: "Backspace", rotulo: "Apagar" })).toBe("Apagar");
  });

  it.each(["es", "zh-CN", "ja", "ko"] as const)("%s usa o catálogo", (idioma) => {
    setLanguage(idioma);
    const c = CATALOGS[idioma] as Record<string, string>;
    expect(keyLabel({ id: "Space", rotulo: "Espaço" })).toBe(c["key.space"]);
    expect(keyLabel({ id: "Backspace", rotulo: "Apagar" })).toBe(c["key.backspace"]);
  });
});
