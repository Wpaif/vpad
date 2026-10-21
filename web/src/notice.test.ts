import { describe, expect, it } from "vitest";
import { describeNotice } from "./notice";

describe("describeNotice", () => {
  it("sem permissão: diz o comando que o dono do PC deve usar, com o id deste aparelho", () => {
    const texto = describeNotice("no-permission", "ab12cd34");
    expect(texto).toMatch(/permissão/i);
    expect(texto).toContain("vpad --permitir-teclado ab12cd34");
  });

  it("sem permissão e sem saber o id: manda ver a lista no PC", () => {
    const texto = describeNotice("no-permission", null);
    expect(texto).toContain("vpad --dispositivos");
    expect(texto).toContain("--permitir-teclado");
  });

  it("aguardando: pede para olhar o terminal do PC", () => {
    expect(describeNotice("waiting", null)).toMatch(/PC/);
    expect(describeNotice("waiting", null)).toMatch(/confirm/i);
  });

  it("negado, expirou e ocupado têm texto próprio", () => {
    expect(describeNotice("denied", null)).toMatch(/recusad|negad/i);
    expect(describeNotice("expired", null)).toMatch(/a tempo|expir/i);
    expect(describeNotice("busy", null)).toMatch(/outro pedido|ocupad/i);
  });

  it("o texto nunca carrega HTML: o id entra como texto simples", () => {
    expect(describeNotice("no-permission", "<b>x</b>")).not.toContain("<b>");
  });
});
