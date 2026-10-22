import { describe, expect, it } from "vitest";
import { portaDoServidor, proxyDoServidor } from "./vite.proxy";

describe("portaDoServidor", () => {
  it("usa a porta HTTPS padrão do vpad", () => {
    expect(portaDoServidor({})).toBe(8443);
  });

  it("aceita outra porta pela variável VPAD_PORTA_HTTPS", () => {
    expect(portaDoServidor({ VPAD_PORTA_HTTPS: "18443" })).toBe(18443);
  });

  it("recusa valor que não é uma porta, em vez de apontar para o lugar errado", () => {
    for (const ruim of ["abc", "0", "65536", "-1", "84.43", ""]) {
      expect(() => portaDoServidor({ VPAD_PORTA_HTTPS: ruim }), ruim).toThrow(/VPAD_PORTA_HTTPS/);
    }
  });
});

describe("proxyDoServidor", () => {
  const proxy = proxyDoServidor(8443);

  it("cobre o WebSocket e toda a API", () => {
    expect(Object.keys(proxy).sort()).toEqual(["/api", "/ws"]);
    expect(proxy["/ws"]!.ws).toBe(true);
  });

  it("fala com o servidor pelo HTTPS, que é onde o controle fica", () => {
    for (const rota of Object.values(proxy)) {
      expect(rota.target).toBe("https://localhost:8443");
    }
  });

  it("aceita o certificado da autoridade própria (o Node não a conhece)", () => {
    for (const rota of Object.values(proxy)) expect(rota.secure).toBe(false);
  });

  it("apresenta ao servidor uma Origin https, que é a única que ele aceita", () => {
    for (const rota of Object.values(proxy)) {
      expect(rota.headers).toEqual({ Origin: "https://localhost:8443" });
    }
  });

  it("o Host que chega ao servidor é sempre o dele (localhost), mesmo acessando o Vite pelo IP", () => {
    for (const rota of Object.values(proxy)) expect(rota.changeOrigin).toBe(true);
  });

  it("segue a porta pedida", () => {
    const outra = proxyDoServidor(18443);
    expect(outra["/ws"]!.target).toBe("https://localhost:18443");
    expect(outra["/api"]!.headers).toEqual({ Origin: "https://localhost:18443" });
  });
});
