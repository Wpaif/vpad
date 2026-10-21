import { defineConfig } from "vitest/config";

export default defineConfig({
  server: { host: true, proxy: { "/ws": { target: "ws://localhost:8080", ws: true } } },
  test: { environment: "happy-dom" },
});
