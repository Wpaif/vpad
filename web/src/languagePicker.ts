import type { Store } from "./auth";
import { LANGUAGES, chooseLanguage, getLanguage, type Language } from "./i18n";

/** Seletor do idioma da página (só os predefinidos). Escolher outro recarrega a página. */
export function mountLanguagePicker(select: HTMLSelectElement, storage: Store, reload?: () => void): void {
  for (const { code, name } of LANGUAGES) {
    const option = document.createElement("option");
    option.value = code;
    option.textContent = name; // textContent: nada vira HTML
    select.append(option);
  }
  select.value = getLanguage();
  select.addEventListener("change", () => chooseLanguage(select.value as Language, storage, reload));
}
