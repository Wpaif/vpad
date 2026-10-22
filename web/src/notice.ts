import { t } from "./i18n";
import type { ProfileNotice } from "./protocol";

/** Texto do aviso sobre o pedido de ativar o modo teclado. O id só entra se tiver o formato esperado. */
export function describeNotice(notice: ProfileNotice, deviceId: string | null): string {
  const id = deviceId !== null && /^[0-9a-f]{8}$/.test(deviceId) ? deviceId : null;
  switch (notice) {
    case "no-permission":
      return id ? t("notice.noPermissionWithId", { id }) : t("notice.noPermission");
    case "waiting":
      return t("notice.waiting");
    case "denied":
      return t("notice.denied");
    case "expired":
      return t("notice.expired");
    case "busy":
      return t("notice.busy");
  }
}
