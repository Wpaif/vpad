import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";
import { portaDoServidor, proxyDoServidor } from "./vite.proxy";

export default defineConfig({
  plugins: [svelte()],
  server: { host: true, proxy: proxyDoServidor(portaDoServidor(process.env)) },
  // Nos testes o Svelte 5 precisa da versão de navegador (o padrão do Node é a de servidor). No
  // build NÃO se mexe em `resolve.conditions`: o Vite já usa "browser" no cliente, e zerar a lista
  // faria o pacote de produção levar o Svelte de servidor (bundle.test.ts cobre isso).
  ...(process.env.VITEST ? { resolve: { conditions: ["browser"] } } : {}),
  test: { environment: "happy-dom" },
});
