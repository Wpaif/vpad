import { readdirSync, readFileSync } from "node:fs";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CONTROLS, groupName } from "./controls";
import { applyStaticTranslations, LANGUAGES, setLanguage, t, type Language } from "./i18n";
import { applyProfileLabels, applyKeyboardLabels, displayedLabel } from "./labels";
import { renderSyncNote } from "./layoutSyncNote";
import { CATALOGS } from "./locales";
import { ptBR } from "./locales/pt-BR";
import { describeNotice } from "./notice";
import { pairDevice } from "./pairing";
import { mountPingToggle } from "./pingToggle";
import { PROFILES } from "./profiles";
import { montarPagina } from "./testing/pagina";
import { renderStatus } from "./ui";

const OUTROS = LANGUAGES.map((l) => l.code).filter((c): c is Exclude<Language, "pt-BR"> => c !== "pt-BR");
const catalogo = (l: Language) => CATALOGS[l] as Record<string, string>;

beforeEach(() => setLanguage("pt-BR"));

describe("a página traduz tudo que é texto fixo do markup", () => {
  it("toda chave data-i18n* do index.html existe no catálogo", () => {
    const html = readFileSync("index.html", "utf8");
    const chaves = [...html.matchAll(/data-i18n(?:-html|-aria-label|-title|-placeholder)?="([^"]+)"/g)].map((m) => m[1]!);
    expect(chaves.length).toBeGreaterThan(30);
    for (const chave of chaves) expect(chave in ptBR, chave).toBe(true);
  });

  it("em português o markup não muda (os textos do HTML são os do catálogo)", () => {
    montarPagina();
    const antes = document.body.innerHTML;
    applyStaticTranslations(document);
    expect(document.body.innerHTML).toBe(antes);
  });

  it.each(OUTROS)("em %s cada elemento marcado recebe o texto do catálogo", (idioma) => {
    setLanguage(idioma);
    montarPagina();
    applyStaticTranslations(document);
    for (const el of document.querySelectorAll<HTMLElement>("[data-i18n]")) {
      expect(el.textContent, el.getAttribute("data-i18n")!).toBe(catalogo(idioma)[el.getAttribute("data-i18n")!]);
    }
    for (const el of document.querySelectorAll<HTMLElement>("[data-i18n-html]")) {
      expect(el.innerHTML, el.getAttribute("data-i18n-html")!).toBe(catalogo(idioma)[el.getAttribute("data-i18n-html")!]);
    }
    for (const el of document.querySelectorAll<HTMLElement>("[data-i18n-aria-label]")) {
      expect(el.getAttribute("aria-label"), el.getAttribute("data-i18n-aria-label")!).toBe(
        catalogo(idioma)[el.getAttribute("data-i18n-aria-label")!],
      );
    }
  });

  it.each(OUTROS)("em %s o Pad (montado já no idioma) tem os nomes acessíveis traduzidos", (idioma) => {
    setLanguage(idioma);
    const pad = montarPagina();
    expect(pad.getAttribute("aria-label")).toBe(catalogo(idioma)["pad.aria"]);
    expect(pad.querySelector("[data-btn=Up]")!.getAttribute("aria-label")).toBe(catalogo(idioma)["direction.up"]);
    expect(pad.querySelector("[data-btn=Down]")!.getAttribute("aria-label")).toBe(catalogo(idioma)["direction.down"]);
  });
});

