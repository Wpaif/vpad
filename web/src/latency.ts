export interface LatencySnapshot {
  last: number | null;
  min: number | null;
  avg: number | null;
  max: number | null;
  /** média da variação entre amostras consecutivas */
  jitter: number | null;
  /** fração de pings sem resposta na janela, 0..1 */
  loss: number;
  samples: number[];
}

export interface LatencyMeter {
  /** Registra um ping enviado; `ts` é o timestamp que vai dentro dele. */
  sent: (ts: number) => void;
  /** Registra o pong de `ts`. Devolve a latência, ou null se for desconhecido ou tardio. */
  received: (ts: number, now: number) => number | null;
  snapshot: (now: number) => LatencySnapshot;
  reset: () => void;
}

interface Options {
  /** quantos resultados (respostas e perdas) entram nas estatísticas */
  window: number;
  /** depois disso sem pong, o ping conta como perdido */
  timeoutMs: number;
}

const round1 = (n: number) => Math.round(n * 10) / 10;

export function createLatencyMeter({ window, timeoutMs }: Options): LatencyMeter {
  const pending = new Set<number>();
  /** latência em ms, ou null para ping perdido */
  let outcomes: Array<number | null> = [];

  const push = (outcome: number | null) => {
    outcomes.push(outcome);
    if (outcomes.length > window) outcomes = outcomes.slice(-window);
  };

  const expire = (now: number) => {
    for (const ts of pending) {
      if (now - ts > timeoutMs) {
        pending.delete(ts);
        push(null);
      }
    }
  };

  return {
    sent: (ts) => void pending.add(ts),

    received(ts, now) {
      expire(now);
      if (!pending.delete(ts)) return null;
      const ms = now - ts;
      push(ms);
      return ms;
    },

    snapshot(now) {
      expire(now);
      const samples = outcomes.filter((o): o is number => o !== null);
      const lost = outcomes.length - samples.length;
      const diffs = samples.slice(1).map((v, i) => Math.abs(v - samples[i]!));
      return {
        last: samples.at(-1) ?? null,
        min: samples.length ? Math.min(...samples) : null,
        avg: samples.length ? round1(samples.reduce((a, b) => a + b, 0) / samples.length) : null,
        max: samples.length ? Math.max(...samples) : null,
        jitter: diffs.length ? round1(diffs.reduce((a, b) => a + b, 0) / diffs.length) : null,
        loss: outcomes.length ? lost / outcomes.length : 0,
        samples,
      };
    },

    reset() {
      pending.clear();
      outcomes = [];
    },
  };
}
