import { describe, expect, it } from "vitest";
import { PING_KEY, PING_NORMAL_MS, PING_SHORT_MS, loadShortPing, pingIntervalFor, saveShortPing } from "./pingMode";

const memoria = () => {
  const m = new Map<string, string>();
  return {
    getItem: (k: string) => m.get(k) ?? null,
    setItem: (k: string, v: string) => void m.set(k, v),
    removeItem: (k: string) => void m.delete(k),
  };
};

describe("pingMode", () => {
  it("o ping curto é bem mais rápido que o normal", () => {
    expect(PING_NORMAL_MS).toBe(1000);
    expect(PING_SHORT_MS).toBe(100);
    expect(pingIntervalFor(false)).toBe(PING_NORMAL_MS);
    expect(pingIntervalFor(true)).toBe(PING_SHORT_MS);
  });

  it("começa desligado", () => {
    expect(loadShortPing(memoria())).toBe(false);
  });

  it("guarda e lê a escolha", () => {
    const s = memoria();
    saveShortPing(s, true);
    expect(loadShortPing(s)).toBe(true);
    saveShortPing(s, false);
    expect(loadShortPing(s)).toBe(false);
    expect(s.getItem(PING_KEY)).toBeNull();
  });

  it("valor estranho guardado conta como desligado", () => {
    const s = memoria();
    s.setItem(PING_KEY, "talvez");
    expect(loadShortPing(s)).toBe(false);
  });

  it("armazenamento que lança não quebra", () => {
    const falha = () => { throw new Error("bloqueado"); };
    const s = { getItem: falha, setItem: falha, removeItem: falha };
    expect(loadShortPing(s)).toBe(false);
    expect(() => saveShortPing(s, true)).not.toThrow();
  });
});
