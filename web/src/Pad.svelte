<script lang="ts">
  import { adjustOf, defaultLayout, isHidden, type ControlId, type GroupId, type Layout } from "./layoutModel";

  let { layout = defaultLayout(), editing = false }: { layout?: Layout; editing?: boolean } = $props();

  const oculto = (c: ControlId) => isHidden(layout, c);
  /** Sem ajuste não há transform, para o padrão continuar exatamente o do CSS. */
  function mover(g: GroupId): string | undefined {
    const { dx, dy, scale } = adjustOf(layout, g);
    if (dx === 0 && dy === 0 && scale === 1) return undefined;
    return `translate(calc(var(--app-w) * ${dx}), calc(var(--app-h) * ${dy})) scale(${scale})`;
  }
</script>

<!--
  O controle: três zonas (esquerda, centro, direita) com sete grupos. Cada grupo tem um
  `data-grupo`, que é por onde o layout (mover, redimensionar, ocultar) o encontra. O
  `data-btn`, `data-trigger` e `data-stick` são o que o controle de toque e o protocolo usam.
  Os estilos ficam em style.css; os rótulos de cada perfil são aplicados depois, por labels.ts.
-->
<main id="pad" aria-label="Controle virtual" data-editando={editing ? "" : undefined}>
  <section class="zone left">
    <div class="shoulders" data-grupo="ombros-esq" style:transform={mover("ombros-esq")}>
      <button class:oculto={oculto("LT")} class="shoulder" data-trigger="LT">LT</button>
      <button class:oculto={oculto("LB")} class="shoulder" data-btn="LB">LB</button>
    </div>
    <div class="dpad" data-grupo="direcional" style:transform={mover("direcional")}>
      <button data-btn="Up" class:oculto={oculto("Up")} class="up" aria-label="Cima"><svg viewBox="0 0 24 24"><path d="M12 6l7 9H5z" /></svg></button>
      <button data-btn="Left" class:oculto={oculto("Left")} class="left" aria-label="Esquerda"><svg viewBox="0 0 24 24"><path d="M6 12l9-7v14z" /></svg></button>
      <button data-btn="Right" class:oculto={oculto("Right")} class="right" aria-label="Direita"><svg viewBox="0 0 24 24"><path d="M18 12l-9 7V5z" /></svg></button>
      <button data-btn="Down" class:oculto={oculto("Down")} class="down" aria-label="Baixo"><svg viewBox="0 0 24 24"><path d="M12 18l-7-9h14z" /></svg></button>
    </div>
    <div class="stick" data-stick="left" class:oculto={oculto("StickLeft")} data-grupo="analogico-esq" style:transform={mover("analogico-esq")}><div class="knob"></div></div>
  </section>

  <section class="zone center" data-grupo="menu" style:transform={mover("menu")}>
    <button class:oculto={oculto("Select")} class="pill" data-btn="Select">Select</button>
    <button class:oculto={oculto("Start")} class="pill" data-btn="Start">Start</button>
  </section>

  <section class="zone right">
    <div class="shoulders" data-grupo="ombros-dir" style:transform={mover("ombros-dir")}>
      <button class:oculto={oculto("RB")} class="shoulder" data-btn="RB">RB</button>
      <button class:oculto={oculto("RT")} class="shoulder" data-trigger="RT">RT</button>
    </div>
    <div class="face" data-grupo="face" style:transform={mover("face")}>
      <button data-btn="Y" class:oculto={oculto("Y")} class="top">Y</button>
      <button data-btn="X" class:oculto={oculto("X")} class="left">X</button>
      <button data-btn="B" class:oculto={oculto("B")} class="right">B</button>
      <button data-btn="A" class:oculto={oculto("A")} class="bottom">A</button>
    </div>
    <div class="stick" data-stick="right" class:oculto={oculto("StickRight")} data-grupo="analogico-dir" style:transform={mover("analogico-dir")}><div class="knob"></div></div>
  </section>
</main>
