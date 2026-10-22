import { beforeEach, describe, expect, it, vi } from "vitest";
import { defaultLayout, setHidden, withGroup } from "./layoutModel";
import { LAYOUT_KEY, PENDING_KEY, isPending, loadLayout, saveLayout, setPending } from "./layoutStorage";

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
    const falha = () => { throw new Error("bloqueado"); };
    vi.stubGlobal("localStorage", { getItem: falha, setItem: falha, removeItem: falha });
    expect(loadLayout()).toEqual(defaultLayout());
    expect(saveLayout(withGroup(defaultLayout(), "face", { dx: 0.1 }))).toBe(false);
    vi.unstubAllGlobals();
  });
});

describe("marca de envio pendente", () => {
  it("começa sem pendência", () => {
    expect(isPending()).toBe(false);
  });

  it("liga e desliga, e desligar apaga a chave", () => {
    setPending(true);
    expect(isPending()).toBe(true);
    expect(localStorage.getItem(PENDING_KEY)).toBe("1");
    setPending(false);
    expect(isPending()).toBe(false);
    expect(localStorage.getItem(PENDING_KEY)).toBeNull();
  });

  it("armazenamento que lança não quebra", () => {
    const falha = () => { throw new Error("bloqueado"); };
    vi.stubGlobal("localStorage", { getItem: falha, setItem: falha, removeItem: falha });
    expect(isPending()).toBe(false);
    expect(() => setPending(true)).not.toThrow();
    vi.unstubAllGlobals();
  });
});
