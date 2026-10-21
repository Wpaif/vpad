import "./style.css";
import { connect } from "./connection";
import { bindController } from "./controller";
import { createLatencyMeter } from "./latency";
import { bindPanelToggle, bindVisibilityToggle, lockGestures, renderStats, renderStatus } from "./ui";

lockGestures();

const now = () => performance.timeOrigin + performance.now();
const pad = document.getElementById("pad")!;
const statusEl = document.getElementById("status")!;
const statsEl = document.getElementById("stats")!;
bindVisibilityToggle(document.getElementById("toggle")!, pad);

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
    if (status !== "open") meter.reset();
    renderStatus(statusEl, status, null);
  },
  onPingSent: (ts) => meter.sent(ts),
  onLatency: (_ms, sentAt) => {
    const ms = meter.received(sentAt, now());
    if (ms !== null) renderStatus(statusEl, "open", Math.round(ms));
  },
});

bindController(pad, link.send);
