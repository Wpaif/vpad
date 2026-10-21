import { beforeEach, describe, expect, it, vi } from "vitest";
import { defaultLayout } from "./layoutModel";
import { LAYOUT_KEY } from "./layoutStorage";
import { createLayoutStore, type LayoutStore } from "./layoutStore.svelte";
import { mountLayoutBar } from "./layoutBar";

let store: LayoutStore;
let bar: HTMLElement;
let edit: HTMLButtonElement;
let onChange: ReturnType<typeof vi.fn<() => void>>;
const act = (name: string) => bar.querySelector<HTMLButtonElement>(`[data-act=${name}]`)!;

beforeEach(() => {
  localStorage.clear();
  document.body.innerHTML = `
    <button id="edit"></button>
    <div id="bar" hidden>
      <button data-act="reset"></button><button data-act="cancel"></button><button data-act="save"></button>
      <span data-note role="status"></span>
    </div>`;
  bar = document.getElementById("bar")!;
  edit = document.getElementById("edit") as HTMLButtonElement;
  store = createLayoutStore();
  onChange = vi.fn<() => void>();
  mountLayoutBar({ bar, editButton: edit, store, onChange });
});

describe("layoutBar", () => {
  it("o botão de editar abre o modo edição e mostra a barra", () => {
    edit.click();
    expect(store.editing).toBe(true);
    expect(bar.hidden).toBe(false);
    expect(onChange).toHaveBeenCalled();
  });

  it("salvar grava, fecha a edição e esconde a barra", () => {
    edit.click();
    store.move("face", { dx: 0.1 });
    act("save").click();
    expect(store.editing).toBe(false);
    expect(bar.hidden).toBe(true);
    expect(localStorage.getItem(LAYOUT_KEY)).not.toBeNull();
  });

  it("cancelar descarta e esconde a barra", () => {
    edit.click();
    store.move("face", { dx: 0.1 });
    act("cancel").click();
    expect(store.editing).toBe(false);
    expect(bar.hidden).toBe(true);
    expect(store.current).toEqual(defaultLayout());
  });

  it("restaurar zera o rascunho mas continua em edição", () => {
    edit.click();
    store.move("face", { dx: 0.1 });
    act("reset").click();
    expect(store.current).toEqual(defaultLayout());
    expect(store.editing).toBe(true);
    expect(bar.hidden).toBe(false);
  });

  it("se não conseguir gravar, avisa e continua editando", () => {
    edit.click();
    store.move("face", { dx: 0.1 });
    const falha = () => { throw new Error("cheio"); };
    vi.stubGlobal("localStorage", { getItem: falha, setItem: falha, removeItem: falha });
    act("save").click();
    vi.unstubAllGlobals();
    expect(store.editing).toBe(true);
    expect(bar.hidden).toBe(false);
    expect(bar.querySelector("[data-note]")!.textContent).not.toBe("");
  });

  it("o aviso some ao tentar de novo com sucesso", () => {
    edit.click();
    store.move("face", { dx: 0.1 });
    const falha = () => { throw new Error("cheio"); };
    vi.stubGlobal("localStorage", { getItem: falha, setItem: falha, removeItem: falha });
    act("save").click();
    vi.unstubAllGlobals();
    act("save").click();
    expect(bar.querySelector("[data-note]")!.textContent).toBe("");
    expect(bar.hidden).toBe(true);
  });
});
