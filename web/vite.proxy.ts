import type { ProxyOptions } from "vite";

/** Porta HTTPS do vpad (a mesma de `--porta-https`), ou a de `VPAD_PORTA_HTTPS`. */
export function portaDoServidor(env: Record<string, string | undefined>): number {
  const texto = env.VPAD_PORTA_HTTPS;
  if (texto === undefined) return 8443;
  const porta = Number(texto);
  if (!/^\d+$/.test(texto) || porta < 1 || porta > 65535) {
    throw new Error(`VPAD_PORTA_HTTPS inválida: ${JSON.stringify(texto)} (use um número de 1 a 65535)`);
  }
  return porta;
}

/**
 * No `npm run dev` o Vite serve a página e repassa o WebSocket e a API ao servidor, que só
 * atende em HTTPS. Duas adaptações:
 * - `secure: false`: o certificado é de uma autoridade própria que o Node não conhece;
 * - `Origin` https: o servidor recusa qualquer outra (barreira contra páginas de fora), e a página
 *   do Vite é http. O `changeOrigin` faz o `Host` chegar como `localhost`, o que ele também exige.
 */
export function proxyDoServidor(porta: number): Record<string, ProxyOptions> {
  const rota = (ws: boolean): ProxyOptions => ({
    target: `https://localhost:${porta}`,
    ws,
    secure: false,
    changeOrigin: true,
    headers: { Origin: `https://localhost:${porta}` },
  });
  return { "/ws": rota(true), "/api": rota(false) };
}
