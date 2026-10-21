<!--
  O controle: três zonas (esquerda, centro, direita) com sete grupos. Cada grupo tem um
  `data-grupo`, que é por onde o layout (mover, redimensionar, ocultar) o encontra. O
  `data-btn`, `data-trigger` e `data-stick` são o que o controle de toque e o protocolo usam.
  Os estilos ficam em style.css; os rótulos de cada perfil são aplicados depois, por labels.ts.
-->
<main id="pad" aria-label="Controle virtual">
  <section class="zone left">
    <div class="shoulders" data-grupo="ombros-esq">
      <button class="shoulder" data-trigger="LT">LT</button>
      <button class="shoulder" data-btn="LB">LB</button>
    </div>
    <div class="dpad" data-grupo="direcional">
      <button data-btn="Up" class="up" aria-label="Cima"><svg viewBox="0 0 24 24"><path d="M12 6l7 9H5z" /></svg></button>
      <button data-btn="Left" class="left" aria-label="Esquerda"><svg viewBox="0 0 24 24"><path d="M6 12l9-7v14z" /></svg></button>
      <button data-btn="Right" class="right" aria-label="Direita"><svg viewBox="0 0 24 24"><path d="M18 12l-9 7V5z" /></svg></button>
      <button data-btn="Down" class="down" aria-label="Baixo"><svg viewBox="0 0 24 24"><path d="M12 18l-7-9h14z" /></svg></button>
    </div>
    <div class="stick" data-stick="left" data-grupo="analogico-esq"><div class="knob"></div></div>
  </section>

  <section class="zone center" data-grupo="menu">
    <button class="pill" data-btn="Select">Select</button>
    <button class="pill" data-btn="Start">Start</button>
  </section>

  <section class="zone right">
    <div class="shoulders" data-grupo="ombros-dir">
      <button class="shoulder" data-btn="RB">RB</button>
      <button class="shoulder" data-trigger="RT">RT</button>
    </div>
    <div class="face" data-grupo="face">
      <button data-btn="Y" class="top">Y</button>
      <button data-btn="X" class="left">X</button>
      <button data-btn="B" class="right">B</button>
      <button data-btn="A" class="bottom">A</button>
    </div>
    <div class="stick" data-stick="right" data-grupo="analogico-dir"><div class="knob"></div></div>
  </section>
</main>
