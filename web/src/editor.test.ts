import { beforeEach, describe, expect, it, vi, type Mock } from "vitest";
import { CONTROLS } from "./controls";
import { mountKeyEditor, type EditorDeps } from "./editor";
import { setLanguage } from "./i18n";
import type { KeyboardConfig, KeyMap } from "./keyboard";

const TECLAS = [
  { id: "KeyK", rotulo: "K", grupo: "Letras" },
  { id: "KeyL", rotulo: "L", grupo: "Letras" },
  { id: "Space", rotulo: "Espaço", grupo: "Especiais" },
  { id: "ArrowUp", rotulo: "↑", grupo: "Setas" },
];

function config(mapa: KeyMap): KeyboardConfig {
  return {
    mapa,
    padrao: { A: "KeyK", B: "KeyL" },
    controles: CONTROLS.map((c) => c.id),
    teclas: TECLAS,
  };
}

let root: HTMLElement;
let load: Mock<EditorDeps["load"]>;
let save: Mock<EditorDeps["save"]>;
let onSaved: Mock<EditorDeps["onSaved"]>;
let editor: ReturnType<typeof mountKeyEditor>;

const flush = () => new Promise((r) => setTimeout(r, 0));
const q = <T extends HTMLElement>(sel: string) => root.querySelector<T>(sel)!;
const row = (id: string) => q(`[data-control="${id}"]`);
const chip = (id: string) => row(id).querySelector("kbd")!.textContent;
const picker = () => q(".key-picker");
const status = () => q(".editor-status").textContent;

async function abrir(mapa: KeyMap = { A: "KeyK", B: "KeyL" }) {
  load.mockResolvedValue(config(mapa));
  await editor.open();
  await flush();
}

beforeEach(() => {
  document.body.innerHTML = `<div id="editor" hidden></div>`;
  root = document.getElementById("editor")!;
  load = vi.fn<EditorDeps["load"]>();
  save = vi.fn<EditorDeps["save"]>(async (m: KeyMap) => m);
  onSaved = vi.fn<EditorDeps["onSaved"]>();
  editor = mountKeyEditor(root, { load, save, onSaved });
});

describe("abrir e listar", () => {
  it("começa escondido e só carrega ao abrir", async () => {
    expect(root.hidden).toBe(true);
    expect(load).not.toHaveBeenCalled();
    await abrir();
    expect(root.hidden).toBe(false);
    expect(load).toHaveBeenCalledTimes(1);
  });

  it("lista os 24 controles agrupados, mostrando a tecla de cada um", async () => {
    await abrir();
    expect(root.querySelectorAll("[data-control]").length).toBe(24);
    expect(chip("A")).toBe("K");
    expect(chip("B")).toBe("L");
    expect(chip("RS_Up")).toBe("–");
    const grupos = [...root.querySelectorAll(".editor-body h3")].map((h) => h.textContent);
    expect(grupos[0]).toBe("Botões de face");
    expect(grupos.at(-1)).toBe("Analógico direito");
  });

  it("falha ao carregar mostra o erro e não monta a lista", async () => {
    load.mockRejectedValue(new Error("sem conexão com o PC"));
    await editor.open();
    await flush();
    expect(status()).toContain("sem conexão com o PC");
    expect(root.querySelectorAll("[data-control]").length).toBe(0);
  });

  it("abrir de novo recarrega do servidor", async () => {
    await abrir();
    editor.close();
    await abrir({ A: "Space" });
    expect(load).toHaveBeenCalledTimes(2);
    expect(chip("A")).toBe("Espaço");
  });

  it("fechar esconde o editor", async () => {
    await abrir();
    q<HTMLButtonElement>("[data-action=close]").click();
    expect(root.hidden).toBe(true);
  });
});

