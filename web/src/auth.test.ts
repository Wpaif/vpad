import { beforeEach, describe, expect, it } from "vitest";
import { getClientId, readToken, safeStorage, wsUrl } from "./auth";

const TOKEN = "0123456789abcdef0123456789abcdef";
const OUTRO = "fedcba9876543210fedcba9876543210";

function memoria() {
  const dados = new Map<string, string>();
  return {
    dados,
    getItem: (k: string) => dados.get(k) ?? null,
    setItem: (k: string, v: string) => void dados.set(k, v),
  };
}

function lugar(search: string, hash = "") {
  return { pathname: "/", search, hash } as Location;
}

let storage: ReturnType<typeof memoria>;

beforeEach(() => {
  storage = memoria();
});

describe("readToken", () => {
  it("pega o token da URL do QR code e guarda", () => {
    expect(readToken(lugar(`?t=${TOKEN}`), storage)).toBe(TOKEN);
    expect(storage.getItem("vpad:token")).toBe(TOKEN);
  });

  it("acha o token entre outros parâmetros", () => {
    expect(readToken(lugar(`?debug&t=${TOKEN}&x=1`, "#a"), storage)).toBe(TOKEN);
  });

  it("sem token na URL usa o guardado (atalho da tela inicial)", () => {
    storage.setItem("vpad:token", TOKEN);
    expect(readToken(lugar(""), storage)).toBe(TOKEN);
  });

  it("o token da URL vale mais que o guardado e o substitui (token renovado)", () => {
    storage.setItem("vpad:token", OUTRO);
    expect(readToken(lugar(`?t=${TOKEN}`), storage)).toBe(TOKEN);
    expect(storage.getItem("vpad:token")).toBe(TOKEN);
  });

  it("token de formato estranho na URL é ignorado", () => {
    storage.setItem("vpad:token", OUTRO);
    expect(readToken(lugar("?t=<script>"), storage)).toBe(OUTRO);
    expect(readToken(lugar("?t=curto"), storage)).toBe(OUTRO);
  });

  it("sem nenhum token devolve null", () => {
    expect(readToken(lugar(""), storage)).toBeNull();
  });

  it("armazenamento bloqueado não impede de usar o token da URL", () => {
    const quebrado = {
      getItem: () => {
        throw new Error("bloqueado");
      },
      setItem: () => {
        throw new Error("bloqueado");
      },
    };
    expect(readToken(lugar(`?t=${TOKEN}`), quebrado)).toBe(TOKEN);
    expect(readToken(lugar(""), quebrado)).toBeNull();
  });
});

describe("getClientId", () => {
  const aleatorio = (n: number) => Uint8Array.from({ length: n }, (_, i) => i + 1);

  it("gera um id de 32 caracteres hexadecimais e o guarda", () => {
    const id = getClientId(storage, aleatorio);
    expect(id).toMatch(/^[0-9a-f]{32}$/);
    expect(storage.getItem("vpad:cliente")).toBe(id);
  });

  it("devolve sempre o mesmo id deste aparelho", () => {
    const primeiro = getClientId(storage, aleatorio);
    expect(getClientId(storage, () => new Uint8Array(16).fill(255))).toBe(primeiro);
  });

  it("armazenamento bloqueado ainda gera um id válido", () => {
    const quebrado = {
      getItem: () => {
        throw new Error("bloqueado");
      },
      setItem: () => {
        throw new Error("bloqueado");
      },
    };
    expect(getClientId(quebrado, aleatorio)).toMatch(/^[0-9a-f]{32}$/);
  });
});

describe("wsUrl", () => {
  it("monta a URL do WebSocket com token e id do aparelho", () => {
    const loc = { protocol: "http:", host: "192.168.68.103:8080" } as Location;
    expect(wsUrl(loc, TOKEN, "abc")).toBe(`ws://192.168.68.103:8080/ws?t=${TOKEN}&id=abc`);
  });

  it("usa wss em páginas https", () => {
    const loc = { protocol: "https:", host: "pc.local" } as Location;
    expect(wsUrl(loc, TOKEN, "abc")).toMatch(/^wss:\/\/pc\.local\/ws\?/);
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
      expect(s.getItem("b")).toBeNull();
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
