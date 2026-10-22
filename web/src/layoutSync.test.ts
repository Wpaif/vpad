import { beforeEach, describe, expect, it } from "vitest";
import { defaultLayout, withGroup, type Layout } from "./layoutModel";
import type { PushResult, RemoteLayout } from "./layoutRemote";
import { createLayoutSync, type SyncState } from "./layoutSync";

const local = withGroup(defaultLayout(), "face", { dx: 0.1 });
const remoto = withGroup(defaultLayout(), "menu", { dy: 0.2 });

let saved: Layout;
let editing: boolean;
let pending: boolean;
let adopted: Layout[];
let states: SyncState[];
let fetched: RemoteLayout;
let pushResult: PushResult;
let pushed: Layout[];

const store = {
  get saved() {
    return saved;
  },
  get editing() {
    return editing;
  },
  adopt(l: Layout) {
    if (editing) return false;
    saved = l;
    adopted.push(l);
    return true;
  },
};

const sync = () =>
  createLayoutSync({
    store,
    token: "t",
    fetchRemote: async () => fetched,
    pushRemote: async (l) => {
      pushed.push(l);
      return pushResult;
    },
    isPending: () => pending,
    setPending: (p) => void (pending = p),
    onState: (s) => states.push(s),
  });

beforeEach(() => {
  saved = defaultLayout();
  editing = false;
  pending = false;
  adopted = [];
  states = [];
  pushed = [];
  fetched = { status: "ok", layout: null };
  pushResult = "ok";
});

describe("start (ao abrir o app)", () => {
  it("adota o layout do PC quando é diferente do local", async () => {
    saved = local;
    fetched = { status: "ok", layout: remoto };
    await sync().start();
    expect(adopted).toEqual([remoto]);
    expect(pushed).toEqual([]);
    expect(states.at(-1)).toBe("synced");
  });

  it("não adota nada quando já é igual", async () => {
    saved = local;
    fetched = { status: "ok", layout: local };
    await sync().start();
    expect(adopted).toEqual([]);
    expect(states.at(-1)).toBe("synced");
  });

  it("PC sem layout e local padrão: nada a fazer", async () => {
    await sync().start();
    expect(pushed).toEqual([]);
    expect(adopted).toEqual([]);
    expect(states.at(-1)).toBe("none");
  });

  it("PC sem layout e local personalizado: envia o local (migração)", async () => {
    saved = local;
    await sync().start();
    expect(pushed).toEqual([local]);
    expect(states.at(-1)).toBe("synced");
    expect(pending).toBe(false);
  });

  it("migração que falha por falta de rede fica pendente", async () => {
    saved = local;
    pushResult = "offline";
    await sync().start();
    expect(pending).toBe(true);
    expect(states.at(-1)).toBe("pending");
  });

  it("com envio pendente, o local vence: envia em vez de adotar", async () => {
    saved = local;
    pending = true;
    fetched = { status: "ok", layout: remoto };
    await sync().start();
    expect(pushed).toEqual([local]);
    expect(adopted).toEqual([]);
    expect(pending).toBe(false);
    expect(states.at(-1)).toBe("synced");
  });

  it("pendente que ainda não consegue enviar continua pendente", async () => {
    saved = local;
    pending = true;
    pushResult = "error";
    await sync().start();
    expect(pending).toBe(true);
    expect(states.at(-1)).toBe("pending");
  });

  it("durante a edição não troca o layout (o adopt é recusado)", async () => {
    saved = local;
    editing = true;
    fetched = { status: "ok", layout: remoto };
    await sync().start();
    expect(saved).toBe(local);
  });

  it("sem conexão com o PC mantém o local e o estado anterior", async () => {
    saved = local;
    fetched = { status: "offline" };
    await sync().start();
    expect(adopted).toEqual([]);
    expect(pushed).toEqual([]);
    expect(states.at(-1)).toBe("none");
  });

  it("sem conexão e com envio pendente mostra pendente", async () => {
    pending = true;
    saved = local;
    pushResult = "offline";
    await sync().start();
    expect(states.at(-1)).toBe("pending");
  });

  it("aparelho não reconhecido não altera nada (a conexão trata o pareamento)", async () => {
    saved = local;
    fetched = { status: "unauthorized" };
    await sync().start();
    expect(adopted).toEqual([]);
    expect(pushed).toEqual([]);
  });
});

describe("saved (depois de o usuário tocar em Salvar)", () => {
  it("marca pendente, envia o salvo e confirma", async () => {
    saved = local;
    const s = sync();
    const p = s.saved();
    expect(pending).toBe(true); // marcado antes da resposta: sobrevive a fechar o app no meio
    await p;
    expect(pushed).toEqual([local]);
    expect(pending).toBe(false);
    expect(states.at(-1)).toBe("synced");
  });

  it("sem rede deixa pendente", async () => {
    saved = local;
    pushResult = "offline";
    await sync().saved();
    expect(pending).toBe(true);
    expect(states.at(-1)).toBe("pending");
  });

  it("recusado pelo PC não fica tentando de novo", async () => {
    saved = local;
    pushResult = "rejected";
    await sync().saved();
    expect(pending).toBe(false);
    expect(states.at(-1)).toBe("rejected");
  });

  it("não autorizado deixa pendente (volta quando o aparelho for pareado de novo)", async () => {
    saved = local;
    pushResult = "unauthorized";
    await sync().saved();
    expect(pending).toBe(true);
  });
});

describe("refresh (voltar ao app ou à rede)", () => {
  it("faz o mesmo que start", async () => {
    saved = local;
    fetched = { status: "ok", layout: remoto };
    await sync().refresh();
    expect(adopted).toEqual([remoto]);
  });

  it("chamadas seguidas não se atropelam: saved e refresh rodam em ordem", async () => {
    saved = local;
    fetched = { status: "ok", layout: remoto };
    const s = sync();
    const ordem: string[] = [];
    const a = s.saved().then(() => ordem.push("saved"));
    const b = s.refresh().then(() => ordem.push("refresh"));
    await Promise.all([a, b]);
    expect(ordem).toEqual(["saved", "refresh"]);
    // o saved enviou o local e limpou a pendência; só depois o refresh viu o PC
    expect(pushed).toEqual([local]);
  });
});
