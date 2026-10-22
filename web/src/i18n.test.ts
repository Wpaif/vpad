import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  DEFAULT_LANGUAGE,
  LANGUAGES,
  LANGUAGE_KEY,
  applyStaticTranslations,
  chooseLanguage,
  detectLanguage,
  getLanguage,
  initLanguage,
  setLanguage,
  t,
} from "./i18n";
import { CATALOGS } from "./locales";
import { ptBR } from "./locales/pt-BR";

const memoria = () => {
  const m = new Map<string, string>();
  return {
    getItem: (k: string) => m.get(k) ?? null,
    setItem: (k: string, v: string) => void m.set(k, v),
    removeItem: (k: string) => void m.delete(k),
  };
};

beforeEach(() => {
  setLanguage("pt-BR");
  document.documentElement.removeAttribute("lang");
});

describe("idiomas disponíveis", () => {
  it("são seis, com o português primeiro e o nome de cada um na própria língua", () => {
    expect(LANGUAGES.map((l) => l.code)).toEqual(["pt-BR", "en", "es", "zh-CN", "ja", "ko"]);
    expect(LANGUAGES.map((l) => l.name)).toEqual(["Português", "English", "Español", "中文", "日本語", "한국어"]);
    expect(DEFAULT_LANGUAGE).toBe("pt-BR");
  });

  it("cada idioma tem um catálogo", () => {
    expect(Object.keys(CATALOGS).sort()).toEqual(LANGUAGES.map((l) => l.code).sort());
  });
});

describe("catálogos", () => {
  const chaves = Object.keys(ptBR);
  const marcadores = (texto: string) => [...texto.matchAll(/\{(\w+)\}/g)].map((m) => m[1]!).sort();

  it.each(LANGUAGES.map((l) => l.code))("%s tem exatamente as mesmas chaves do português", (codigo) => {
    expect(Object.keys(CATALOGS[codigo]).sort()).toEqual([...chaves].sort());
  });

  it.each(LANGUAGES.map((l) => l.code))("%s não tem texto vazio e usa os mesmos {marcadores}", (codigo) => {
    for (const chave of chaves) {
      const texto = (CATALOGS[codigo] as Record<string, string>)[chave]!;
      expect(texto.trim(), `${codigo}: ${chave}`).not.toBe("");
      expect(marcadores(texto), `${codigo}: ${chave}`).toEqual(marcadores((ptBR as Record<string, string>)[chave]!));
    }
  });
});

describe("t", () => {
  it("devolve o texto do idioma atual (português por padrão)", () => {
    expect(getLanguage()).toBe("pt-BR");
    expect(t("language.label")).toBe(ptBR["language.label"]);
  });

  it("troca de texto quando o idioma muda", () => {
    setLanguage("en");
    expect(t("language.label")).toBe(CATALOGS.en["language.label"]);
    expect(t("language.label")).not.toBe(ptBR["language.label"]);
  });

  it("preenche os {marcadores}", () => {
    setLanguage("pt-BR");
    const chave = Object.keys(ptBR).find((k) => /\{\w+\}/.test((ptBR as Record<string, string>)[k]!)) as keyof typeof ptBR | undefined;
    if (!chave) return; // ainda não há texto com marcador
    const nome = /\{(\w+)\}/.exec((ptBR as Record<string, string>)[chave]!)![1]!;
    expect(t(chave, { [nome]: "XYZ" })).toContain("XYZ");
    expect(t(chave, { [nome]: "XYZ" })).not.toContain(`{${nome}}`);
  });

  it("chave desconhecida devolve a própria chave, para o erro aparecer na tela", () => {
    expect(t("nao.existe" as never)).toBe("nao.existe");
  });
});

