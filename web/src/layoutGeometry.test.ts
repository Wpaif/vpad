import { describe, expect, it } from "vitest";
import { limitShift, snapShift, type Rect } from "./layoutGeometry";

const r = (left: number, top: number, width: number, height: number): Rect => ({
  left,
  top,
  right: left + width,
  bottom: top + height,
});

const BOUNDS = r(0, 0, 800, 400);

describe("limitShift", () => {
  it("deixa passar um deslocamento que cabe", () => {
    expect(limitShift(r(100, 100, 50, 50), { x: 30, y: -20 }, BOUNDS)).toEqual({ x: 30, y: -20 });
  });

  it("não deixa sair pela esquerda e por cima", () => {
    expect(limitShift(r(10, 10, 50, 50), { x: -100, y: -100 }, BOUNDS)).toEqual({ x: -10, y: -10 });
  });

  it("não deixa sair pela direita e por baixo", () => {
    expect(limitShift(r(700, 300, 50, 50), { x: 100, y: 100 }, BOUNDS)).toEqual({ x: 50, y: 50 });
  });

  it("grupo maior que a área fica alinhado à esquerda/topo", () => {
    expect(limitShift(r(0, 0, 900, 500), { x: 40, y: 40 }, BOUNDS)).toEqual({ x: 0, y: 0 });
  });
});

describe("snapShift", () => {
  const alvo = r(300, 100, 100, 100); // esquerda 300, centro 350, direita 400

  it("sem alvo perto não mexe e não há guia", () => {
    expect(snapShift(r(0, 0, 50, 50), { x: 120, y: 250 }, [alvo], 8)).toEqual({
      x: 120,
      y: 250,
      guideX: null,
      guideY: null,
    });
  });

  it("gruda a borda esquerda na borda esquerda do alvo", () => {
    const s = snapShift(r(0, 0, 50, 50), { x: 297, y: 250 }, [alvo], 8);
    expect(s.x).toBe(300);
    expect(s.guideX).toBe(300);
  });

  it("gruda o centro no centro do alvo", () => {
    // retângulo de 50 de largura: centro em 25+x; alvo em 350 → x = 325
    const s = snapShift(r(0, 0, 50, 50), { x: 328, y: 250 }, [alvo], 8);
    expect(s.x).toBe(325);
    expect(s.guideX).toBe(350);
  });

  it("gruda a borda de baixo na de baixo do alvo (eixo y)", () => {
    // altura 60; bottom = y+60; alvo bottom = 200 → y = 140 (o topo só chegaria ao centro, 150, a 7 px)
    const s = snapShift(r(0, 0, 50, 60), { x: 120, y: 143 }, [alvo], 8);
    expect(s.y).toBe(140);
    expect(s.guideY).toBe(200);
  });

  it("escolhe o alvo mais próximo quando há vários", () => {
    const outro = r(310, 0, 20, 20);
    const s = snapShift(r(0, 0, 50, 50), { x: 309, y: 250 }, [alvo, outro], 8);
    expect(s.x).toBe(310); // esquerda do outro está a 1 px; a do primeiro, a 9
  });

  it("os eixos são independentes", () => {
    const s = snapShift(r(0, 0, 50, 50), { x: 299, y: 99 }, [alvo], 8);
    expect(s.x).toBe(300);
    expect(s.y).toBe(100);
  });

  it("limiar zero desliga o ímã", () => {
    expect(snapShift(r(0, 0, 50, 50), { x: 299, y: 99 }, [alvo], 0)).toEqual({
      x: 299,
      y: 99,
      guideX: null,
      guideY: null,
    });
  });
});
