import { existsSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const raiz = (arquivo: string) => readFileSync(`../${arquivo}`, "utf8");
const SEMVER = /^\d+\.\d+\.\d+$/;

/** A versão do pacote vpad no Cargo.toml. */
const versaoDoCargo = () => /^version = "([^"]+)"$/m.exec(raiz("Cargo.toml"))![1]!;

/** Versões e datas das seções `## [x.y.z] - AAAA-MM-DD` do changelog, da mais nova para a mais antiga. */
const versoesDoChangelog = () =>
  [...raiz("CHANGELOG.md").matchAll(/^## \[(\d+\.\d+\.\d+)\] - (\d{4}-\d{2}-\d{2})$/gm)].map((m) => ({ versao: m[1]!, data: m[2]! }));

describe("versão do projeto", () => {
  it("usa versionamento semântico", () => {
    expect(versaoDoCargo()).toMatch(SEMVER);
  });

  it("o Cargo.lock tem a mesma versão do Cargo.toml", () => {
    const m = /name = "vpad"\nversion = "([^"]+)"/.exec(raiz("Cargo.lock"));
    expect(m?.[1]).toBe(versaoDoCargo());
  });

  it("o package.json da página tem a mesma versão", () => {
    const pacote = JSON.parse(readFileSync("package.json", "utf8")) as { version?: string };
    expect(pacote.version).toBe(versaoDoCargo());
  });

  it("o package-lock.json da página tem a mesma versão (na raiz e no pacote)", () => {
    const trava = JSON.parse(readFileSync("package-lock.json", "utf8")) as {
      version?: string;
      packages: Record<string, { version?: string }>;
    };
    expect(trava.version).toBe(versaoDoCargo());
    expect(trava.packages[""]?.version).toBe(versaoDoCargo());
  });
});

describe("CHANGELOG", () => {
  it("existe e segue o formato 'Keep a Changelog'", () => {
    expect(existsSync("../CHANGELOG.md")).toBe(true);
    const c = raiz("CHANGELOG.md");
    expect(c).toMatch(/^# Changelog$/m);
    expect(c).toContain("keepachangelog.com");
    expect(c).toContain("semver.org");
  });

  it("a versão mais nova do changelog é a versão do projeto", () => {
    const versoes = versoesDoChangelog();
    expect(versoes.length).toBeGreaterThan(0);
    expect(versoes[0]!.versao).toBe(versaoDoCargo());
  });

  it("as versões estão em ordem decrescente, sem repetição, com datas válidas", () => {
    const versoes = versoesDoChangelog();
    const numeros = versoes.map((v) => v.versao.split(".").map(Number));
    for (let i = 1; i < numeros.length; i++) {
      const [a, b] = [numeros[i - 1]!, numeros[i]!];
      const decrescente = a[0]! > b[0]! || (a[0] === b[0] && (a[1]! > b[1]! || (a[1] === b[1] && a[2]! > b[2]!)));
      expect(decrescente, `${versoes[i - 1]!.versao} deve vir antes de ${versoes[i]!.versao}`).toBe(true);
    }
    for (const { data } of versoes) expect(Number.isNaN(Date.parse(data)), data).toBe(false);
  });

  it("a versão do lançamento tem as seções Adicionado e Limitações conhecidas", () => {
    const c = raiz("CHANGELOG.md");
    const secao = c.slice(c.indexOf(`## [${versaoDoCargo()}]`));
    expect(secao).toMatch(/^### Adicionado$/m);
    expect(secao).toMatch(/^### Limitações conhecidas$/m);
  });

  it("cada versão tem o link de comparação/tag no rodapé", () => {
    const c = raiz("CHANGELOG.md");
    for (const { versao } of versoesDoChangelog()) {
      expect(c, versao).toMatch(new RegExp(`^\\[${versao.replace(/\./g, "\\.")}\\]: https://github\\.com/Wpaif/vpad/`, "m"));
    }
  });

  it("o README em português aponta para o changelog", () => {
    expect(raiz("README.md")).toContain("[CHANGELOG.md](CHANGELOG.md)");
  });

  it("o changelog registra o binário pré-compilado em 'Não lançado' e não diz mais que ele não existe", () => {
    const c = raiz("CHANGELOG.md");
    const naoLancado = c.slice(c.indexOf("## [Não lançado]"), c.indexOf("## [0.1.0]"));
    expect(naoLancado).toMatch(/^### Adicionado$/m);
    expect(naoLancado).toMatch(/linux-x86_64/);
    expect(c).not.toMatch(/Não há binário pré-compilado/);
  });

  it("o changelog registra o binário ARM em 'Não lançado'", () => {
    const c = raiz("CHANGELOG.md");
    const naoLancado = c.slice(c.indexOf("## [Não lançado]"), c.indexOf("## [0.1.0]"));
    expect(naoLancado).toMatch(/aarch64/);
  });
});
