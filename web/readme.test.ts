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

// ---------- traduções do README ----------

const IDIOMAS = [
  { arquivo: "README.md", nome: "Português" },
  { arquivo: "README.en.md", nome: "English" },
  { arquivo: "README.es.md", nome: "Español" },
  { arquivo: "README.zh-CN.md", nome: "中文" },
  { arquivo: "README.ja.md", nome: "日本語" },
  { arquivo: "README.ko.md", nome: "한국어" },
] as const;
const TRADUCOES = IDIOMAS.slice(1);

/** Seções do README em português que o usuário final lê (as de contribuidor ficam só em português). */
const SO_EM_PORTUGUES = ["Estrutura do projeto", "Desenvolvimento"];

const secoes = (md: string) => {
  const mapa = new Map<string, string>();
  const partes = md.split(/^## /m).slice(1);
  for (const p of partes) mapa.set(p.slice(0, p.indexOf("\n")).trim(), p.slice(p.indexOf("\n") + 1));
  return mapa;
};
const doUsuario = (md: string) =>
  [...secoes(md)].filter(([nome]) => !SO_EM_PORTUGUES.includes(nome) && nome !== "Sumário").map(([, corpo]) => corpo).join("\n");

const blocosDeCodigo = (md: string) => [...md.matchAll(/```[a-z]*\n([\s\S]*?)```/g)].map((m) => m[1]!);
const linhasDeComando = (blocos: string[]) =>
  blocos.flatMap((b) => b.split("\n").map((l) => l.trim()).filter((l) => l && !l.startsWith("#")));
const semCodigo = (md: string) => md.replace(/```[\s\S]*?```/g, "").replace(/`[^`\n]*`/g, "");
const ancora = (titulo: string) =>
  titulo.toLowerCase().replace(/[^\p{L}\p{N}\p{M}\s-]/gu, "").trim().replace(/\s/g, "-");
const tabelas = (md: string) =>
  md.split("\n\n").filter((bloco) => bloco.trimStart().startsWith("|")).map((bloco) => bloco.trim().split("\n").length);

describe("traduções do README", () => {
  const pt = raiz("README.md");

  it.each(TRADUCOES)("$arquivo existe", ({ arquivo }) => {
    expect(existsSync(`../${arquivo}`), arquivo).toBe(true);
  });

  it.each(IDIOMAS)("$arquivo tem a barra de idiomas, com o idioma atual em negrito e os outros com link", ({ arquivo, nome }) => {
    const barra = raiz(arquivo).split("\n").find((l) => l.includes("README.zh-CN.md") || l.includes("**中文**"));
    expect(barra, arquivo).toBeDefined();
    const esperada = IDIOMAS.map((i) => (i.arquivo === arquivo ? `**${i.nome}**` : `[${i.nome}](${i.arquivo})`)).join(" · ");
    expect(barra, `${arquivo} (${nome})`).toBe(esperada);
  });

  it.each(IDIOMAS)("$arquivo tem os selos de CI e de licença", ({ arquivo }) => {
    const md = raiz(arquivo);
    expect(md).toContain("https://github.com/Wpaif/vpad/actions/workflows/ci.yml/badge.svg");
    expect(md).toContain("](LICENSE)");
  });

  it.each(TRADUCOES)("$arquivo tem as mesmas seções do usuário que o português (13 no total, sem a estrutura do projeto)", ({ arquivo }) => {
    expect([...secoes(raiz(arquivo)).keys()]).toHaveLength(13);
    expect([...secoes(pt).keys()]).toHaveLength(14);
  });

  it.each(IDIOMAS)("$arquivo: todo link do sumário aponta para um título que existe", ({ arquivo }) => {
    const md = raiz(arquivo);
    const titulos = new Set([...secoes(md).keys()].map(ancora));
    const sumario = [...(secoes(md).get(arquivo === "README.md" ? "Sumário" : [...secoes(md).keys()][0]!) ?? "").matchAll(/\]\(#([^)]+)\)/g)].map((m) => m[1]!);
    expect(sumario.length, arquivo).toBeGreaterThanOrEqual(11);
    for (const alvo of sumario) expect(titulos.has(alvo), `${arquivo}: #${alvo}`).toBe(true);
  });

  it.each(TRADUCOES)("$arquivo: os comandos dos blocos de código são os mesmos do português", ({ arquivo }) => {
    const esperados = linhasDeComando(blocosDeCodigo(doUsuario(pt)));
    const reais = new Set(linhasDeComando(blocosDeCodigo(raiz(arquivo))));
    expect(esperados.length).toBeGreaterThan(8);
    for (const linha of esperados) expect(reais.has(linha), `${arquivo}: ${linha}`).toBe(true);
  });

  it.each(TRADUCOES)("$arquivo: opções, arquivos, caminhos e comandos em `código` não são traduzidos", ({ arquivo }) => {
    const md = raiz(arquivo);
    const comando = /^(--[a-z-]+|vpad\b.*|make .*|\.\/.*|~\/.*|\$XDG.*|https?:.*|[\w.-]+\.(json|pem)|tls\/|\/dev\/.*|\/etc\/.*)$/;
    const tokens = [...doUsuario(pt).matchAll(/`([^`\n]+)`/g)].map((m) => m[1]!).filter((t) => comando.test(t) && !t.includes("IP-DO-PC")); // o nome do espaço reservado é traduzido; as portas são conferidas à parte
    expect(new Set(tokens).size).toBeGreaterThan(20);
    for (const t of new Set(tokens)) expect(md.includes(t), `${arquivo}: \`${t}\``).toBe(true);
  });

  it.each(TRADUCOES)("$arquivo: as tabelas têm as mesmas linhas do português", ({ arquivo }) => {
    expect(tabelas(doUsuario(raiz(arquivo)))).toEqual(tabelas(doUsuario(pt)));
    expect(tabelas(doUsuario(pt)).length).toBeGreaterThanOrEqual(4);
  });

  it.each(TRADUCOES)("$arquivo: os números que importam não mudam (portas, bloqueios, validade)", ({ arquivo }) => {
    const md = raiz(arquivo);
    for (const n of ["8080", "8443", "1800", "300", "6 digit|6 dígitos|6 位|6 桁|6자리", "90"]) {
      expect(new RegExp(n).test(md), `${arquivo}: ${n}`).toBe(true);
    }
  });

  it.each(TRADUCOES)("$arquivo: não sobra texto em português (fora de código e da barra de idiomas)", ({ arquivo }) => {
    const texto = semCodigo(raiz(arquivo)).split("\n").filter((l) => !l.includes("README.zh-CN.md") && !l.includes("**中文**")).join("\n");
    for (const palavra of ["você", "não ", "aparelho", "Instalação", "Segurança", "Pareamento"]) {
      expect(texto.includes(palavra), `${arquivo}: "${palavra}"`).toBe(false);
    }
  });

  it.each(TRADUCOES)("$arquivo: a seção de desenvolvimento é curta e aponta para o README em português", ({ arquivo }) => {
    const md = raiz(arquivo);
    const nomes = [...secoes(md).keys()];
    const dev = secoes(md).get(nomes[nomes.length - 3]!)!; // antes de "problemas comuns" e "licença"
    for (const trecho of ["make test", "gh pr create", "make hooks", "(README.md)"]) {
      expect(dev.includes(trecho), `${arquivo}: ${trecho}`).toBe(true);
    }
    expect(dev.split("\n").length).toBeLessThan(25);
  });

  it.each(IDIOMAS)("$arquivo: todo link relativo aponta para um arquivo que existe", ({ arquivo }) => {
    const alvos = [...raiz(arquivo).matchAll(/\]\(([^)#]+)\)/g)].map((m) => m[1]!).filter((a) => !/^https?:/.test(a));
    expect(alvos.length).toBeGreaterThan(2);
    for (const alvo of alvos) expect(existsSync(`../${alvo}`), `${arquivo}: ${alvo}`).toBe(true);
  });
});
