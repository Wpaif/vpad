import { describe, expect, it } from "vitest";
import { stickFromOffset } from "./stick";

const R = 100;

describe("stickFromOffset", () => {
  it("no centro é zero", () => {
    expect(stickFromOffset(0, 0, R)).toEqual({ x: 0, y: 0, knobX: 0, knobY: 0 });
  });

  it("deslocamento total à direita satura em 32767", () => {
    const s = stickFromOffset(100, 0, R);
    expect(s.x).toBe(32767);
    expect(s.y).toBe(0);
  });

  it("além do raio o botão fica preso no círculo", () => {
    const s = stickFromOffset(300, 400, R);
    expect(Math.hypot(s.knobX, s.knobY)).toBeCloseTo(R);
    expect(Math.hypot(s.x, s.y)).toBeLessThanOrEqual(32768);
  });

  it("zona morta ignora movimentos pequenos mas move o botão visual", () => {
    const s = stickFromOffset(5, 0, R, 0.1);
    expect(s.x).toBe(0);
    expect(s.knobX).toBe(5);
  });

  it("fora da zona morta a escala recomeça do zero", () => {
    const s = stickFromOffset(55, 0, R, 0.1);
    expect(s.x).toBeGreaterThan(0);
    expect(s.x).toBeLessThan(32767 * 0.55);
  });

  it("y para baixo é positivo", () => {
    expect(stickFromOffset(0, 100, R).y).toBe(32767);
    expect(stickFromOffset(0, -100, R).y).toBe(-32767);
  });
});
