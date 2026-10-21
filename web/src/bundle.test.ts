import { existsSync, readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * Executa o JavaScript que o `vite build` gerou (e não o código-fonte): os testes de unidade
 * rodam com a configuração do Vitest, então só isto prova que o pacote de produção funciona,
 * por exemplo, que o Svelte entrou na versão de navegador e não na de servidor.
 * Sem um build feito (`npm run build`), o teste é pulado.
 */
const pasta = resolve("dist/assets");
const arquivo = existsSync(pasta) ? readdirSync(pasta).find((f) => f.endsWith(".js")) : undefined;

describe.skipIf(!arquivo)("bundle de produção", () => {
  it("monta o controle de verdade, com os sete grupos", async () => {
    // sem o <link> de CSS e o <script>: o happy-dom não carrega arquivos externos, e o JS é importado abaixo
    document.documentElement.innerHTML = readFileSync("dist/index.html", "utf8")
      .replace(/<link[^>]*rel="stylesheet"[^>]*>/g, "")
      .replace(/<script[^>]*><\/script>/g, "");
    await import(/* @vite-ignore */ pathToFileURL(resolve(pasta, arquivo!)).href);

    const pad = document.getElementById("pad");
    expect(pad, "o pad não foi montado pelo bundle").not.toBeNull();
    expect(pad!.querySelectorAll("[data-grupo]").length).toBe(7);
    expect(pad!.querySelectorAll("[data-btn],[data-trigger],[data-stick]").length).toBe(16);
  });
});
