import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { connect, type Status } from "./connection";
import { encodePing } from "./protocol";

class FakeSocket {
  static instances: FakeSocket[] = [];
  static OPEN = 1;
  readyState = 0;
  sent: Uint8Array[] = [];
  binaryType = "";
  onopen: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onmessage: ((e: { data: ArrayBuffer }) => void) | null = null;
  onerror: (() => void) | null = null;
  constructor(public url: string) {
    FakeSocket.instances.push(this);
  }
  send(d: Uint8Array) {
    this.sent.push(d);
  }
  close() {
    this.readyState = 3;
    this.onclose?.();
  }
  open() {
    this.readyState = 1;
    this.onopen?.();
  }
  receive(d: Uint8Array) {
    this.onmessage?.({ data: d.buffer.slice(d.byteOffset, d.byteOffset + d.byteLength) as ArrayBuffer });
  }
}

const last = () => FakeSocket.instances.at(-1)!;
let statuses: Status[];
let latencies: number[];

function start() {
  return connect({
    url: "ws://pc/ws",
    WebSocketImpl: FakeSocket as unknown as typeof WebSocket,
    onStatus: (s) => statuses.push(s),
    onLatency: (ms) => latencies.push(ms),
    pingEveryMs: 1000,
    now: () => Date.now(),
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  FakeSocket.instances = [];
  statuses = [];
  latencies = [];
});
afterEach(() => vi.useRealTimers());

describe("connect", () => {
  it("passa de connecting para open", () => {
    start();
    expect(statuses).toEqual(["connecting"]);
    last().open();
    expect(statuses).toEqual(["connecting", "open"]);
  });

  it("send só envia com o socket aberto e descarta o resto", () => {
    const c = start();
    c.send(Uint8Array.of(1, 0, 1));
    expect(last().sent).toEqual([]);
    last().open();
    c.send(Uint8Array.of(1, 0, 1));
    expect(last().sent).toEqual([Uint8Array.of(1, 0, 1)]);
  });

  it("send devolve se a mensagem foi enviada", () => {
    const c = start();
    expect(c.send(Uint8Array.of(1))).toBe(false);
    last().open();
    expect(c.send(Uint8Array.of(1))).toBe(true);
  });

  it("reconecta com espera crescente depois de cair", () => {
    start();
    last().open();
    last().close();
    expect(statuses.at(-1)).toBe("reconnecting");
    expect(FakeSocket.instances.length).toBe(1);

    vi.advanceTimersByTime(500);
    expect(FakeSocket.instances.length).toBe(2);

    last().close(); // falha de novo, sem ter aberto
    vi.advanceTimersByTime(500);
    expect(FakeSocket.instances.length).toBe(2); // espera maior agora
    vi.advanceTimersByTime(600);
    expect(FakeSocket.instances.length).toBe(3);
  });

  it("zera a espera depois de reconectar com sucesso", () => {
    start();
    last().close();
    vi.advanceTimersByTime(500);
    last().open();
    last().close();
    vi.advanceTimersByTime(500);
    expect(FakeSocket.instances.length).toBe(3);
  });

  it("envia ping periódico e calcula a latência pelo eco", () => {
    start();
    last().open();
    vi.advanceTimersByTime(1000);
    const ping = last().sent.at(-1)!;
    expect(ping[0]).toBe(3);
    expect(ping.length).toBe(9);

    vi.advanceTimersByTime(23);
    last().receive(ping);
    expect(latencies).toEqual([23]);
  });

  it("ignora eco que não é ping", () => {
    start();
    last().open();
    last().receive(Uint8Array.of(1, 2, 3));
    last().receive(encodePing(0).slice(0, 5));
    expect(latencies).toEqual([]);
  });

  it("para de enviar ping quando cai", () => {
    start();
    last().open();
    const socket = last();
    socket.close();
    vi.advanceTimersByTime(900);
    expect(socket.sent).toEqual([]);
  });

  it("close() encerra de vez, sem reconectar", () => {
    const c = start();
    last().open();
    c.close();
    vi.advanceTimersByTime(60_000);
    expect(FakeSocket.instances.length).toBe(1);
    expect(statuses.at(-1)).toBe("closed");
  });
});
