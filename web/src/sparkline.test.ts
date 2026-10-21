import { describe, expect, it } from "vitest";
import { sparklinePoints } from "./sparkline";

describe("sparklinePoints", () => {
  it("sem amostras não desenha nada", () => {
    expect(sparklinePoints([], 100, 20, 50)).toBe("");
  });

  it("uma amostra vira uma linha horizontal de ponta a ponta", () => {
    expect(sparklinePoints([25], 100, 20, 50)).toBe("0,10 100,10");
  });

  it("espalha em x e inverte y (maior latência = mais alto)", () => {
    expect(sparklinePoints([0, 50], 100, 20, 50)).toBe("0,20 100,0");
  });

  it("valores acima do teto ficam no topo", () => {
    expect(sparklinePoints([0, 500], 100, 20, 50)).toBe("0,20 100,0");
  });

  it("três amostras ficam igualmente espaçadas", () => {
    expect(sparklinePoints([10, 20, 10], 100, 20, 20)).toBe("0,10 50,0 100,10");
  });
});
