import { beforeEach, describe, expect, it, vi } from "vitest";
import { defaultLayout, setHidden, withGroup } from "./layoutModel";
import { LAYOUT_KEY, loadLayout, saveLayout } from "./layoutStorage";

beforeEach(() => localStorage.clear());

describe("layoutStorage", () => {
  it("sem nada salvo devolve o padrão", () => {
    expect(loadLayout()).toEqual(defaultLayout());
  });

  it("salva e recarrega o mesmo layout", () => {
    const layout = setHidden(withGroup(defaultLayout(), "face", { dx: 0.1, scale: 1.2 }), "A", true);
    expect(saveLayout(layout)).toBe(true);
    expect(loadLayout()).toEqual(layout);
  });

  it("salvar o padrão remove a chave", () => {
    saveLayout(withGroup(defaultLayout(), "menu", { dy: 0.2 }));
    expect(localStorage.getItem(LAYOUT_KEY)).not.toBeNull();
    saveLayout(defaultLayout());
    expect(localStorage.getItem(LAYOUT_KEY)).toBeNull();
  });

  it("JSON corrompido volta ao padrão", () => {
    localStorage.setItem(LAYOUT_KEY, "{não é json");
    expect(loadLayout()).toEqual(defaultLayout());
  });

  it("versão futura volta ao padrão sem apagar o que estava salvo", () => {
    const futuro = JSON.stringify({ version: 99, groups: {}, hidden: [] });
    localStorage.setItem(LAYOUT_KEY, futuro);
    expect(loadLayout()).toEqual(defaultLayout());
    expect(localStorage.getItem(LAYOUT_KEY)).toBe(futuro);
  });

  it("armazenamento que lança não quebra a leitura nem a gravação", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => { throw new Error("bloqueado"); });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new Error("cheio"); });
    expect(loadLayout()).toEqual(defaultLayout());
    expect(saveLayout(withGroup(defaultLayout(), "face", { dx: 0.1 }))).toBe(false);
    vi.restoreAllMocks();
  });
});
