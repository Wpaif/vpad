/** Interruptor do ping curto: o botão mostra o estado e cada clique o inverte. */
export function mountPingToggle(button: HTMLElement, initial: boolean, onChange: (on: boolean) => void): void {
  let on = initial;
  const render = () => {
    button.setAttribute("aria-pressed", String(on));
    button.textContent = `ping curto: ${on ? "ligado" : "desligado"}`;
  };
  render();
  button.addEventListener("click", () => {
    on = !on;
    render();
    onChange(on);
  });
}
