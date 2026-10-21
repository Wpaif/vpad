import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const manifest = JSON.parse(readFileSync("public/manifest.webmanifest", "utf8")) as Record<string, unknown>;

describe("manifest do app", () => {
  it("abre sempre na raiz, sem segredo na URL (o acesso vem do pareamento feito dentro do app)", () => {
    expect(manifest.start_url).toBe("/");
    expect(JSON.stringify(manifest)).not.toMatch(/[?&]t=/);
  });

  it("continua abrindo em tela cheia, na horizontal", () => {
    expect(manifest.display).toBe("fullscreen");
    expect(manifest.orientation).toBe("landscape");
  });
});