describe("detectLanguage", () => {
  it.each([
    [["pt-BR"], "pt-BR"],
    [["pt-PT"], "pt-BR"],
    [["pt"], "pt-BR"],
    [["en-US"], "en"],
    [["en-GB"], "en"],
    [["es-MX"], "es"],
    [["es"], "es"],
    [["zh-CN"], "zh-CN"],
    [["zh-Hans-CN"], "zh-CN"],
    [["zh-TW"], "zh-CN"],
    [["ja-JP"], "ja"],
    [["ko-KR"], "ko"],
    [["KO"], "ko"],
  ] as const)("%j vira %s", (tags, esperado) => {
    expect(detectLanguage(tags)).toBe(esperado);
  });

  it("a primeira língua suportada da lista de preferência vence", () => {
    expect(detectLanguage(["fr-FR", "ja-JP", "en-US"])).toBe("ja");
  });

  it("língua sem tradução cai para o inglês, e lista vazia também", () => {
    expect(detectLanguage(["fr-FR", "de"])).toBe("en");
    expect(detectLanguage([])).toBe("en");
  });
});

describe("initLanguage", () => {
  it("usa o idioma que o usuário escolheu antes", () => {
    const s = memoria();
    s.setItem(LANGUAGE_KEY, "ko");
    expect(initLanguage({ languages: ["en-US"], storage: s })).toBe("ko");
    expect(getLanguage()).toBe("ko");
    expect(document.documentElement.lang).toBe("ko");
  });

  it("sem escolha, segue o idioma do celular", () => {
    expect(initLanguage({ languages: ["es-ES"], storage: memoria() })).toBe("es");
    expect(document.documentElement.lang).toBe("es");
  });

  it("escolha guardada inválida é ignorada", () => {
    const s = memoria();
    s.setItem(LANGUAGE_KEY, "klingon");
    expect(initLanguage({ languages: ["ja-JP"], storage: s })).toBe("ja");
  });

  it("armazenamento que lança não quebra", () => {
    const falha = () => { throw new Error("bloqueado"); };
    expect(initLanguage({ languages: ["zh-CN"], storage: { getItem: falha, setItem: falha, removeItem: falha } })).toBe("zh-CN");
  });
});

describe("chooseLanguage", () => {
  it("guarda a escolha e recarrega a página para tudo aparecer no idioma novo", () => {
    const s = memoria();
    const recarregar = vi.fn();
    chooseLanguage("es", s, recarregar);
    expect(s.getItem(LANGUAGE_KEY)).toBe("es");
    expect(recarregar).toHaveBeenCalledTimes(1);
  });

  it("idioma desconhecido não guarda nem recarrega", () => {
    const s = memoria();
    const recarregar = vi.fn();
    chooseLanguage("xx" as never, s, recarregar);
    expect(s.getItem(LANGUAGE_KEY)).toBeNull();
    expect(recarregar).not.toHaveBeenCalled();
  });

  it("escolher o idioma que já está em uso não recarrega à toa", () => {
    const recarregar = vi.fn();
    chooseLanguage(getLanguage(), memoria(), recarregar);
    expect(recarregar).not.toHaveBeenCalled();
  });
});

describe("applyStaticTranslations", () => {
  beforeEach(() => {
    document.body.innerHTML = `
      <p id="a" data-i18n="language.label">Idioma</p>
      <button id="b" data-i18n-aria-label="language.label" aria-label="Idioma"></button>
      <span id="c" data-i18n-title="language.label" title="Idioma"></span>
      <p id="d" data-i18n="nao.existe">Texto original</p>`;
  });

  it("traduz o conteúdo, o aria-label e o title marcados", () => {
    setLanguage("en");
    applyStaticTranslations(document);
    const en = CATALOGS.en["language.label"];
    expect(document.getElementById("a")!.textContent).toBe(en);
    expect(document.getElementById("b")!.getAttribute("aria-label")).toBe(en);
    expect(document.getElementById("c")!.getAttribute("title")).toBe(en);
  });

  it("chave que não existe deixa o texto original", () => {
    setLanguage("en");
    applyStaticTranslations(document);
    expect(document.getElementById("d")!.textContent).toBe("Texto original");
  });

  it("em português deixa tudo como está", () => {
    applyStaticTranslations(document);
    expect(document.getElementById("a")!.textContent).toBe(ptBR["language.label"]);
  });
});
