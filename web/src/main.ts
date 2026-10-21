import "./style.css";
import { connect } from "./connection";
import { bindController } from "./controller";
import { bindVisibilityToggle, lockGestures, renderStatus } from "./ui";

lockGestures();

const pad = document.getElementById("pad")!;
const statusEl = document.getElementById("status")!;
bindVisibilityToggle(document.getElementById("toggle")!, pad);

let latency: number | null = null;
const link = connect({
  url: `${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/ws`,
  onStatus: (status) => {
    if (status !== "open") latency = null;
    renderStatus(statusEl, status, latency);
  },
  onLatency: (ms) => {
    latency = ms;
    renderStatus(statusEl, "open", ms);
  },
});

bindController(pad, link.send);
