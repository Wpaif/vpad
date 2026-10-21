import "./style.css";
import { connect } from "./connection";
import { bindController } from "./controller";
import { bindInstallHint, enterFullscreenOnTouch } from "./fullscreen";
import { createLatencyMeter } from "./latency";
import { mountProfilePicker } from "./picker";
import { encodeProfile } from "./protocol";
import { bindAppHeight, showViewportDebug } from "./viewport";
import { bindPanelToggle, bindVisibilityToggle, lockGestures, renderStats, renderStatus } from "./ui";

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
  localStorage,
);

const now = () => performance.timeOrigin + performance.now();
const pad = document.getElementById("pad")!;
const statusEl = document.getElementById("status")!;
const statsEl = document.getElementById("stats")!;
bindVisibilityToggle(document.getElementById("toggle")!, pad);

const picker = mountProfilePicker(document.getElementById("profile-picker")!, (i) => link.send(encodeProfile(i)));
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
  url: `${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/ws`,
  now,
  onStatus: (status) => {
    if (status !== "open") {
      meter.reset();
      picker.setCurrent(null);
    }
    renderStatus(statusEl, status, null);
  },
  onPingSent: (ts) => meter.sent(ts),
  onProfile: (index) => picker.setCurrent(index),
  onLatency: (_ms, sentAt) => {
    const ms = meter.received(sentAt, now());
    if (ms !== null) renderStatus(statusEl, "open", Math.round(ms));
  },
});

bindController(pad, link.send);
