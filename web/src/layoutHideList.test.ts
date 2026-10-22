import { beforeEach, describe, expect, it, vi } from "vitest";
import { CONTROL_IDS, isHidden } from "./layoutModel";
import { createLayoutStore, type LayoutStore } from "./layoutStore.svelte";
import { mountHideList } from "./layoutHideList";

let store: LayoutStore;
let panel: HTMLElement;
let opener: HTMLButtonElement;
let onChange: ReturnType<typeof vi.fn<() => void>>;
let list: { sync: () => void };
let rotulos: Record<string, string | null>;

const chip = (id: string) => panel.querySelector<HTMLElement>(`[data-control=${id}]`)!;

beforeEach(() => {
  localStorage.clear();
  document.body.innerHTML = `<button id="op"></button><div id="panel" hidden></div>`;
  panel = document.getElementById("panel")!;
  opener = document.getElementById("op") as HTMLButtonElement;
  store = createLayoutStore();
  onChange = vi.fn<() => void>();
  rotulos = {};
  list = mountHideList({ panel, opener, store, onChange, labelOf: (id) => rotulos[id] ?? null });
});

describe("layoutHideList", () => {
  it("tem um interruptor por controle, na ordem do catálogo", () => {
    const ids = [...panel.querySelectorAll<HTMLElement>("[data-control]")].map((c) => c.dataset.control);
    expect(ids).toEqual([...CONTROL_IDS]);
    for (const c of panel.querySelectorAll("[data-control]")) {
      expect(c.getAttribute("role")).toBe("switch");
      expect(c.textContent!.trim()).not.toBe("");
    }
  });

  it("os nomes são legíveis, inclusive para os analógicos e o direcional", () => {
    expect(chip("A").textContent).toBe("A");
    expect(chip("Up").textContent).toBe("Cima");
    expect(chip("StickLeft").textContent).toBe("Analógico esq.");
    expect(chip("StickRight").textContent).toBe("Analógico dir.");
  });

  it("começa tudo visível (aria-checked verdadeiro)", () => {
    for (const c of panel.querySelectorAll("[data-control]")) expect(c.getAttribute("aria-checked")).toBe("true");
  });

  it("o painel só abre pelo botão, e só em modo edição", () => {
    opener.click();
    expect(panel.hidden).toBe(true); // sem edição
    store.edit();
    opener.click();
    expect(panel.hidden).toBe(false);
    opener.click();
    expect(panel.hidden).toBe(true);
  });

  it("clicar oculta o controle no rascunho e marca o interruptor", () => {
    store.edit();
    opener.click();
    chip("A").click();
    expect(isHidden(store.current, "A")).toBe(true);
    expect(chip("A").getAttribute("aria-checked")).toBe("false");
    expect(onChange).toHaveBeenCalled();
    chip("A").click();
    expect(isHidden(store.current, "A")).toBe(false);
    expect(chip("A").getAttribute("aria-checked")).toBe("true");
  });

  it("fora do modo edição o clique não muda nada", () => {
    chip("B").click();
    expect(isHidden(store.current, "B")).toBe(false);
  });

  it("sync reflete mudanças feitas de fora, como restaurar o padrão", () => {
    store.edit();
    opener.click();
    chip("X").click();
    store.restoreDefault();
    list.sync();
    expect(chip("X").getAttribute("aria-checked")).toBe("true");
  });

  it("sync fecha o painel quando a edição termina", () => {
    store.edit();
    opener.click();
    expect(panel.hidden).toBe(false);
    store.cancel();
    list.sync();
    expect(panel.hidden).toBe(true);
    store.edit();
    list.sync();
    expect(panel.hidden).toBe(true); // reabrir exige clicar de novo
  });
});

describe("nomes seguem o controle em uso", () => {
  it("usa o rótulo do perfil quando existe e o nome padrão quando não", () => {
    rotulos = { A: "Cruz", LT: "L2", Select: null };
    list.sync();
    expect(chip("A").textContent).toBe("Cruz");
    expect(chip("LT").textContent).toBe("L2");
    expect(chip("Select").textContent).toBe("Select");
    expect(chip("Up").textContent).toBe("Cima");
  });

  it("troca os nomes quando o perfil muda (sync de novo)", () => {
    rotulos = { A: "Cruz" };
    list.sync();
    rotulos = { A: "B" };
    list.sync();
    expect(chip("A").textContent).toBe("B");
  });

  it("abrir a lista já mostra os nomes atuais", () => {
    store.edit();
    rotulos = { X: "Quadrado" };
    opener.click();
    expect(chip("X").textContent).toBe("Quadrado");
  });

  it("o nome acessível do interruptor continua sendo o rótulo atual", () => {
    rotulos = { Y: "Triângulo" };
    list.sync();
    expect(chip("Y").getAttribute("aria-label")).toBe("Triângulo");
  });
});