describe("escolher a tecla de um controle", () => {
  beforeEach(() => abrir());

  it("tocar num controle abre a lista de teclas, com a atual marcada", () => {
    expect(picker().hidden).toBe(true);
    row("A").click();
    expect(picker().hidden).toBe(false);
    expect(picker().textContent).toContain("Baixo (A)");
    expect(q('[data-key="KeyK"]').getAttribute("aria-pressed")).toBe("true");
    expect(q('[data-key="KeyL"]').getAttribute("aria-pressed")).toBe("false");
    const grupos = [...picker().querySelectorAll("h4")].map((h) => h.textContent);
    expect(grupos).toEqual(["Letras", "Especiais", "Setas"]);
  });

  it("escolher uma tecla salva o mapa inteiro, atualiza o botão e avisa", async () => {
    row("A").click();
    q('[data-key="Space"]').click();
    await flush();

    expect(save).toHaveBeenCalledWith({ A: "Space", B: "KeyL" });
    expect(chip("A")).toBe("Espaço");
    expect(picker().hidden).toBe(true);
    expect(onSaved).toHaveBeenCalledWith({ A: "Space", B: "KeyL" }, expect.objectContaining({ teclas: TECLAS }));
  });

  it("'Nenhuma' tira a tecla do controle", async () => {
    row("A").click();
    q("[data-action=none]").click();
    await flush();
    expect(save).toHaveBeenCalledWith({ B: "KeyL" });
    expect(chip("A")).toBe("–");
  });

  it("controle sem tecla pode ganhar uma", async () => {
    row("RS_Up").click();
    q('[data-key="ArrowUp"]').click();
    await flush();
    expect(save).toHaveBeenCalledWith({ A: "KeyK", B: "KeyL", RS_Up: "ArrowUp" });
  });

  it("voltar fecha só a lista de teclas", () => {
    row("A").click();
    q("[data-action=back]").click();
    expect(picker().hidden).toBe(true);
    expect(root.hidden).toBe(false);
    expect(save).not.toHaveBeenCalled();
  });

  it("avisa quando a tecla escolhida já é usada por outro controle, mas salva", async () => {
    row("B").click();
    q('[data-key="KeyK"]').click(); // KeyK já é do A
    await flush();
    expect(save).toHaveBeenCalledWith({ A: "KeyK", B: "KeyK" });
    expect(status()).toContain("Baixo (A)");
  });

  it("erro ao salvar mostra a mensagem, mantém a tecla antiga e a lista aberta para tentar de novo", async () => {
    save.mockRejectedValue(new Error("não consegui gravar: disco cheio"));
    row("A").click();
    q('[data-key="Space"]').click();
    await flush();

    expect(status()).toContain("disco cheio");
    expect(chip("A")).toBe("K");
    expect(picker().hidden).toBe(false);
    expect(onSaved).not.toHaveBeenCalled();
  });
});

describe("restaurar o padrão", () => {
  it("salva o mapa padrão e atualiza a lista", async () => {
    await abrir({ A: "Space", RS_Up: "ArrowUp" });
    q("[data-action=reset]").click();
    await flush();
    expect(save).toHaveBeenCalledWith({ A: "KeyK", B: "KeyL" });
    expect(chip("A")).toBe("K");
    expect(chip("RS_Up")).toBe("–");
    expect(onSaved).toHaveBeenCalled();
  });
});

describe("rolagem", () => {
  it("as duas listas são áreas rolável marcadas com data-scroll", async () => {
    await abrir();
    expect(root.querySelectorAll("[data-scroll]").length).toBe(2);
  });
});

describe("idioma", () => {
  it("em inglês os grupos e as teclas com nome em palavra aparecem traduzidos", async () => {
    setLanguage("en");
    await abrir();
    row("A").click();
    const grupos = [...picker().querySelectorAll("h4")].map((h) => h.textContent);
    expect(grupos).toEqual(["Letters", "Special", "Arrows"]);
    expect(picker().querySelector('[data-key="Space"]')!.textContent).toBe("Space");
    expect(picker().querySelector('[data-key="KeyK"]')!.textContent).toBe("K");
    // a tecla de cada controle também aparece com o nome traduzido
    await (async () => {
      load.mockResolvedValue(config({ A: "Space" }));
      await editor.open();
      await flush();
    })();
    expect(chip("A")).toBe("Space");
    setLanguage("pt-BR");
  });
});
