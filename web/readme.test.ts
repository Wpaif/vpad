import { existsSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const raiz = (arquivo: string) => readFileSync(`../${arquivo}`, "utf8");

describe("README", () => {
  const readme = raiz("README.md");

  it("diz de onde baixar o projeto, por HTTPS e por SSH", () => {
    expect(readme).toContain("git clone https://github.com/Wpaif/vpad.git");
    expect(readme).toContain("git clone git@github.com:Wpaif/vpad.git");
  });

  it("a instalação começa por baixar o projeto e entrar na pasta", () => {
    const instalacao = readme.slice(readme.indexOf("## Instalação"), readme.indexOf("## Primeiro uso"));
    expect(instalacao.indexOf("git clone")).toBeGreaterThan(-1);
    expect(instalacao.indexOf("cd vpad")).toBeGreaterThan(instalacao.indexOf("git clone"));
    expect(instalacao.indexOf("npm run build")).toBeGreaterThan(instalacao.indexOf("cd vpad"));
  });

  it("tem o selo do CI apontando para o workflow que existe", () => {
    expect(readme).toContain("https://github.com/Wpaif/vpad/actions/workflows/ci.yml/badge.svg");
    expect(existsSync("../.github/workflows/ci.yml")).toBe(true);
  });

  it("explica o que é o projeto logo no começo, antes do sumário", () => {
    const antes = readme.slice(0, readme.indexOf("## Sumário"));
    expect(antes).toMatch(/gamepad ou teclado virtual/i);
    expect(antes).toMatch(/Linux/);
  });

  it("menciona o CI na parte de desenvolvimento", () => {
    const dev = readme.slice(readme.indexOf("## Desenvolvimento"));
    expect(dev).toMatch(/\bCI\b|integração contínua/);
  });
});

describe("workflow de CI", () => {
  const ci = () => raiz(".github/workflows/ci.yml");

  it("roda nos pull requests e nos pushes na master", () => {
    expect(ci()).toMatch(/^on:/m);
    expect(ci()).toMatch(/pull_request:/);
    expect(ci()).toMatch(/push:\s*\n\s*branches: \[master\]/);
  });

  it("tem os três jobs, com os nomes que a proteção da branch vai exigir", () => {
    for (const job of ["web", "rust", "hooks"]) {
      expect(ci(), job).toMatch(new RegExp(`^  ${job}:\\s*\\n    name: ${job}$`, "m"));
    }
  });

  it("usa permissão mínima de leitura", () => {
    expect(ci()).toMatch(/^permissions:\s*\n  contents: read$/m);
  });

  it("o job de Rust espera a página (os testes do servidor leem web/dist)", () => {
    expect(ci()).toMatch(/rust:[\s\S]*?needs: web/);
    expect(ci()).toContain("download-artifact");
    expect(ci()).toContain("upload-artifact");
  });

  it("roda os mesmos comandos de make test, com o lockfile respeitado", () => {
    expect(ci()).toContain("npm ci");
    expect(ci()).toContain("npm run build");
    expect(ci()).toContain("npm test");
    expect(ci()).toContain("cargo test --locked");
    expect(ci()).toContain("./scripts/testar-hooks.sh");
  });

  it("cancela execuções antigas da mesma branch", () => {
    expect(ci()).toMatch(/cancel-in-progress: true/);
  });
});
