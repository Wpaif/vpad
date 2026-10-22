import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, describe, expect, it } from "vitest";

const raiz = (arquivo: string) => join(process.cwd(), "..", arquivo);
const versao = /^version = "([^"]+)"$/m.exec(readFileSync(raiz("Cargo.toml"), "utf8"))![1]!;
const ARQUITETURAS = ["x86_64", "aarch64"] as const;
type Arq = (typeof ARQUITETURAS)[number];
const ELF_MACHINE: Record<Arq, number> = { x86_64: 0x3e, aarch64: 0xb7 };
const arquiteturaDoComputador = (): Arq => (process.arch === "arm64" ? "aarch64" : "x86_64");

const temporarios: string[] = [];
const pasta = () => {
  const p = mkdtempSync(join(tmpdir(), "vpad-release-"));
  temporarios.push(p);
  return p;
};
afterAll(() => temporarios.forEach((p) => rmSync(p, { recursive: true, force: true })));

const rodar = (script: string, args: string[], env: Record<string, string> = {}) => {
  const r = spawnSync(raiz(`scripts/${script}`), args, { encoding: "utf8", env: { ...process.env, ...env } });
  return { codigo: r.status, saida: r.stdout, erro: r.stderr };
};

/** Um "binário" que só tem o cabeçalho ELF mínimo, com a arquitetura pedida: basta para o script conferir. */
const elfFalso = (dir: string, arq: Arq, nome = `elf-${arq}`) => {
  const cabecalho = Buffer.alloc(64);
  cabecalho.set([0x7f, 0x45, 0x4c, 0x46, 2, 1, 1], 0); // magia ELF, 64 bits, little endian, versão 1
  cabecalho.writeUInt16LE(ELF_MACHINE[arq], 18); // e_machine
  const caminho = join(dir, nome);
  writeFileSync(caminho, cabecalho);
  chmodSync(caminho, 0o755);
  return caminho;
};

