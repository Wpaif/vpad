/** Geometria do arrastar: limites da tela e ímã nas bordas/centros de outros grupos. Tudo em px. */

export interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface Shift {
  x: number;
  y: number;
}

export interface Snapped extends Shift {
  /** coordenada da linha de alinhamento a desenhar, ou null se não grudou */
  guideX: number | null;
  guideY: number | null;
}

const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v));

/** Faz `rect` deslocado por `shift` caber em `bounds`. Se for maior, fica alinhado à esquerda/topo. */
export function limitShift(rect: Rect, shift: Shift, bounds: Rect): Shift {
  const axis = (lo: number, hi: number, bLo: number, bHi: number, s: number) =>
    hi - lo >= bHi - bLo ? bLo - lo : clamp(s, bLo - lo, bHi - hi);
  return {
    x: axis(rect.left, rect.right, bounds.left, bounds.right, shift.x),
    y: axis(rect.top, rect.bottom, bounds.top, bounds.bottom, shift.y),
  };
}

/** Melhor alinhamento (borda ou centro do retângulo nas bordas/centro de um alvo) dentro do limiar. */
function snapAxis(lo: number, hi: number, targets: Array<[number, number]>, threshold: number) {
  let best: { delta: number; guide: number } | null = null;
  const mine = [lo, (lo + hi) / 2, hi];
  for (const [tLo, tHi] of targets) {
    for (const t of [tLo, (tLo + tHi) / 2, tHi]) {
      for (const m of mine) {
        const delta = t - m;
        if (Math.abs(delta) <= threshold && (!best || Math.abs(delta) < Math.abs(best.delta))) {
          best = { delta, guide: t };
        }
      }
    }
  }
  return best;
}

/** Gruda o retângulo deslocado nos alvos próximos, cada eixo por si. `threshold` 0 desliga. */
export function snapShift(rect: Rect, shift: Shift, targets: Rect[], threshold: number): Snapped {
  if (threshold <= 0) return { ...shift, guideX: null, guideY: null };
  const moved = {
    left: rect.left + shift.x,
    right: rect.right + shift.x,
    top: rect.top + shift.y,
    bottom: rect.bottom + shift.y,
  };
  const sx = snapAxis(moved.left, moved.right, targets.map((t) => [t.left, t.right]), threshold);
  const sy = snapAxis(moved.top, moved.bottom, targets.map((t) => [t.top, t.bottom]), threshold);
  return {
    x: shift.x + (sx?.delta ?? 0),
    y: shift.y + (sy?.delta ?? 0),
    guideX: sx?.guide ?? null,
    guideY: sy?.guide ?? null,
  };
}
