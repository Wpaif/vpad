import { describe, expect, it } from "vitest";
import { createLatencyMeter } from "./latency";

const meter = (opts = {}) => createLatencyMeter({ window: 10, timeoutMs: 2000, ...opts });

/** Envia um ping em `t` e recebe o pong `ms` depois. */
function roundTrip(m: ReturnType<typeof meter>, t: number, ms: number) {
  m.sent(t);
  return m.received(t, t + ms);
}

describe("createLatencyMeter", () => {
  it("sem amostras tudo é nulo e a perda é zero", () => {
    expect(meter().snapshot(0)).toEqual({
      last: null, min: null, avg: null, max: null, jitter: null, loss: 0, samples: [],
    });
  });

  it("calcula último, mínimo, média e máximo", () => {
    const m = meter();
    roundTrip(m, 0, 10);
    roundTrip(m, 1000, 30);
    roundTrip(m, 2000, 20);
    const s = m.snapshot(3000);
    expect(s).toMatchObject({ last: 20, min: 10, avg: 20, max: 30, samples: [10, 30, 20] });
  });

  it("received devolve a latência do pong", () => {
    expect(roundTrip(meter(), 0, 17)).toBe(17);
  });

  it("jitter é a média da variação entre amostras consecutivas", () => {
    const m = meter();
    roundTrip(m, 0, 10);
    roundTrip(m, 1000, 20);
    roundTrip(m, 2000, 10);
    expect(m.snapshot(3000).jitter).toBe(10);
  });

  it("jitter precisa de pelo menos duas amostras", () => {
    const m = meter();
    roundTrip(m, 0, 10);
    expect(m.snapshot(1000).jitter).toBeNull();
  });

  it("ping sem resposta só conta como perdido depois do timeout", () => {
    const m = meter();
    roundTrip(m, 0, 10);
    m.sent(1000);
    expect(m.snapshot(2500).loss).toBe(0);
    expect(m.snapshot(3001).loss).toBe(0.5);
  });

  it("pong atrasado depois do timeout é ignorado", () => {
    const m = meter();
    m.sent(0);
    m.snapshot(2500);
    expect(m.received(0, 2600)).toBeNull();
    expect(m.snapshot(2600).samples).toEqual([]);
    expect(m.snapshot(2600).loss).toBe(1);
  });

  it("pong de ping desconhecido é ignorado", () => {
    expect(meter().received(123, 130)).toBeNull();
  });

  it("pong repetido não conta duas vezes", () => {
    const m = meter();
    roundTrip(m, 0, 10);
    expect(m.received(0, 20)).toBeNull();
    expect(m.snapshot(100).samples).toEqual([10]);
  });

  it("mantém só os últimos N resultados", () => {
    const m = meter({ window: 3 });
    [5, 6, 7, 8, 9].forEach((ms, i) => roundTrip(m, i * 1000, ms));
    expect(m.snapshot(9000).samples).toEqual([7, 8, 9]);
  });

  it("perdas antigas saem da janela", () => {
    const m = meter({ window: 2 });
    m.sent(0);
    m.snapshot(2500); // perdido
    roundTrip(m, 3000, 10);
    roundTrip(m, 4000, 10);
    expect(m.snapshot(5000).loss).toBe(0);
  });

  it("reset limpa tudo, inclusive pings pendentes", () => {
    const m = meter();
    roundTrip(m, 0, 10);
    m.sent(1000);
    m.reset();
    expect(m.received(1000, 1010)).toBeNull();
    expect(m.snapshot(1010).samples).toEqual([]);
  });
});
