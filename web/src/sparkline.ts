const fmt = (n: number) => String(Math.round(n * 10) / 10);

/** Pontos de uma polyline SVG: maior latência = mais alto; `ceiling` é o topo do gráfico. */
export function sparklinePoints(samples: number[], width: number, height: number, ceiling: number): string {
  if (samples.length === 0) return "";
  const y = (v: number) => height - Math.min(v / ceiling, 1) * height;
  if (samples.length === 1) return `0,${fmt(y(samples[0]!))} ${width},${fmt(y(samples[0]!))}`;
  return samples
    .map((v, i) => `${fmt((i / (samples.length - 1)) * width)},${fmt(y(v))}`)
    .join(" ");
}
