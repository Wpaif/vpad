import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const publico = (nome: string) => readFileSync(`public/${nome}`);

/** Lê o cabeçalho de um PNG: largura, altura e o tipo de cor (2 = RGB sem transparência). */
function png(nome: string) {
  const b = publico(nome);
  expect(b.subarray(0, 8).toString("hex"), `${nome} não é PNG`).toBe("89504e470d0a1a0a");
  return { w: b.readUInt32BE(16), h: b.readUInt32BE(20), profundidade: b[24], tipoDeCor: b[25] };
}

describe("ícones do app", () => {
  it.each([
    ["icon-192.png", 192],
    ["icon-512.png", 512],
    ["apple-touch-icon.png", 180],
    ["favicon-32.png", 32],
  ])("%s é um PNG quadrado de %d px", (nome, lado) => {
    const p = png(nome);
    expect([p.w, p.h]).toEqual([lado, lado]);
  });

  it("os PNGs da tela de início não têm transparência (o iOS pinta o fundo de preto)", () => {
    for (const nome of ["icon-192.png", "icon-512.png", "apple-touch-icon.png"]) {
      const p = png(nome);
      expect(p.tipoDeCor, `${nome} tem canal alfa`).toBe(2);
      expect(p.profundidade).toBe(8);
    }
  });

  it("o SVG é quadrado, de 512, e não carrega nada de fora (a CSP só deixa 'self' e data:)", () => {
    const svg = publico("icon.svg").toString("utf8");
    expect(svg).toMatch(/viewBox="0 0 512 512"/);
    expect(svg).not.toMatch(/<script|<foreignObject/i);
    expect(svg).not.toMatch(/href="https?:/i);
    expect(svg).not.toMatch(/xlink:href="(?!#)/i);
  });

  it("o SVG é bem formado", () => {
    const doc = new DOMParser().parseFromString(publico("icon.svg").toString("utf8"), "image/svg+xml");
    expect(doc.querySelector("parsererror")).toBeNull();
    expect(doc.documentElement.tagName).toBe("svg");
  });
});

describe("ligações do ícone", () => {
  const html = readFileSync("index.html", "utf8");
  const manifesto = JSON.parse(publico("manifest.webmanifest").toString("utf8")) as {
    icons: Array<{ src: string; sizes: string }>;
  };

  it("a página aponta para o SVG, o PNG de 32 px e o ícone da tela de início do iPhone", () => {
    expect(html).toMatch(/<link rel="icon" type="image\/svg\+xml" href="\/icon\.svg">/);
    expect(html).toMatch(/<link rel="icon" type="image\/png" sizes="32x32" href="\/favicon-32\.png">/);
    expect(html).toMatch(/<link rel="apple-touch-icon" href="\/apple-touch-icon\.png">/);
  });

  it("cada ícone do manifesto existe e tem o tamanho que anuncia", () => {
    for (const { src, sizes } of manifesto.icons) {
      const p = png(src.replace(/^\//, ""));
      expect(`${p.w}x${p.h}`, src).toBe(sizes);
    }
  });
});
