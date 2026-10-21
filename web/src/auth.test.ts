import { beforeEach, describe, expect, it } from "vitest";
import { clearToken, getDeviceId, getToken, safeStorage, saveDeviceId, saveToken, wsUrl } from "./auth";

const TOKEN = "0123456789abcdef0123456789abcdef";

function memoria() {
  const dados = new Map<string, string>();
  return {
    dados,
    getItem: (k: string) => dados.get(k) ?? null,
    setItem: (k: string, v: string) => void dados.set(k, v),
    removeItem: (k: string) => void dados.delete(k),
  };
}

const quebrado = {
  getItem: () => {
    throw new Error("bloqueado");
  },
  setItem: () => {
    throw new Error("bloqueado");
  },
  removeItem: () => {
    throw new Error("bloqueado");
  },
};

let storage: ReturnType<typeof memoria>;
beforeEach(() => {
  storage = memoria();
});

describe("token deste aparelho", () => {
  it("sem pareamento não há token", () => {
    expect(getToken(storage)).toBeNull();
  });

  it("guarda o token recebido no pareamento e o devolve depois", () => {
    saveToken(storage, TOKEN);
    expect(getToken(storage)).toBe(TOKEN);
  });

  it("ignora um valor guardado que não parece um token", () => {
    storage.setItem("vpad:token", "<script>");
    expect(getToken(storage)).toBeNull();
    storage.setItem("vpad:token", "curto");
    expect(getToken(storage)).toBeNull();
  });

  it("esquecer o token faz o aparelho voltar a precisar de pareamento", () => {
    saveToken(storage, TOKEN);
    clearToken(storage);
    expect(getToken(storage)).toBeNull();
  });

  it("armazenamento bloqueado não gera erro", () => {
    expect(getToken(quebrado)).toBeNull();
    expect(() => saveToken(quebrado, TOKEN)).not.toThrow();
    expect(() => clearToken(quebrado)).not.toThrow();
  });
});

describe("id do aparelho (para o dono do PC liberar o teclado)", () => {
  it("guarda o id recebido no pareamento", () => {
    expect(getDeviceId(storage)).toBeNull();
    saveDeviceId(storage, "ab12cd34");
    expect(getDeviceId(storage)).toBe("ab12cd34");
  });

  it("ignora um valor guardado que não parece um id", () => {
    storage.setItem("vpad:id", "<script>");
    expect(getDeviceId(storage)).toBeNull();
  });

  it("esquecer o token esquece também o id", () => {
    saveToken(storage, TOKEN);
    saveDeviceId(storage, "ab12cd34");
    clearToken(storage);
    expect(getDeviceId(storage)).toBeNull();
  });

  it("armazenamento bloqueado não gera erro", () => {
    expect(getDeviceId(quebrado)).toBeNull();
    expect(() => saveDeviceId(quebrado, "ab12cd34")).not.toThrow();
  });
});

describe("wsUrl", () => {
  it("monta a URL do WebSocket com o token do aparelho", () => {
    const loc = { protocol: "http:", host: "192.168.68.103:8080" } as Location;
    expect(wsUrl(loc, TOKEN)).toBe(`ws://192.168.68.103:8080/ws?t=${TOKEN}`);
  });

  it("usa wss em páginas https", () => {
    const loc = { protocol: "https:", host: "pc.local" } as Location;
    expect(wsUrl(loc, TOKEN)).toMatch(/^wss:\/\/pc\.local\/ws\?t=/);
  });
});

describe("safeStorage", () => {
  it("se o navegador bloqueia o localStorage, devolve um armazenamento em memória que funciona", () => {
    const original = Object.getOwnPropertyDescriptor(window, "localStorage")!;
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      get() {
        throw new Error("SecurityError");
      },
    });
    try {
      const s = safeStorage();
      s.setItem("a", "1");
      expect(s.getItem("a")).toBe("1");
      s.removeItem("a");
      expect(s.getItem("a")).toBeNull();
    } finally {
      Object.defineProperty(window, "localStorage", original);
    }
  });

  it("com localStorage disponível, usa ele", () => {
    const s = safeStorage();
    s.setItem("vpad:teste", "ok");
    expect(window.localStorage.getItem("vpad:teste")).toBe("ok");
    window.localStorage.removeItem("vpad:teste");
  });
});
