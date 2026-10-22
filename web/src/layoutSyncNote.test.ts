import { describe, expect, it } from "vitest";
import { renderSyncNote } from "./layoutSyncNote";

const el = () => document.createElement("p");

describe("renderSyncNote", () => {
  it("sem estado conhecido não mostra nada", () => {
    const e = el();
    renderSyncNote(e, "none");
    expect(e.textContent).toBe("");
  });

  it("sincronizado diz que está salvo no PC", () => {
    const e = el();
    renderSyncNote(e, "synced");
    expect(e.textContent).toContain("PC");
    expect(e.dataset.state).toBe("synced");
  });

  it("pendente explica que fica só neste aparelho por enquanto", () => {
    const e = el();
    renderSyncNote(e, "pending");
    expect(e.textContent).toMatch(/só neste aparelho/i);
    expect(e.dataset.state).toBe("pending");
  });

  it("recusado avisa que o PC não aceitou", () => {
    const e = el();
    renderSyncNote(e, "rejected");
    expect(e.textContent).toMatch(/recus/i);
  });

  it("trocar de estado substitui o texto", () => {
    const e = el();
    renderSyncNote(e, "pending");
    renderSyncNote(e, "synced");
    expect(e.textContent).not.toMatch(/só neste aparelho/i);
  });
});
