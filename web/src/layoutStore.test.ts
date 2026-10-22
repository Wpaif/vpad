import { beforeEach, describe, expect, it } from "vitest";
import { controlOffset, defaultLayout, isHidden, opacityOf, withGroup } from "./layoutModel";
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

  it("adopt troca o layout salvo (e a cópia local) quando não há edição", () => {
    const s = createLayoutStore();
    const remoto = withGroup(defaultLayout(), "face", { dx: 0.2 });
    expect(s.adopt(remoto)).toBe(true);
    expect(s.current.groups.face?.dx).toBe(0.2);
    expect(createLayoutStore().current.groups.face?.dx).toBe(0.2);
  });

  it("adopt é recusado durante a edição, sem mexer no rascunho", () => {
    const s = createLayoutStore();
    s.edit();
    s.move("menu", { dy: 0.1 });
    expect(s.adopt(withGroup(defaultLayout(), "face", { dx: 0.2 }))).toBe(false);
    expect(s.current.groups.face).toBeUndefined();
    expect(s.current.groups.menu?.dy).toBe(0.1);
  });

  it("adopt do padrão apaga a cópia local", () => {
    const s = createLayoutStore();
    s.adopt(withGroup(defaultLayout(), "face", { dx: 0.2 }));
    s.adopt(defaultLayout());
    expect(localStorage.getItem(LAYOUT_KEY)).toBeNull();
    expect(s.current).toEqual(defaultLayout());
  });

  it("saved devolve o layout salvo, sem o rascunho", () => {
    const s = createLayoutStore();
    s.edit();
    s.move("face", { dx: 0.3 });
    expect(s.saved).toEqual(defaultLayout());
  });

  it("setOpacity mexe no rascunho, salva e cancela como o resto", () => {
    const s = createLayoutStore();
    s.edit();
    s.setOpacity(0.5);
    expect(opacityOf(s.current)).toBe(0.5);
    expect(localStorage.getItem(LAYOUT_KEY)).toBeNull();
    s.cancel();
    expect(opacityOf(s.current)).toBe(1);
    s.edit();
    s.setOpacity(0.5);
    s.save();
    expect(opacityOf(createLayoutStore().current)).toBe(0.5);
  });

  it("setOpacity fora do modo edição não faz nada", () => {
    const s = createLayoutStore();
    s.setOpacity(0.5);
    expect(s.current).toEqual(defaultLayout());
  });

  it("restaurar o padrão volta a opacidade a 100%", () => {
    const s = createLayoutStore();
    s.edit();
    s.setOpacity(0.4);
    s.restoreDefault();
    expect(opacityOf(s.current)).toBe(1);
  });
});
