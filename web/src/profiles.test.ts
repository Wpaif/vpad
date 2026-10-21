import { describe, expect, it } from "vitest";
import { PROFILES } from "./profiles";

describe("PROFILES", () => {
  it("segue a mesma ordem dos perfis do servidor", () => {
    // O índice é o que trafega no protocolo: ver PERFIS em src/perfil.rs.
    expect(PROFILES.map((p) => p.id)).toEqual(["xbox360", "ds4", "switch-pro", "teclado"]);
  });

  it("tem rótulo completo e curto para cada perfil", () => {
    for (const p of PROFILES) {
      expect(p.label.length).toBeGreaterThan(0);
      expect(p.short.length).toBeGreaterThan(0);
      expect(p.short.length).toBeLessThanOrEqual(p.label.length);
    }
  });
});