/** Um binário de verdade (o `true` do sistema), da arquitetura deste computador, que pode ser executado. */
const binarioReal = () => ["/usr/bin/true", "/bin/true"].find((c) => existsSync(c))!;

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

  it("acrescenta como baixar o pacote de cada arquitetura, a soma e o link do changelog", () => {
    const { saida } = rodar("notas-do-release.sh", ["0.1.0"]);
    expect(saida).toContain("vpad-0.1.0-linux-x86_64.tar.gz");
    expect(saida).toContain("vpad-0.1.0-linux-aarch64.tar.gz");
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

describe("scripts/arquiteturas-do-release.sh", () => {
  const matriz = (entrada: string[]) => {
    const r = rodar("arquiteturas-do-release.sh", entrada);
    return { ...r, json: r.codigo === 0 ? (JSON.parse(r.saida) as { include: Array<{ arch: string; runner: string }> }) : null };
  };

  it("sem entrada, ou 'todas', monta a matriz com as duas arquiteturas, cada uma no seu runner nativo", () => {
    for (const entrada of [[], [""], ["todas"]]) {
      const { codigo, json } = matriz(entrada);
      expect(codigo, JSON.stringify(entrada)).toBe(0);
      expect(json!.include).toEqual([
        { arch: "x86_64", runner: "ubuntu-22.04" },
        { arch: "aarch64", runner: "ubuntu-22.04-arm" },
      ]);
    }
  });

  it("uma arquitetura só devolve só ela (para anexar uma a um release que já tem a outra)", () => {
    expect(matriz(["aarch64"]).json!.include).toEqual([{ arch: "aarch64", runner: "ubuntu-22.04-arm" }]);
    expect(matriz(["x86_64"]).json!.include).toEqual([{ arch: "x86_64", runner: "ubuntu-22.04" }]);
  });

  it("a saída é uma linha só (cabe numa saída de job do GitHub)", () => {
    expect(rodar("arquiteturas-do-release.sh", []).saida.trim().split("\n")).toHaveLength(1);
  });

  it("recusa o que não é uma arquitetura conhecida, inclusive tentativas de injeção", () => {
    for (const ruim of ["riscv64", "arm", "x86_64,aarch64", "x86_64; ls", '"}]}', "$(id)", "TODAS"]) {
      const r = rodar("arquiteturas-do-release.sh", [ruim]);
      expect(r.codigo, ruim).not.toBe(0);
      expect(r.saida, ruim).toBe("");
    }
  });
});

describe("scripts/empacotar-release.sh", () => {
  const pacote = (dir: string, arq: Arq = "x86_64") => join(dir, "saida", `vpad-${versao}-linux-${arq}.tar.gz`);

  it("gera o .tar.gz e o .sha256 com o nome da versão e da arquitetura (x86_64 é o padrão)", () => {
    const dir = pasta();
    const r = rodar("empacotar-release.sh", [versao, elfFalso(dir, "x86_64"), join(dir, "saida")]);
    expect(r.codigo, r.erro).toBe(0);
    expect(existsSync(pacote(dir))).toBe(true);
    expect(existsSync(`${pacote(dir)}.sha256`)).toBe(true);
    expect(r.saida.trim()).toBe(pacote(dir));
  });

  it.each(ARQUITETURAS)("empacota para %s quando o binário é dessa arquitetura", (arq) => {
    const dir = pasta();
    const r = rodar("empacotar-release.sh", [versao, elfFalso(dir, arq), join(dir, "saida"), arq]);
    expect(r.codigo, r.erro).toBe(0);
    expect(existsSync(pacote(dir, arq))).toBe(true);
    const lista = execFileSync("tar", ["tzf", pacote(dir, arq)], { encoding: "utf8" }).split("\n");
    expect(lista).toContain(`vpad-${versao}-linux-${arq}/vpad`);
  });

  it("o pacote traz o binário, a licença, o changelog e o README nos seis idiomas", () => {
    const dir = pasta();
    rodar("empacotar-release.sh", [versao, elfFalso(dir, "x86_64"), join(dir, "saida")]);
    const nome = `vpad-${versao}-linux-x86_64`;
    const lista = execFileSync("tar", ["tzf", pacote(dir)], { encoding: "utf8" }).split("\n");
    for (const arquivo of ["vpad", "LICENSE", "CHANGELOG.md", "README.md", "README.en.md", "README.es.md", "README.zh-CN.md", "README.ja.md", "README.ko.md"]) {
      expect(lista, arquivo).toContain(`${nome}/${arquivo}`);
    }
  });

  it("o binário do pacote continua executável e roda (um binário real, da arquitetura deste computador)", () => {
    const dir = pasta();
    const arq = arquiteturaDoComputador();
    const r = rodar("empacotar-release.sh", [versao, binarioReal(), join(dir, "saida"), arq]);
    expect(r.codigo, r.erro).toBe(0);
    execFileSync("tar", ["xzf", pacote(dir, arq), "-C", dir]);
    expect(spawnSync(join(dir, `vpad-${versao}-linux-${arq}`, "vpad")).status).toBe(0);
  });

  it("o .sha256 confere com o pacote (sha256sum -c)", () => {
    const dir = pasta();
    rodar("empacotar-release.sh", [versao, elfFalso(dir, "aarch64"), join(dir, "saida"), "aarch64"]);
    const r = spawnSync("sha256sum", ["-c", `vpad-${versao}-linux-aarch64.tar.gz.sha256`], {
      cwd: join(dir, "saida"),
      encoding: "utf8",
      env: { ...process.env, LC_ALL: "C" }, // locale C: a palavra "OK" não depende do idioma do sistema
    });
    expect(r.status, r.stderr).toBe(0);
    expect(r.stdout).toContain("OK");
  });

  it("o pacote não guarda o dono dos arquivos de quem o gerou (uid/gid 0)", () => {
    const dir = pasta();
    rodar("empacotar-release.sh", [versao, elfFalso(dir, "x86_64"), join(dir, "saida")]);
    const detalhe = execFileSync("tar", ["tvzf", pacote(dir)], { encoding: "utf8" });
    for (const linha of detalhe.split("\n").filter(Boolean)) expect(linha, linha).toMatch(/\b0\/0\b/);
  });

  it("recusa um binário de OUTRA arquitetura (evita publicar um pacote com o binário trocado)", () => {
    const dir = pasta();
    const r = rodar("empacotar-release.sh", [versao, elfFalso(dir, "x86_64"), join(dir, "saida"), "aarch64"]);
    expect(r.codigo).not.toBe(0);
    expect(r.erro).toContain("aarch64");
    expect(r.erro).toContain("x86_64");
    expect(existsSync(join(dir, "saida"))).toBe(false);
  });

  it("recusa um arquivo que não é um binário ELF", () => {
    const dir = pasta();
    const script = join(dir, "script");
    writeFileSync(script, "#!/bin/sh\necho oi\n");
    chmodSync(script, 0o755);
    const r = rodar("empacotar-release.sh", [versao, script, join(dir, "saida")]);
    expect(r.codigo).not.toBe(0);
    expect(r.erro).toMatch(/ELF/);
    expect(existsSync(join(dir, "saida"))).toBe(false);
  });

  it("recusa arquitetura desconhecida ou com cara de injeção", () => {
    const dir = pasta();
    for (const ruim of ["riscv64", "arm", "x86_64/../x", "x86_64; ls", ""]) {
      expect(rodar("empacotar-release.sh", [versao, elfFalso(dir, "x86_64"), join(dir, "saida"), ruim]).codigo, ruim).not.toBe(0);
    }
    expect(existsSync(join(dir, "saida"))).toBe(false);
  });

  it("recusa uma versão diferente da do Cargo.toml, sem gerar nada", () => {
    const dir = pasta();
    const r = rodar("empacotar-release.sh", ["9.9.9", elfFalso(dir, "x86_64"), join(dir, "saida")]);
    expect(r.codigo).not.toBe(0);
    expect(r.erro).toContain(versao);
    expect(existsSync(join(dir, "saida"))).toBe(false);
  });

  it("recusa binário inexistente ou sem permissão de execução", () => {
    const dir = pasta();
    expect(rodar("empacotar-release.sh", [versao, join(dir, "nao-existe"), join(dir, "saida")]).codigo).not.toBe(0);
    const sem = elfFalso(dir, "x86_64", "sem-permissao");
    chmodSync(sem, 0o644);
    expect(rodar("empacotar-release.sh", [versao, sem, join(dir, "saida")]).codigo).not.toBe(0);
  });

  it("recusa argumentos que faltam", () => {
    expect(rodar("empacotar-release.sh", []).codigo).not.toBe(0);
    expect(rodar("empacotar-release.sh", [versao]).codigo).not.toBe(0);
  });
});

describe("scripts/versao-do-release.sh", () => {
  it("de uma tag vX.Y.Z devolve X.Y.Z", () => {
    expect(rodar("versao-do-release.sh", ["v0.1.0"]).saida.trim()).toBe("0.1.0");
    expect(rodar("versao-do-release.sh", ["v12.3.45"]).saida.trim()).toBe("12.3.45");
  });

  it("de qualquer outra coisa (nome de branch, vazio, tag malformada, tentativa de injeção) usa a versão do Cargo.toml", () => {
    for (const nome of ["master", "", "v1.2", "v0.1.0-rc1", "0.1.0", "v0.1.0; ls", "$(id)"]) {
      const r = rodar("versao-do-release.sh", nome === "" ? [] : [nome]);
      expect(r.codigo, nome).toBe(0);
      expect(r.saida.trim(), nome).toBe(versao);
    }
  });
});

/** Os comandos (`run:`) de um workflow, um texto por passo, para checar o que roda no shell. */
const comandosDe = (yaml: string) => {
  const linhas = yaml.split("\n");
  const comandos: string[] = [];
  for (let i = 0; i < linhas.length; i++) {
    const m = /^(\s*)(?:- )?run:\s*(.*)$/.exec(linhas[i]!);
    if (!m) continue;
    const recuo = m[1]!.length + (linhas[i]!.includes("- run:") ? 2 : 0);
    let texto = /^[|>]-?$/.test(m[2]!) ? "" : m[2]!;
    while (i + 1 < linhas.length && (linhas[i + 1]!.trim() === "" || linhas[i + 1]!.search(/\S/) > recuo)) texto += `\n${linhas[++i]}`;
    comandos.push(texto);
  }
  return comandos;
};

describe("workflow de release", () => {
  const wf = () => readFileSync(raiz(".github/workflows/release.yml"), "utf8");

  it("roda ao criar uma tag v* e também manualmente, pedindo a tag, a fonte e as arquiteturas", () => {
    expect(wf()).toMatch(/tags:\s*\n\s*- 'v\*'/);
    expect(wf()).toMatch(/workflow_dispatch:/);
    for (const entrada of ["tag", "fonte", "arquiteturas"]) expect(wf(), entrada).toMatch(new RegExp(`inputs:[\\s\\S]*?\\n      ${entrada}:`));
  });

  it("começa só com leitura; escrever é permissão do job que publica", () => {
    expect(wf()).toMatch(/^permissions:\s*\n  contents: read$/m);
    expect(wf()).toMatch(/publicar:[\s\S]*?permissions:\s*\n\s+contents: write/);
    expect(wf().match(/contents: write/g)).toHaveLength(1);
  });

  it("nenhuma expressão ${{ }} entra num comando de shell (tag, fonte, arquiteturas e matriz vão por variável de ambiente)", () => {
    const comandos = comandosDe(wf());
    expect(comandos.length).toBeGreaterThan(5);
    for (const c of comandos) expect(c, c).not.toContain("${{");
  });

  it("uma entrada da interface nunca vai direto para um comando: só para `env`, `with` e `if`", () => {
    for (const linha of wf().split("\n")) {
      if (!/\$\{\{\s*(inputs\.|github\.ref_name|github\.ref\b)/.test(linha)) continue;
      expect(linha, linha).toMatch(/^\s+(TAG|FONTE|ARQUITETURAS|ref|name|path|if):/);
    }
  });

  it("um job 'plano' decide a matriz com o script testado, e o empacotamento roda uma vez por arquitetura", () => {
    expect(wf()).toMatch(/^  plano:/m);
    expect(wf()).toContain("./scripts/arquiteturas-do-release.sh");
    expect(wf()).toMatch(/empacotar:[\s\S]*?needs: plano/);
    expect(wf()).toMatch(/matrix: \$\{\{ fromJSON\(needs\.plano\.outputs\.matriz\) \}\}/);
    expect(wf()).toMatch(/runs-on: \$\{\{ matrix\.runner \}\}/);
    expect(wf()).toMatch(/fail-fast: false/);
  });

  it("cada arquitetura compila no seu runner (nativo, sem compilação cruzada) e entrega um artefato com o nome dela", () => {
    expect(wf()).not.toMatch(/cross|--target/);
    expect(wf()).toMatch(/name: pacote-\$\{\{ matrix\.arch \}\}/);
  });

  it("o job que publica junta os artefatos de todas as arquiteturas", () => {
    expect(wf()).toMatch(/pattern: pacote-\*/);
    expect(wf()).toMatch(/merge-multiple: true/);
  });

  it("publica só quando há uma tag para publicar", () => {
    expect(wf()).toMatch(/publicar:[\s\S]*?if: /);
  });

  it("aceita uma 'fonte': compilar de outra ref, para anexar o pacote a uma tag anterior aos scripts de release", () => {
    expect(wf()).toMatch(/ref: \$\{\{ inputs\.fonte \|\| inputs\.tag \|\| github\.ref \}\}/);
  });

  it("a versão vem do script testado, e o teste sem tag usa a versão do Cargo.toml", () => {
    expect(wf()).toContain("./scripts/versao-do-release.sh");
    expect(wf()).not.toMatch(/\$\{TAG#v\}/);
  });

  it("compila com o lockfile, numa distribuição com glibc antiga, depois de construir a página", () => {
    expect(wf()).toContain("ubuntu-22.04");
    expect(wf()).toContain("cargo build --release --locked");
    expect(wf().indexOf("npm run build")).toBeLessThan(wf().indexOf("cargo build --release"));
  });

  it("usa os scripts testados e confere o binário, rodando-o, antes de publicar", () => {
    expect(wf()).toContain("./scripts/empacotar-release.sh");
    expect(wf()).toContain("./scripts/notas-do-release.sh");
    expect(wf()).toMatch(/--dispositivos/);
    expect(wf()).toMatch(/ARQ: \$\{\{ matrix\.arch \}\}/);
  });
});

describe("workflow de CI em ARM", () => {
  const ci = () => readFileSync(raiz(".github/workflows/ci.yml"), "utf8");

  it("roda os testes do Rust também num runner ARM nativo, depois de construir a página", () => {
    expect(ci()).toMatch(/^  rust-arm:\s*\n    name: rust-arm$/m);
    expect(ci()).toMatch(/rust-arm:[\s\S]*?runs-on: ubuntu-22\.04-arm/);
    expect(ci()).toMatch(/rust-arm:[\s\S]*?needs: web/);
    expect(ci()).toMatch(/rust-arm:[\s\S]*?cargo test --locked/);
  });

  it("os três checks obrigatórios continuam com os mesmos nomes", () => {
    for (const job of ["web", "rust", "hooks"]) expect(ci(), job).toMatch(new RegExp(`^  ${job}:\\s*\\n    name: ${job}$`, "m"));
  });
});
