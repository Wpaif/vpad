import { decodePing, decodeProfile, encodePing } from "./protocol";

export type Status = "connecting" | "open" | "reconnecting" | "closed";

export interface Options {
  url: string;
  onStatus: (status: Status) => void;
  onLatency: (ms: number, sentAt: number) => void;
  onPingSent?: (ts: number) => void;
  /** Índice do perfil ativo no servidor, avisado ao conectar e a cada troca. */
  onProfile?: (index: number) => void;
  WebSocketImpl?: typeof WebSocket;
  pingEveryMs?: number;
  now?: () => number;
}

export interface Connection {
  /** Envia se o socket estiver aberto; entrada velha não vale a pena enfileirar. */
  send: (msg: Uint8Array<ArrayBuffer>) => boolean;
  close: () => void;
}

const BACKOFF_MS = [500, 1000, 2000, 4000];
const OPEN = 1;

export function connect(opts: Options): Connection {
  const Impl = opts.WebSocketImpl ?? WebSocket;
  const pingEvery = opts.pingEveryMs ?? 1000;
  const now = opts.now ?? (() => performance.timeOrigin + performance.now());

  let socket: WebSocket | null = null;
  let pingTimer: ReturnType<typeof setInterval> | undefined;
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  let failures = 0;
  let stopped = false;

  const open = () => {
    const ws = new Impl(opts.url);
    ws.binaryType = "arraybuffer";
    socket = ws;

    ws.onopen = () => {
      failures = 0;
      opts.onStatus("open");
      pingTimer = setInterval(() => {
        const ts = now();
        ws.send(encodePing(ts));
        opts.onPingSent?.(ts);
      }, pingEvery);
    };
    ws.onmessage = (e) => {
      const data = new Uint8Array(e.data as ArrayBuffer);
      const sentAt = decodePing(data);
      if (sentAt !== null) {
        opts.onLatency(Math.round(now() - sentAt), sentAt);
        return;
      }
      const profile = decodeProfile(data);
      if (profile !== null) opts.onProfile?.(profile);
    };
    ws.onclose = () => {
      if (ws !== socket) return;
      clearInterval(pingTimer);
      if (stopped) return;
      opts.onStatus("reconnecting");
      const wait = BACKOFF_MS[Math.min(failures, BACKOFF_MS.length - 1)]!;
      failures++;
      retryTimer = setTimeout(open, wait);
    };
    ws.onerror = () => ws.close();
    opts.onStatus(failures === 0 && !retryTimer ? "connecting" : "reconnecting");
  };

  open();

  return {
    send(msg) {
      if (socket?.readyState !== OPEN) return false;
      socket.send(msg);
      return true;
    },
    close() {
      stopped = true;
      clearInterval(pingTimer);
      clearTimeout(retryTimer);
      socket?.close();
      opts.onStatus("closed");
    },
  };
}
