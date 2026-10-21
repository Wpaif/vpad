import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const manifest = JSON.parse(readFileSync("public/manifest.webmanifest", "utf8")) as Record<string, unknown>;

describe("manifest do app", () => {
  it("não fixa start_url: o atalho da Tela de Início guarda a URL de onde foi criado, com o token", () => {
    // Com start_url "/", o ícone abriria sem o token e o app (que tem armazenamento próprio no iOS)
    // mostraria "acesso não autorizado".
    expect(manifest).not.toHaveProperty("start_url");
  });

  it("continua abrindo em tela cheia, na horizontal", () => {
    expect(manifest.display).toBe("fullscreen");
    expect(manifest.orientation).toBe("landscape");
  });
});
