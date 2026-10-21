import { describe, expect, it } from "vitest";
import { Axis, Button, decodeProfile, decodeProfileNotice, decodeRefusal, encodeAxis, encodeButton, encodePing, encodeProfile } from "./protocol";

describe("protocol", () => {
  it("codifica botão pressionado e solto", () => {
    expect([...encodeButton(Button.A, true)]).toEqual([1, 0, 1]);
    expect([...encodeButton(Button.Start, false)]).toEqual([1, 7, 0]);
  });

  it("direcional usa ids 10..13", () => {
    expect([...encodeButton(Button.Left, true)]).toEqual([1, 12, 1]);
  });

  it("codifica eixo em i16 little-endian", () => {
    expect([...encodeAxis(Axis.LX, 0x1234)]).toEqual([2, 0, 0x34, 0x12]);
    expect([...encodeAxis(Axis.RY, -1)]).toEqual([2, 3, 0xff, 0xff]);
  });

  it("limita o eixo ao intervalo i16", () => {
    expect([...encodeAxis(Axis.LX, 99999)]).toEqual([2, 0, 0xff, 0x7f]);
    expect([...encodeAxis(Axis.LX, -99999)]).toEqual([2, 0, 0x00, 0x80]);
  });

  it("gatilhos ficam entre 0 e 255", () => {
    expect([...encodeAxis(Axis.LT, 300)]).toEqual([2, 4, 255, 0]);
    expect([...encodeAxis(Axis.RT, -5)]).toEqual([2, 5, 0, 0]);
  });

  it("ping carrega timestamp de 8 bytes", () => {
    const msg = encodePing(1234.5);
    expect(msg.length).toBe(9);
    expect(msg[0]).toBe(3);
    expect(new DataView(msg.buffer).getFloat64(1, true)).toBe(1234.5);
  });

  it("codifica o pedido de perfil", () => {
    expect([...encodeProfile(2)]).toEqual([4, 2]);
  });

  it("decodifica o aviso de perfil do servidor", () => {
    expect(decodeProfile(Uint8Array.of(4, 1))).toBe(1);
  });

  it("ignora mensagens que não são aviso de perfil", () => {
    expect(decodeProfile(Uint8Array.of(3, 1))).toBeNull();
    expect(decodeProfile(Uint8Array.of(4))).toBeNull();
    expect(decodeProfile(Uint8Array.of(4, 1, 1))).toBeNull();
  });

  it("decodifica a recusa do servidor (token ausente, inválido ou revogado)", () => {
    expect(decodeRefusal(Uint8Array.of(5, 1))).toBe("unauthorized");
  });

  it("ignora o que não é recusa", () => {
    expect(decodeRefusal(Uint8Array.of(5, 2))).toBeNull(); // o antigo 'ocupado' não existe mais
    expect(decodeRefusal(Uint8Array.of(5, 9))).toBeNull();
    expect(decodeRefusal(Uint8Array.of(4, 1))).toBeNull();
    expect(decodeRefusal(Uint8Array.of(5))).toBeNull();
  });

  it("decodifica os avisos sobre o pedido de ativar o modo teclado", () => {
    expect(decodeProfileNotice(Uint8Array.of(6, 1))).toBe("no-permission");
    expect(decodeProfileNotice(Uint8Array.of(6, 2))).toBe("waiting");
    expect(decodeProfileNotice(Uint8Array.of(6, 3))).toBe("denied");
    expect(decodeProfileNotice(Uint8Array.of(6, 4))).toBe("expired");
    expect(decodeProfileNotice(Uint8Array.of(6, 5))).toBe("busy");
  });

  it("ignora o que não é aviso de perfil", () => {
    expect(decodeProfileNotice(Uint8Array.of(6, 9))).toBeNull();
    expect(decodeProfileNotice(Uint8Array.of(5, 1))).toBeNull();
    expect(decodeProfileNotice(Uint8Array.of(6))).toBeNull();
  });
});
