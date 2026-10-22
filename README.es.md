# vpad

[![CI](https://github.com/Wpaif/vpad/actions/workflows/ci.yml/badge.svg)](https://github.com/Wpaif/vpad/actions/workflows/ci.yml)
[![Licencia: MIT](https://img.shields.io/badge/licencia-MIT-blue.svg)](LICENSE)

[Português](README.md) · [English](README.en.md) · **Español** · [中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

Un mando virtual para tu PC, en el teléfono. El iPhone abre una página (o una app en la pantalla de
inicio), envía los toques por la red local y un servidor en Rust en el PC crea un **gamepad o
teclado virtual** (uinput, Linux) que cualquier juego o programa ve como un dispositivo de verdad.

- **Perfiles**: Xbox 360, DualShock 4, Switch Pro y teclado.
- **Multitáctil**, sticks analógicos virtuales, gatillos y cruceta, en horizontal y a pantalla completa.
- **Latencia a la vista**: medidor con ping/pong y, si el Wi-Fi del teléfono "se duerme", un modo de ping corto.
- **Diseño de botones editable**: mover, cambiar de tamaño, ocultar y soltar botones uno por uno.
- **Seis idiomas**: portugués, inglés, español, chino (simplificado), japonés y coreano, elegidos
  según el idioma del teléfono o en el panel.
- **Seguro para la red local**: HTTPS con autoridad propia, emparejamiento por código y aprobación en el PC.

> Objetivo actual: **Linux** (probado en Manjaro) e **iPhone** (Safari / app de la pantalla de inicio).
> Windows no es compatible.

## Índice

- [Requisitos](#requisitos)
- [Instalación](#instalación)
- [Primer uso](#primer-uso)
- [Uso diario](#uso-diario)
- [Opciones de línea de comandos](#opciones-de-línea-de-comandos)
- [Diseño de botones](#diseño-de-botones)
- [Modo teclado](#modo-teclado)
- [Seguridad](#seguridad)
- [Archivos de configuración](#archivos-de-configuración)
- [Desarrollo](#desarrollo)
- [Problemas comunes](#problemas-comunes)
- [Licencia](#licencia)

## Requisitos

En el PC:

- Linux con el módulo `uinput` y permiso de escritura en `/dev/uinput`.
- Rust (edición 2024, es decir, 1.85 o más reciente) para compilar el servidor.
- Node.js y npm para compilar la página (Vite + Svelte).

En el teléfono:

- Un iPhone con Safari. El PC y el teléfono deben estar en la **misma red local**.

Para habilitar `uinput`:

```sh
sudo modprobe uinput
# permiso para tu usuario (ejemplo con ACL):
sudo setfacl -m u:$USER:rw /dev/uinput
```

Para que el módulo se cargue solo en cada arranque, crea `/etc/modules-load.d/uinput.conf` con la
línea `uinput`. El permiso por ACL (o una regla de udev) también debe persistir si quieres que
sobreviva a un reinicio del PC.

## Instalación

Descarga el proyecto y entra en la carpeta:

```sh
git clone https://github.com/Wpaif/vpad.git
# o, con SSH: git clone git@github.com:Wpaif/vpad.git
cd vpad
```

La página va **incrustada en el binario** (en una compilación release; en debug el servidor lee
`web/dist` del disco), así que compila la página antes que el servidor:

```sh
cd web && npm install && npm run build && cd ..
cargo build --release
```

El binario queda en `target/release/vpad`. Si cambias algo en `web/`, ejecuta
`npm run build` y `cargo build --release` de nuevo.

## Primer uso

1. **Inicia el servidor** en el PC:

   ```sh
   ./target/release/vpad
   ```

   El terminal muestra un código QR, la dirección del mando (`https://IP-DEL-PC:8443`), la huella
   SHA-256 de la autoridad de certificación y las IP cubiertas por el certificado.

2. **Instala el certificado en el iPhone** (solo una vez). En la primera ejecución el servidor crea
   una autoridad de certificación propia y abre durante 5 minutos una página de configuración por
   HTTP (`http://IP-DEL-PC:8080`). Para abrirla de nuevo más tarde, escribe `c` en el terminal. En
   el iPhone (la página se abre en el idioma del dispositivo y tiene enlaces para cambiar de idioma
   arriba; los nombres de los menús de iOS de abajo son los de la versión en español):

   1. Abre la página de configuración y toca **Descargar el perfil** → **Permitir**.
   2. En **Ajustes → Perfil descargado**, toca el certificado y luego **Más detalles**, y comprueba
      que la **huella SHA-256** coincide con la del **terminal del PC**. Solo entonces toca **Instalar**.
   3. En **Ajustes → General → Información → Ajustes de confianza de certificados**, activa la
      confianza en **vpad**.

3. **Empareja el dispositivo.** Por defecto el emparejamiento está **cerrado**. En el terminal del
   PC, pulsa **Enter**: abre el emparejamiento durante 2 minutos y muestra un código de 6 dígitos
   que cambia cada 30 segundos. En el iPhone, abre `https://IP-DEL-PC:8443` y escribe el código. El
   PC pide tu aprobación: **`s`** (o **`y`**) aprueba, **`t`** (o **`k`**) aprueba y también
   habilita el [modo teclado](#modo-teclado), cualquier otra tecla deniega.

4. **Instálala como app** (opcional, recomendado): en Safari, **Compartir → Añadir a pantalla de
   inicio**. La app se abre a pantalla completa y en horizontal.

> La app de la pantalla de inicio tiene **almacenamiento propio**, separado de Safari: hay que
> emparejarla por separado. Si reinstalas la app, hay que emparejar de nuevo; el diseño de los
> botones vuelve solo, porque también se guarda en el PC.

## Uso diario

Con el servidor en marcha, abre la app en el iPhone. Se reconecta sola si el Wi-Fi se cae.

- **Píldora de estado** (arriba): muestra la latencia. Al tocarla se abre el panel con la elección
  de perfil, las estadísticas (último, media, mín, máx, jitter, pérdida), el botón de ping corto y
  el acceso al editor de diseño.
- **Ojo** (arriba): oculta o muestra los controles.
- **Idioma**: el panel tiene un selector con los seis idiomas. En la primera apertura vale el idioma
  del teléfono (quien habla un idioma sin traducción recibe inglés); cambiarlo recarga la página.
- **Ping corto**: si la latencia sube cuando dejas de tocar, es el Wi-Fi del iPhone entrando en
  ahorro de energía. Activar el *ping corto* envía un aviso cada 100 ms y mantiene la radio
  despierta (gasta más batería del teléfono).

Comandos del terminal del servidor (escribe y pulsa Enter):

| Comando | Qué hace |
| --- | --- |
| `Enter` | abre el emparejamiento durante 2 minutos y muestra el código |
| `f` o `close` | cierra el emparejamiento ahora |
| `c` | abre durante 5 minutos la página que instala el certificado |
| `?` | ayuda |
| `s` o `y` / `t` o `k` / otra | responde a una solicitud de aprobación: sí / sí y habilitar el teclado / denegar |

## Opciones de línea de comandos

```
vpad [--perfil <xbox360|ds4|switch-pro|teclado>] [--porta <1-65535>]
     [--porta-https <1-65535>] [--parear]
vpad --dispositivos
vpad --revogar <id|todos>
vpad --permitir-teclado <id> | --negar-teclado <id>
```

Los nombres de las opciones están en portugués y son los mismos en todos los idiomas.

| Opción | Significado |
| --- | --- |
| `--perfil` | perfil inicial (el teléfono puede cambiarlo después). Por defecto: `xbox360` |
| `--porta` | puerto HTTP de la página de configuración del certificado. Por defecto: `8080` |
| `--porta-https` | puerto HTTPS del mando. Por defecto: `8443` |
| `--parear` | abre el emparejamiento al iniciar (útil cuando el servidor corre sin terminal) |
| `--dispositivos` | lista los dispositivos emparejados (id, nombre, validez, último uso, teclado) |
| `--revogar` | revoca un dispositivo por id, o todos (`todos` o `all`). Surte efecto al instante, incluso con el servidor en marcha |
| `--permitir-teclado` / `--negar-teclado` | habilita o bloquea el modo teclado de un dispositivo |

## Diseño de botones

Toca la píldora de estado y luego **Editar diseño…**. El panel se cierra, los grupos reciben un
contorno y aparece una barra en el centro de la pantalla.

- **Mover**: arrastra un grupo. No sale de la pantalla, respeta las zonas seguras del iPhone y se
  ajusta (con una línea guía) a los bordes y centros de los otros grupos.
- **Cambiar de tamaño**: arrastra el tirador redondo de la esquina inferior derecha. La escala va
  de 0,5× a 2×, el grupo no crece más allá del borde y el botón más pequeño nunca baja de 44 px.
- **Ocultar**: el botón **Botones** abre una lista con un interruptor por control. Los nombres
  siguen el perfil en uso (Cruz, Círculo… en el DualShock; las teclas en el modo teclado).
- **Soltar un grupo**: el candado de la esquina superior derecha de cada grupo (con más de un
  botón) lo desbloquea para mover cada botón por separado. Un grupo bloqueado se mueve como un bloque.
- **Opacidad**: el control deslizante de la barra vuelve todos los botones más transparentes, del
  100% al 20%. Un botón pulsado se muestra completo, para que veas el toque incluso con poca opacidad.
- **Restaurar / Cancelar / Guardar**: solo **Guardar** graba. **Restaurar** vuelve al diseño
  predeterminado (dentro del borrador) y **Cancelar** descarta lo que hiciste.

Durante la edición los toques **no** van al PC. El diseño se guarda en el dispositivo y también en
el PC (`layout.json`), compartido entre los dispositivos emparejados: sobrevive a reinstalar la app
y a la diferencia entre Safari y la app de la pantalla de inicio. Sin conexión con el PC, el diseño
queda solo en el dispositivo y se envía después; el panel muestra en qué situación está.

## Modo teclado

En el perfil **Teclado** los botones escriben en el PC. Como eso es poderoso, tiene dos candados:

1. El dispositivo debe tener el **permiso de teclado**, concedido al aprobar el emparejamiento (`t`)
   o después con `vpad --permitir-teclado <id>`.
2. Cada vez que se activa el modo, **el PC pide confirmación** en el terminal.

El botón **Editar teclas…** del panel elige la tecla de cada botón; el mapa se guarda en
`teclado.json`.

## Seguridad

vpad está pensado para una red local doméstica, pero trata la red como no confiable:

- **HTTPS con autoridad propia.** La autoridad de certificación (CA) se crea en el PC y solo vale
  para nombres e IP de red local (restricción de nombres). Su clave nunca sale del PC. La huella
  SHA-256 aparece en el terminal para que la compruebes antes de instalar el perfil.
- **Emparejamiento cerrado por defecto.** El código tiene 6 dígitos y cambia cada 30 s; los errores
  seguidos provocan bloqueos crecientes (60 s, 300 s, 1800 s) y alertas con la IP en el terminal.
  Incluso con el código correcto, **quien está en el PC debe aprobar**.
- **Un token por dispositivo.** El servidor guarda solo el hash SHA-256; el token vale 90 días y
  cada dispositivo puede revocarse sin afectar a los demás.
- **Sin token en la URL.** El WebSocket recibe el token por el subprotocolo, y la API por la
  cabecera `Authorization`.
- **Barreras en el servidor.** Comprueba `Host` y `Origin` (contra *DNS rebinding* y páginas
  externas), limita el tamaño de los mensajes y de los cuerpos, y envía cabeceras de seguridad (CSP
  restrictiva, `X-Frame-Options`, HSTS y otras). Sin señal del teléfono durante 6 s (envía un ping
  por segundo), la conexión se cierra y se sueltan todos los botones.
- **Teclado con permiso y confirmación** (véase arriba).

Para las dependencias, `make audit` consulta la base de vulnerabilidades de RustSec.

## Archivos de configuración

Todo está en `$XDG_CONFIG_HOME/vpad/` (o `~/.config/vpad/`):

| Archivo | Contenido |
| --- | --- |
| `dispositivos.json` | dispositivos emparejados (id, nombre, **hash** del token, validez, permiso de teclado) |
| `dispositivos.vistos.json` | último uso de cada dispositivo |
| `teclado.json` | mapa de teclas del modo teclado |
| `layout.json` | diseño de los botones, compartido entre los dispositivos emparejados |
| `tls/` | autoridad de certificación y certificado del servidor (`ca-chave.pem` es la clave de la CA: **no la compartas**) |

Para empezar de cero, borra la carpeta `vpad` (tendrás que instalar el perfil y emparejar de nuevo).

## Desarrollo

```sh
make test      # cargo test + npm test + la prueba del hook
make hooks     # instala el hook que bloquea los commits directos en master
```

Esta sección es solo un resumen: la estructura del proyecto, el protocolo binario, los
mecanismos de traducción y todo el flujo de contribución están en el [README en portugués](README.md).
En resumen: una rama por funcionalidad, las pruebas se escriben primero, y `master` solo recibe
cambios mediante pull request (`git push -u origin feature/<nombre>`, `gh pr create --fill`,
`gh pr merge --merge`) con el CI en verde.

## Problemas comunes

| Síntoma | Qué hacer |
| --- | --- |
| `could not create the virtual controller` | carga el módulo (`sudo modprobe uinput`) y da permiso de escritura en `/dev/uinput` a tu usuario |
| Safari avisa de que la conexión no es segura | el perfil del certificado no se instaló, o falta activar la confianza en *Ajustes de confianza de certificados* |
| el certificado dejó de valer después de que el PC cambiara de IP | el servidor renueva el certificado solo al iniciar; si se cambió la CA, instala el perfil de nuevo |
| "el emparejamiento está cerrado" | en el terminal del PC, pulsa Enter para abrirlo durante 2 minutos |
| "demasiados intentos fallidos" | espera a que termine el bloqueo (el tiempo aparece en el mensaje) |
| la primera tecla tras una pausa llega con retraso | activa el **ping corto** en el panel; es el Wi-Fi del teléfono durmiendo |
| la app de la pantalla de inicio pide emparejar de nuevo | tiene un almacenamiento separado de Safari; empareja una vez dentro de la app |
| el icono antiguo sigue en la pantalla de inicio | iOS fija el icono al añadirla: elimina la app y añádela de nuevo |

## Licencia

MIT. Consulta el archivo [LICENSE](LICENSE).
