import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, describe, expect, it } from "vitest";

const raiz = (arquivo: string) => join(process.cwd(), "..", arquivo);
const versao = /^version = "([^"]+)"$/m.exec(readFileSync(raiz("Cargo.toml"), "utf8"))![1]!;
const temporarios: string[] = [];
const pasta = () => {
  const p = mkdtempSync(join(tmpdir(), "vpad-release-"));
  temporarios.push(p);
  return p;
};
afterAll(() => temporarios.forEach((p) => rmSync(p, { recursive: true, force: true })));

const rodar = (script: string, args: string[]) => {
  const r = spawnSync(raiz(`scripts/${script}`), args, { encoding: "utf8" });
  return { codigo: r.status, saida: r.stdout, erro: r.stderr };
};

describe("scripts/notas-do-release.sh", () => {
  it("devolve a seção da versão no changelog, sem o título nem os links do rodapé", () => {
    const { codigo, saida } = rodar("notas-do-release.sh", ["0.1.0"]);
    expect(codigo).toBe(0);
    expect(saida).toContain("Primeira versão pública");
    expect(saida).toContain("### Adicionado");
    expect(saida).toContain("### Limitações conhecidas");
    expect(saida).not.toMatch(/^## \[/m);
    expect(saida).not.toContain("[Não lançado]:");
    expect(saida).not.toMatch(/^\[0\.1\.0\]:/m);
  });

  it("não vaza a seção de outra versão nem a 'Não lançado'", () => {
    const { saida } = rodar("notas-do-release.sh", ["0.1.0"]);
    expect(saida).not.toContain("Não lançado");
  });

  it("acrescenta como baixar o pacote do release e o link do changelog", () => {
    const { saida } = rodar("notas-do-release.sh", ["0.1.0"]);
    expect(saida).toContain("vpad-0.1.0-linux-x86_64.tar.gz");
    expect(saida).toContain(".sha256");
    expect(saida).toContain("https://github.com/Wpaif/vpad/blob/master/CHANGELOG.md");
  });

  it("falha, com mensagem, para uma versão que não está no changelog", () => {
    const r = rodar("notas-do-release.sh", ["9.9.9"]);
    expect(r.codigo).not.toBe(0);
    expect(r.erro).toMatch(/9\.9\.9/);
    expect(r.saida).toBe("");
  });

  it("falha sem argumento e com argumento que não é uma versão", () => {
    expect(rodar("notas-do-release.sh", []).codigo).not.toBe(0);
    for (const ruim of ["v0.1.0", "0.1", "abc", "0.1.0; ls", ""]) {
      expect(rodar("notas-do-release.sh", [ruim]).codigo, ruim).not.toBe(0);
    }
  });
});

describe("scripts/empacotar-release.sh", () => {
  const falso = (dir: string) => {
    const bin = join(dir, "vpad-falso");
    writeFileSync(bin, "#!/bin/sh\necho vpad-falso-ok\n");
    chmodSync(bin, 0o755);
    return bin;
  };

  it("gera o .tar.gz e o .sha256 com o nome da versão", () => {
    const dir = pasta();
    const r = rodar("empacotar-release.sh", [versao, falso(dir), join(dir, "saida")]);
    expect(r.codigo, r.erro).toBe(0);
    const nome = `vpad-${versao}-linux-x86_64`;
    expect(existsSync(join(dir, "saida", `${nome}.tar.gz`))).toBe(true);
    expect(existsSync(join(dir, "saida", `${nome}.tar.gz.sha256`))).toBe(true);
    expect(r.saida.trim()).toBe(join(dir, "saida", `${nome}.tar.gz`));
  });

  it("o pacote traz o binário, a licença, o changelog e o README nos seis idiomas", () => {
    const dir = pasta();
    rodar("empacotar-release.sh", [versao, falso(dir), join(dir, "saida")]);
    const nome = `vpad-${versao}-linux-x86_64`;
    const lista = execFileSync("tar", ["tzf", join(dir, "saida", `${nome}.tar.gz`)], { encoding: "utf8" }).split("\n");
    for (const arquivo of ["vpad", "LICENSE", "CHANGELOG.md", "README.md", "README.en.md", "README.es.md", "README.zh-CN.md", "README.ja.md", "README.ko.md"]) {
      expect(lista, arquivo).toContain(`${nome}/${arquivo}`);
    }
  });

  it("o binário do pacote continua executável e roda", () => {
    const dir = pasta();
    rodar("empacotar-release.sh", [versao, falso(dir), join(dir, "saida")]);
    const nome = `vpad-${versao}-linux-x86_64`;
    execFileSync("tar", ["xzf", join(dir, "saida", `${nome}.tar.gz`), "-C", dir]);
    expect(execFileSync(join(dir, nome, "vpad"), { encoding: "utf8" }).trim()).toBe("vpad-falso-ok");
  });

  it("o .sha256 confere com o pacote (sha256sum -c)", () => {
    const dir = pasta();
    rodar("empacotar-release.sh", [versao, falso(dir), join(dir, "saida")]);
    const r = spawnSync("sha256sum", ["-c", `vpad-${versao}-linux-x86_64.tar.gz.sha256`], { cwd: join(dir, "saida"), encoding: "utf8", env: { ...process.env, LC_ALL: "C" } }); // locale C: a palavra "OK" não depende do idioma do sistema
    expect(r.status, r.stderr).toBe(0);
    expect(r.stdout).toContain("OK");
  });

  it("o pacote não guarda o dono dos arquivos de quem o gerou (uid/gid 0)", () => {
    const dir = pasta();
    rodar("empacotar-release.sh", [versao, falso(dir), join(dir, "saida")]);
    const detalhe = execFileSync("tar", ["tvzf", join(dir, "saida", `vpad-${versao}-linux-x86_64.tar.gz`)], { encoding: "utf8" });
    for (const linha of detalhe.split("\n").filter(Boolean)) expect(linha, linha).toMatch(/\b0\/0\b/);
  });

  it("recusa uma versão diferente da do Cargo.toml, sem gerar nada", () => {
    const dir = pasta();
    const r = rodar("empacotar-release.sh", ["9.9.9", falso(dir), join(dir, "saida")]);
    expect(r.codigo).not.toBe(0);
    expect(r.erro).toContain(versao);
    expect(existsSync(join(dir, "saida"))).toBe(false);
  });

  it("recusa binário inexistente ou sem permissão de execução", () => {
    const dir = pasta();
    expect(rodar("empacotar-release.sh", [versao, join(dir, "nao-existe"), join(dir, "saida")]).codigo).not.toBe(0);
    const sem = join(dir, "sem-permissao");
    writeFileSync(sem, "x");
    chmodSync(sem, 0o644);
    expect(rodar("empacotar-release.sh", [versao, sem, join(dir, "saida")]).codigo).not.toBe(0);
  });

  it("recusa argumentos que faltam", () => {
    expect(rodar("empacotar-release.sh", []).codigo).not.toBe(0);
    expect(rodar("empacotar-release.sh", [versao]).codigo).not.toBe(0);
  });
});

describe("workflow de release", () => {
  const wf = () => readFileSync(raiz(".github/workflows/release.yml"), "utf8");

  it("roda ao criar uma tag v* e também manualmente, pedindo a tag", () => {
    expect(wf()).toMatch(/tags:\s*\n\s*- 'v\*'/);
    expect(wf()).toMatch(/workflow_dispatch:/);
    expect(wf()).toMatch(/inputs:\s*\n\s*tag:/);
  });

  it("começa só com leitura; escrever é permissão do job que publica", () => {
    expect(wf()).toMatch(/^permissions:\s*\n  contents: read$/m);
    expect(wf()).toMatch(/publicar:[\s\S]*?permissions:\s*\n\s+contents: write/);
    expect(wf().match(/contents: write/g)).toHaveLength(1);
  });

  it("a tag (de uma entrada manual ou do nome da ref) nunca é interpolada num comando", () => {
    for (const linha of wf().split("\n")) {
      if (!/\$\{\{\s*(inputs\.|github\.ref_name|github\.ref\b)/.test(linha)) continue;
      expect(linha, linha).toMatch(/^\s+(TAG|ref|name|path|if):/);
    }
  });

  it("publica só quando há uma tag para publicar", () => {
    expect(wf()).toMatch(/publicar:[\s\S]*?if: /);
  });

  it("compila com o lockfile, numa distribuição com glibc antiga, depois de construir a página", () => {
    expect(wf()).toContain("ubuntu-22.04");
    expect(wf()).toContain("cargo build --release --locked");
    expect(wf().indexOf("npm run build")).toBeLessThan(wf().indexOf("cargo build --release"));
  });

  it("usa os scripts testados e confere o binário antes de publicar", () => {
    expect(wf()).toContain("./scripts/empacotar-release.sh");
    expect(wf()).toContain("./scripts/notas-do-release.sh");
    expect(wf()).toMatch(/--dispositivos/);
  });
});
