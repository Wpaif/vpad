import { mount } from "svelte";
import "./style.css";
import Pad from "./Pad.svelte";
import { clearToken, getDeviceId, getToken, safeStorage, saveDeviceId, saveToken, wsProtocols, wsUrl } from "./auth";
import { connect } from "./connection";
import { bindController } from "./controller";
import { mountKeyEditor } from "./editor";
import { bindInstallHint, enterFullscreenOnTouch } from "./fullscreen";
import { createLayoutStore } from "./layoutStore.svelte";
import { loadKeyboard, saveKeyboard, type KeyboardConfig } from "./keyboard";
import { applyKeyboardLabels, applyProfileLabels } from "./labels";
import { createLatencyMeter } from "./latency";
import { deviceName, mountPairing, pairDevice } from "./pairing";
import { describeNotice } from "./notice";
import { mountProfilePicker } from "./picker";
import { loadShortPing, pingIntervalFor, saveShortPing } from "./pingMode";
import { mountPingToggle } from "./pingToggle";
import { PROFILES } from "./profiles";
import { encodeProfile } from "./protocol";
import {
  bindPanelToggle,
  bindVisibilityToggle,
  lockGestures,
  renderStats,
  renderStatus,
} from "./ui";
import { bindAppHeight, showViewportDebug } from "./viewport";

const layoutStore = createLayoutStore();
mount(Pad, {
  target: document.getElementById("app")!,
  props: {
    get layout() {
      return layoutStore.current;
    },
  },
});

const storage = safeStorage();
const pad = document.getElementById("pad")!;

lockGestures();
enterFullscreenOnTouch(document);
bindAppHeight(window, document.documentElement);

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

// Cada aparelho se pareia uma vez, digitando o código que o terminal do PC mostra, e guarda o
// token que recebe. O app da Tela de Início tem armazenamento próprio: ele pareia por conta própria.
const pairing = mountPairing(document.getElementById("pair")!, {
  pair: (code) => pairDevice(code, deviceName(navigator.userAgent, navigator.maxTouchPoints)),
  onPaired: ({ token, id }) => {
    saveToken(storage, token);
    if (id) saveDeviceId(storage, id);
    pairing.hide();
    start(token);
  },
});

const saved = getToken(storage);
if (saved === null) pairing.show();
else start(saved);

function start(token: string) {
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

  const noteEl = document.getElementById("profile-note")!;
  const showNote = (text: string) => {
    noteEl.textContent = text; // textContent: nada do servidor vira HTML
  };

  const picker = mountProfilePicker(document.getElementById("profile-picker")!, (i) =>
    link.send(encodeProfile(i)),
  );
  mountPingToggle(document.getElementById("ping-toggle")!, loadShortPing(storage), (on) => {
    saveShortPing(storage, on);
    link.setPingEvery(pingIntervalFor(on));
  });
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
    pingEveryMs: pingIntervalFor(loadShortPing(storage)),
    url: wsUrl(location),
    protocols: wsProtocols(token),
    now,
    onStatus: (status) => {
      if (status !== "open") {
        meter.reset();
        picker.setCurrent(null);
      }
      renderStatus(statusEl, status, null);
    },
    onPingSent: (ts) => meter.sent(ts),
    // O modo teclado digita no PC: sem permissão o servidor recusa, e com ela o PC pede confirmação.
    onProfileNotice: (notice) => {
      showNote(describeNotice(notice, getDeviceId(storage)));
      if (notice === "waiting") picker.waitingForPc();
      else picker.cancelPending();
    },
    onProfile: (index) => {
      showNote("");
      picker.setCurrent(index);
      profileId = PROFILES[index]?.id ?? null;
      refreshLabels();
    },
    // O servidor não reconhece mais este aparelho (revogado): esquece o token e volta ao pareamento.
    onRefused: () => {
      clearToken(storage);
      location.reload();
    },
    onLatency: (_ms, sentAt) => {
      const ms = meter.received(sentAt, now());
      if (ms !== null) renderStatus(statusEl, "open", Math.round(ms));
    },
  });

  bindController(pad, link.send);
}
