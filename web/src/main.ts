import "./style.css";
import { getClientId, readToken, safeStorage, wsUrl } from "./auth";
import { connect } from "./connection";
import { bindController } from "./controller";
import { mountKeyEditor } from "./editor";
import { bindInstallHint, enterFullscreenOnTouch } from "./fullscreen";
import { loadKeyboard, saveKeyboard, type KeyboardConfig } from "./keyboard";
import { applyKeyboardLabels, applyProfileLabels } from "./labels";
import { createLatencyMeter } from "./latency";
import { mountProfilePicker } from "./picker";
import { PROFILES } from "./profiles";
import { encodeProfile } from "./protocol";
import {
  bindPanelToggle,
  bindVisibilityToggle,
  lockGestures,
  renderGate,
  renderStats,
  renderStatus,
} from "./ui";
import { bindAppHeight, showViewportDebug } from "./viewport";

const storage = safeStorage();
const pad = document.getElementById("pad")!;
const gate = document.getElementById("gate")!;

lockGestures();
enterFullscreenOnTouch(document);
bindAppHeight(window, document.documentElement);
document.getElementById("gate-retry")!.addEventListener("click", () => location.reload());

const standalone =
  (navigator as Navigator & { standalone?: boolean }).standalone === true ||
  matchMedia("(display-mode: fullscreen), (display-mode: standalone)").matches;
if (location.search.includes("debug")) showViewportDebug(document, window, standalone);
bindInstallHint(
  document.getElementById("install-hint")!,
  document.getElementById("install-hint-close")!,
  { standalone, fullscreenEnabled: document.fullscreenEnabled === true },
  storage,
);

const token = readToken(location, storage);
if (token === null) {
  // Sem token não há o que tentar: o servidor recusaria. O QR code do terminal o traz.
  renderGate(gate, "unauthorized", standalone);
} else {
  start(token, getClientId(storage));
}

function start(token: string, clientId: string) {
  const now = () => performance.timeOrigin + performance.now();
  const statusEl = document.getElementById("status")!;
  const statsEl = document.getElementById("stats")!;
  bindVisibilityToggle(document.getElementById("toggle")!, pad);

  // Rótulos dos botões: os do perfil ativo ou, no modo teclado, as teclas do mapa.
  let profileId: string | null = null;
  let keyboard: KeyboardConfig | null = null;

  function refreshLabels() {
    if (profileId === null) return;
    if (profileId !== "teclado") {
      applyProfileLabels(pad, profileId);
    } else if (keyboard) {
      applyKeyboardLabels(pad, keyboard.mapa, keyboard.teclas);
    } else {
      loadKeyboard(token)
        .then((config) => {
          keyboard = config;
          if (profileId === "teclado") refreshLabels();
        })
        .catch(() => {});
    }
  }

  const editor = mountKeyEditor(document.getElementById("key-editor")!, {
    load: () => loadKeyboard(token),
    save: (mapa) => saveKeyboard(mapa, token),
    onSaved: (_mapa, config) => {
      keyboard = config;
      refreshLabels();
    },
  });
  document.getElementById("open-key-editor")!.addEventListener("click", () => void editor.open());

  const picker = mountProfilePicker(document.getElementById("profile-picker")!, (i) =>
    link.send(encodeProfile(i)),
  );
  const meter = createLatencyMeter({ window: 30, timeoutMs: 2000 });

  let refresh: ReturnType<typeof setInterval> | undefined;
  const draw = () => renderStats(statsEl, meter.snapshot(now()));
  bindPanelToggle(statusEl, statsEl, (open) => {
    clearInterval(refresh);
    if (open) {
      draw();
      refresh = setInterval(draw, 500);
    }
  });

  const link = connect({
    url: wsUrl(location, token, clientId),
    now,
    onStatus: (status) => {
      if (status !== "open") {
        meter.reset();
        picker.setCurrent(null);
      }
      renderStatus(statusEl, status, null);
    },
    onPingSent: (ts) => meter.sent(ts),
    onProfile: (index) => {
      picker.setCurrent(index);
      profileId = PROFILES[index]?.id ?? null;
      refreshLabels();
    },
    onRefused: (reason) => renderGate(gate, reason),
    onLatency: (_ms, sentAt) => {
      const ms = meter.received(sentAt, now());
      if (ms !== null) renderStatus(statusEl, "open", Math.round(ms));
    },
  });

  bindController(pad, link.send);
}
