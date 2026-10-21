import { describe, expect, it, vi } from "vitest";
import { loadKeyboard, saveKeyboard, type KeyboardConfig } from "./keyboard";

const CONFIG: KeyboardConfig = {
  mapa: { A: "KeyK" },
  padrao: { A: "KeyK", B: "KeyL" },
  controles: ["A", "B"],
  teclas: [{ id: "KeyK", rotulo: "K", grupo: "Letras" }],
};

function resposta(status: number, corpo: unknown): Response {
  return new Response(JSON.stringify(corpo), { status, headers: { "Content-Type": "application/json" } });
}

describe("loadKeyboard", () => {
  it("busca a configuração em /api/teclado", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(200, CONFIG));
    expect(await loadKeyboard("tok", fetchFn)).toEqual(CONFIG);
    expect(fetchFn).toHaveBeenCalledWith("/api/teclado", { headers: { Authorization: "Bearer tok" } });
  });

  it("401 pede para parear o aparelho de novo", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(401, { erro: "não autorizado" }));
    await expect(loadKeyboard("errado", fetchFn)).rejects.toThrow(/parear/i);
  });

  it("falha com mensagem clara se o servidor não responde ok", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(500, {}));
    await expect(loadKeyboard("tok", fetchFn)).rejects.toThrow(/500/);
  });
});

describe("saveKeyboard", () => {
  it("envia o mapa por PUT em JSON e devolve o que o servidor guardou", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(200, { mapa: { A: "Space" } }));
    expect(await saveKeyboard({ A: "Space" }, "tok", fetchFn)).toEqual({ A: "Space" });

    const [url, init] = fetchFn.mock.calls[0]!;
    expect(url).toBe("/api/teclado");
    expect(init.method).toBe("PUT");
    expect(init.headers["Content-Type"]).toBe("application/json");
    expect(init.headers.Authorization).toBe("Bearer tok");
    expect(JSON.parse(init.body)).toEqual({ mapa: { A: "Space" } });
  });

  it("repassa a mensagem de erro do servidor", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(400, { erro: "tecla desconhecida: Hyper" }));
    await expect(saveKeyboard({ A: "Hyper" }, "tok", fetchFn)).rejects.toThrow("tecla desconhecida: Hyper");
  });

  it("falha de rede vira erro legível", async () => {
    const fetchFn = vi.fn().mockRejectedValue(new TypeError("Failed to fetch"));
    await expect(saveKeyboard({}, "tok", fetchFn)).rejects.toThrow(/conex/i);
  });

  it("401 vira mensagem de acesso negado", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(401, { erro: "não autorizado" }));
    await expect(saveKeyboard({}, "errado", fetchFn)).rejects.toThrow("não autorizado");
  });
});
