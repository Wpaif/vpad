import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const raiz = (arquivo: string) => readFileSync(`../${arquivo}`, "utf8");

describe("licença do projeto", () => {
  it("o arquivo LICENSE é a MIT, em nome de Wilian", () => {
    const texto = raiz("LICENSE");
    expect(texto).toMatch(/^MIT License\n/);
    expect(texto).toMatch(/^Copyright \(c\) 2026 Wilian$/m);
    expect(texto).toContain("Permission is hereby granted, free of charge, to any person obtaining a copy");
    expect(texto).toContain('THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND');
  });

  it("o Cargo.toml declara a mesma licença", () => {
    expect(raiz("Cargo.toml")).toMatch(/^license = "MIT"$/m);
  });

  it("o package.json da página declara a mesma licença", () => {
    const pacote = JSON.parse(readFileSync("package.json", "utf8")) as { license?: string };
    expect(pacote.license).toBe("MIT");
  });

  it("o README tem a seção Licença apontando para o arquivo", () => {
    const readme = raiz("README.md");
    expect(readme).toMatch(/^## Licença$/m);
    expect(readme).toContain("[LICENSE](LICENSE)");
    expect(readme).toMatch(/- \[Licença\]\(#licença\)/); // e consta no sumário
  });
});
