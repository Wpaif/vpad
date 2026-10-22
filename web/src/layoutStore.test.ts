import { beforeEach, describe, expect, it } from "vitest";
import { controlOffset, defaultLayout, isHidden } from "./layoutModel";
import { LAYOUT_KEY } from "./layoutStorage";
import { createLayoutStore } from "./layoutStore.svelte";

beforeEach(() => localStorage.clear());

describe("layoutStore", () => {
  it("começa com o layout salvo e sem edição", () => {
    const s = createLayoutStore();
    expect(s.editing).toBe(false);
    expect(s.current).toEqual(defaultLayout());
  });

  it("carrega o que estava salvo", () => {
    const a = createLayoutStore();
    a.edit();
    a.move("face", { dx: 0.1 });
    a.save();
    expect(createLayoutStore().current.groups.face?.dx).toBe(0.1);
  });

  it("alterações só aparecem no rascunho até salvar", () => {
    const s = createLayoutStore();
    s.edit();
    s.move("face", { dx: 0.2 });
    expect(s.current.groups.face?.dx).toBe(0.2);
    expect(localStorage.getItem(LAYOUT_KEY)).toBeNull();
    s.save();
    expect(s.editing).toBe(false);
    expect(localStorage.getItem(LAYOUT_KEY)).not.toBeNull();
  });

  it("cancelar descarta o rascunho", () => {
    const s = createLayoutStore();
    s.edit();
    s.hide("A", true);
    expect(isHidden(s.current, "A")).toBe(true);
    s.cancel();
    expect(s.editing).toBe(false);
    expect(isHidden(s.current, "A")).toBe(false);
  });

  it("restaurar o padrão zera o rascunho, mas só vale ao salvar", () => {
    const s = createLayoutStore();
    s.edit();
    s.move("menu", { dy: 0.3 });
    s.save();
    s.edit();
    s.restoreDefault();
    expect(s.current).toEqual(defaultLayout());
    s.cancel();
    expect(s.current.groups.menu?.dy).toBe(0.3);
  });

  it("alterar fora do modo edição não faz nada", () => {
    const s = createLayoutStore();
    s.move("face", { dx: 0.5 });
    expect(s.current).toEqual(defaultLayout());
  });

  it("moveControl mexe no rascunho, salva e cancela como o resto", () => {
    const s = createLayoutStore();
    s.edit();
    s.moveControl("A", { dx: 0.1, dy: 0.2 });
    expect(controlOffset(s.current, "A")).toEqual({ dx: 0.1, dy: 0.2 });
    expect(localStorage.getItem(LAYOUT_KEY)).toBeNull();
    s.cancel();
    expect(controlOffset(s.current, "A")).toEqual({ dx: 0, dy: 0 });
    s.edit();
    s.moveControl("A", { dx: 0.1 });
    s.save();
    expect(controlOffset(createLayoutStore().current, "A").dx).toBe(0.1);
  });

  it("moveControl fora do modo edição não faz nada", () => {
    const s = createLayoutStore();
    s.moveControl("A", { dx: 0.5 });
    expect(s.current).toEqual(defaultLayout());
  });

  it("restaurar o padrão também zera os deslocamentos individuais", () => {
    const s = createLayoutStore();
    s.edit();
    s.moveControl("B", { dx: 0.3 });
    s.restoreDefault();
    expect(s.current).toEqual(defaultLayout());
  });
});
