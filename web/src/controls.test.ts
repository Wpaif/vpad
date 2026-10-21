import { describe, expect, it } from "vitest";
import { CONTROLS, CONTROL_GROUPS } from "./controls";

describe("CONTROLS", () => {
  it("segue a mesma ordem dos controles do servidor", () => {
    // Ver Controle::todos em src/mapa.rs: o servidor também envia esta lista em GET /api/teclado.
    expect(CONTROLS.map((c) => c.id)).toEqual([
      "A", "B", "X", "Y", "LB", "RB", "Select", "Start", "L3", "R3", "Up", "Down", "Left", "Right",
      "LT", "RT", "LS_Up", "LS_Down", "LS_Left", "LS_Right", "RS_Up", "RS_Down", "RS_Left", "RS_Right",
    ]);
  });

  it("todo controle tem nome em português e pertence a um grupo conhecido", () => {
    for (const c of CONTROLS) {
      expect(c.label.length).toBeGreaterThan(0);
      expect(CONTROL_GROUPS).toContain(c.group);
    }
  });

  it("os grupos aparecem na ordem em que os controles são listados, sem repetir", () => {
    const vistos: string[] = [];
    for (const c of CONTROLS) if (vistos.at(-1) !== c.group) vistos.push(c.group);
    expect(vistos).toEqual([...CONTROL_GROUPS]);
  });
});