describe("os módulos traduzem na hora do uso (e não ao carregar)", () => {
  it.each(OUTROS)("%s: status, avisos, perfil, controles e rótulos", (idioma) => {
    setLanguage(idioma);
    const c = catalogo(idioma);

    const el = document.createElement("button");
    renderStatus(el, "reconnecting", null);
    expect(el.textContent).toBe(c["status.reconnecting"]);

    expect(describeNotice("denied", null)).toBe(c["notice.denied"]);
    expect(describeNotice("no-permission", "0123abcd")).toBe(c["notice.noPermissionWithId"]!.replace("{id}", "0123abcd"));

    expect(PROFILES.find((p) => p.id === "teclado")!.label).toBe(c["profiles.keyboard"]);
    expect(PROFILES.find((p) => p.id === "xbox360")!.label).toBe("Xbox 360");

    expect(groupName("Direcional")).toBe(c["controls.group.dpad"]);
    expect(CONTROLS.find((x) => x.id === "A")!.label).toBe(c["controls.A"]);
    expect(CONTROLS.find((x) => x.id === "LB")!.label).toBe("LB");
    expect(CONTROLS.find((x) => x.id === "LS_Left")!.label).toBe(c["direction.left"]);

    const nota = document.createElement("p");
    renderSyncNote(nota, "pending");
    expect(nota.textContent).toBe(c["layoutSync.pending"]);
    renderSyncNote(nota, "none");
    expect(nota.textContent).toBe("");

    const botao = document.createElement("button");
    mountPingToggle(botao, true, () => {});
    expect(botao.textContent).toBe(c["ping.on"]);
    botao.click();
    expect(botao.textContent).toBe(c["ping.off"]);
  });

  it.each(OUTROS)("%s: nomes acessíveis do DualShock, do Switch e do modo teclado", (idioma) => {
    setLanguage(idioma);
    const c = catalogo(idioma);
    const pad = montarPagina();
    applyProfileLabels(pad, "ds4");
    expect(displayedLabel(pad, "A")).toBe(c["shape.cross"]);
    expect(displayedLabel(pad, "Y")).toBe(c["shape.triangle"]);
    applyProfileLabels(pad, "switch-pro");
    expect(displayedLabel(pad, "Select")).toBe(c["sign.minus"]);
    applyKeyboardLabels(pad, { A: "KeyK" }, [{ id: "KeyK", rotulo: "K" }]);
    expect(pad.querySelector("[data-btn=A]")!.getAttribute("aria-label")).toBe(c["labels.keycap"]!.replace("{name}", "K"));
    expect(pad.querySelector("[data-btn=B]")!.getAttribute("aria-label")).toBe(c["labels.keycap.empty"]);
  });

  it.each(OUTROS)("%s: erros do pareamento", async (idioma) => {
    setLanguage(idioma);
    const c = catalogo(idioma);
    const resposta = (status: number, corpo: unknown = {}) =>
      vi.fn(async () => new Response(JSON.stringify(corpo), { status })) as unknown as typeof fetch;
    await expect(pairDevice("123456", "x", resposta(401))).rejects.toThrow(c["pairing.error.wrongCode"]);
    await expect(pairDevice("123456", "x", resposta(403, { erro: "o pareamento está fechado" }))).rejects.toThrow(c["pairing.error.closed"]);
    await expect(pairDevice("123456", "x", resposta(409))).rejects.toThrow(c["pairing.error.busy"]);
    await expect(pairDevice("123456", "x", resposta(429, { segundos: 42 }))).rejects.toThrow(
      c["pairing.error.rateLimited"]!.replace("{seconds}", "42"),
    );
    await expect(pairDevice("123456", "x", resposta(500))).rejects.toThrow(c["pairing.error.http"]!.replace("{status}", "500"));
    const queda = vi.fn(async () => {
      throw new TypeError("Failed to fetch");
    }) as unknown as typeof fetch;
    await expect(pairDevice("123456", "x", queda)).rejects.toThrow(c["pairing.error.offline"]);
  });
});

describe("não sobra texto em português fixo no código", () => {
  /** ids internos que parecem texto, mas nunca aparecem na tela */
  const PERMITIDOS = new Set([
    "Botões de face", "Ombros e menu", "Direcional", "Gatilhos", "Analógico esquerdo", "Analógico direito",
    "Português", // o nome de cada idioma aparece na própria língua, no seletor
  ]);
  /** viewport.ts só monta o painel de diagnóstico de `?debug`, uma ferramenta de desenvolvedor */
  const FORA_DO_ESCOPO = new Set(["viewport.ts"]);

  const arquivos = readdirSync("src")
    .filter((f) => /\.(ts|svelte)$/.test(f) && !f.endsWith(".test.ts") && !f.endsWith(".d.ts") && !FORA_DO_ESCOPO.has(f))
    .map((f) => `src/${f}`);

  /** Tira os comentários (de bloco, de linha e de marcação) e devolve só o que vira texto ou código. */
  const semComentarios = (codigo: string) =>
    codigo
      .replace(/\/\*[\s\S]*?\*\//g, "")
      .replace(/<!--[\s\S]*?-->/g, "")
      .replace(/(^|[^:"'`\\])\/\/.*$/gm, "$1");

  it("os literais de texto com acento estão só nos catálogos (e nos ids internos permitidos)", () => {
    const achados: string[] = [];
    for (const arquivo of arquivos) {
      const codigo = semComentarios(readFileSync(arquivo, "utf8"));
      for (const m of codigo.matchAll(/(["'`])((?:\\.|(?!\1)[^\\\n])*[áéíóúâêôãõçÁÉÍÓÚÂÊÔÃÕÇ](?:\\.|(?!\1)[^\\\n])*)\1/g)) {
        if (!PERMITIDOS.has(m[2]!)) achados.push(`${arquivo}: ${m[0]}`);
      }
    }
    expect(achados).toEqual([]);
  });

  it("o inglês e os outros idiomas não repetem o português por esquecimento", () => {
    // textos que podem ser iguais ao português porque são o mesmo nesse idioma (nomes, siglas, palavras próximas)
    const IGUAL_E_OK = new Set(["stats.jitter", "language.label", "stats.min", "stats.max", "keyEditor.open", "status.closed"]);
    for (const idioma of OUTROS) {
      const c = catalogo(idioma);
      const iguais = Object.keys(ptBR).filter((k) => c[k] === (ptBR as Record<string, string>)[k] && !IGUAL_E_OK.has(k));
      if (idioma === "es") continue; // o espanhol compartilha muitas palavras com o português
      expect(iguais, idioma).toEqual([]);
    }
    expect(t("language.label")).toBe(ptBR["language.label"]);
  });
});
