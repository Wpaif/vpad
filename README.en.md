# vpad

[![CI](https://github.com/Wpaif/vpad/actions/workflows/ci.yml/badge.svg)](https://github.com/Wpaif/vpad/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[Português](README.md) · **English** · [Español](README.es.md) · [中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

A virtual controller for your PC, on your phone. The iPhone opens a page (or a Home Screen app),
sends the touches over the local network, and a Rust server on the PC creates a **virtual gamepad
or keyboard** (uinput, Linux) that any game or program sees as a real device.

- **Profiles**: Xbox 360, DualShock 4, Switch Pro and keyboard.
- **Multi-touch**, virtual analog sticks, triggers and D-pad, in landscape and full screen.
- **Latency in plain sight**: a ping/pong meter and, if the phone's Wi-Fi "falls asleep", a short ping mode.
- **Editable button layout**: move, resize, hide and unlock buttons one by one.
- **Six languages**: Portuguese, English, Spanish, Chinese (Simplified), Japanese and Korean, chosen
  by the phone's language or in the panel.
- **Safe for the local network**: HTTPS with its own authority, pairing by code and approval on the PC.

> Current target: **Linux** (tested on Manjaro) and **iPhone** (Safari / Home Screen app).
> Windows is not supported.

## Table of contents

- [Requirements](#requirements)
- [Installation](#installation)
- [First use](#first-use)
- [Everyday use](#everyday-use)
- [Command-line options](#command-line-options)
- [Button layout](#button-layout)
- [Keyboard mode](#keyboard-mode)
- [Security](#security)
- [Configuration files](#configuration-files)
- [Development](#development)
- [Common problems](#common-problems)
- [License](#license)

## Requirements

On the PC:

- Linux with the `uinput` module and write permission on `/dev/uinput`.
- Rust (edition 2024, that is, 1.85 or newer) to build the server.
- Node.js and npm to build the page (Vite + Svelte).

On the phone:

- An iPhone with Safari. The PC and the phone must be on the **same local network**.

To enable `uinput`:

```sh
sudo modprobe uinput
# permission for your user (example using an ACL):
sudo setfacl -m u:$USER:rw /dev/uinput
```

To load the module automatically on every boot, create `/etc/modules-load.d/uinput.conf` with the
line `uinput`. The ACL permission (or a udev rule) must also persist if you want it to survive a
reboot.

## Installation

Download the project and enter the folder:

```sh
git clone https://github.com/Wpaif/vpad.git
# or, over SSH: git clone git@github.com:Wpaif/vpad.git
cd vpad
```

The page is **embedded in the binary** (in a release build; in debug the server reads `web/dist`
from disk), so build the page before the server:

```sh
cd web && npm install && npm run build && cd ..
cargo build --release
```

The binary ends up in `target/release/vpad`. If you change anything in `web/`, run
`npm run build` and `cargo build --release` again.

## First use

1. **Start the server** on the PC:

   ```sh
   ./target/release/vpad
   ```

   The terminal shows a QR code, the controller address (`https://PC-IP:8443`), the SHA-256
   fingerprint of the certificate authority and the IPs covered by the certificate.

2. **Install the certificate on the iPhone** (only once). On the first run the server creates its
   own certificate authority and opens a setup page over HTTP (`http://PC-IP:8080`) for 5 minutes.
   To open it again later, type `c` in the terminal. On the iPhone (the page opens in the device's
   language and has links to switch language at the top; the iOS menu names below are the ones from
   the English version):

   1. Open the setup page and tap **Download the profile** → **Allow**.
   2. Under **Settings → Profile Downloaded**, tap the certificate and then **More Details**, and
      check that the **SHA-256 fingerprint** matches the one in the **PC terminal**. Only then tap **Install**.
   3. Under **Settings → General → About → Certificate Trust Settings**, turn on trust for
      **vpad**.

3. **Pair the device.** By default pairing is **closed**. In the PC terminal, press **Enter**: it
   opens pairing for 2 minutes and shows a 6-digit code that changes every 30 seconds. On the
   iPhone, open `https://PC-IP:8443` and type the code. The PC asks for your approval: **`y`**
   approves, **`k`** approves and also allows [keyboard mode](#keyboard-mode), any other key denies.

4. **Install it as an app** (optional, recommended): in Safari, **Share → Add to Home Screen**.
   The app opens in full screen and in landscape.

> The Home Screen app has **its own storage**, separate from Safari: it must be paired on its own.
> If you reinstall the app, you have to pair again; the button layout comes back by itself, because
> it is also stored on the PC.

## Everyday use

With the server running, open the app on the iPhone. It reconnects by itself if the Wi-Fi drops.

- **Status pill** (at the top): shows the latency. Tapping it opens the panel with the profile
  picker, the statistics (last, avg, min, max, jitter, loss), the short ping button and the way
  into the layout editor.
- **Eye** (top): hides or shows the controls.
- **Language**: the panel has a selector with the six languages. On the first launch the phone's
  language is used (anyone who speaks a language without a translation gets English); changing it
  reloads the page.
- **Short ping**: if the latency rises when you stop touching the screen, it is the iPhone's Wi-Fi
  entering power saving. Turning on *short ping* sends a message every 100 ms and keeps the radio
  awake (it uses more of the phone's battery).

Commands of the server terminal (type and press Enter):

| Command | What it does |
| --- | --- |
| `Enter` | opens pairing for 2 minutes and shows the code |
| `f` or `close` | closes pairing now |
| `c` | opens, for 5 minutes, the page that installs the certificate |
| `?` | help |
| `s` or `y` / `t` or `k` / other | answers an approval request: yes / yes and allow keyboard / deny |

## Command-line options

```
vpad [--perfil <xbox360|ds4|switch-pro|teclado>] [--porta <1-65535>]
     [--porta-https <1-65535>] [--parear]
vpad --dispositivos
vpad --revogar <id|todos>
vpad --permitir-teclado <id> | --negar-teclado <id>
```

The option names are in Portuguese and are the same in every language.

| Option | Meaning |
| --- | --- |
| `--perfil` | initial profile (the phone can change it later). Default: `xbox360` |
| `--porta` | HTTP port of the certificate setup page. Default: `8080` |
| `--porta-https` | HTTPS port of the controller. Default: `8443` |
| `--parear` | opens pairing on startup (useful when the server runs without a terminal) |
| `--dispositivos` | lists the paired devices (id, name, validity, last use, keyboard) |
| `--revogar` | revokes one device by id, or all of them (`todos` or `all`). It takes effect immediately, even with the server running |
| `--permitir-teclado` / `--negar-teclado` | allows or blocks a device's keyboard mode |

## Button layout

Tap the status pill and then **Edit layout…**. The panel closes, the groups get an outline and a
bar appears in the middle of the screen.

- **Move**: drag a group. It cannot leave the screen, it respects the iPhone's safe areas and
  snaps (with a guide line) to the edges and centers of the other groups.
- **Resize**: drag the round handle at the bottom-right corner. The scale goes from 0.5× to 2×, the
  group cannot grow past the edge, and the smallest button never gets below 44 px.
- **Hide**: the **Buttons** button opens a list with one switch per control. The names follow the
  profile in use (Cross, Circle… on the DualShock; the keys in keyboard mode).
- **Unlock a group**: the padlock at the top-right corner of each group (with more than one
  button) unlocks it so you can move each button individually. A locked group moves as one block.
- **Opacity**: the slider in the bar makes all buttons more transparent, from 100% down to 20%. A
  pressed button is shown in full, so you can see the touch even at low opacity.
- **Reset / Cancel / Save**: only **Save** writes. **Reset** goes back to the default layout
  (inside the draft) and **Cancel** discards what you did.

While editing, touches do **not** go to the PC. The layout is stored on the device and also on the
PC (`layout.json`), shared by the paired devices: it survives reinstalling the app and the
difference between Safari and the Home Screen app. Without a connection to the PC, the layout stays
on the device only and is sent later; the panel shows which situation it is in.

## Keyboard mode

In the **Keyboard** profile the buttons type on the PC. Because that is powerful, it has two locks:

1. The device must have the **keyboard permission**, granted when approving the pairing (`k`) or
   later with `vpad --permitir-teclado <id>`.
2. Every time the mode is turned on, **the PC asks for confirmation** in the terminal.

The **Edit keys…** button in the panel chooses the key for each button; the map is stored in
`teclado.json`.

## Security

vpad is designed for a home local network, but it treats the network as untrusted:

- **HTTPS with its own authority.** The certificate authority (CA) is created on the PC and is only
  valid for local-network names and IPs (name constraints). Its key never leaves the PC. The SHA-256
  fingerprint is shown in the terminal so you can check it before installing the profile.
- **Pairing closed by default.** The code has 6 digits and changes every 30 s; repeated errors
  cause growing lockouts (60 s, 300 s, 1800 s) and alerts with the IP in the terminal. Even with
  the right code, **whoever is at the PC must approve**.
- **One token per device.** The server keeps only the SHA-256 hash; the token is valid for 90 days
  and each device can be revoked without affecting the others.
- **No token in the URL.** The WebSocket receives the token through the subprotocol, and the API
  through the `Authorization` header.
- **Server-side barriers.** It checks `Host` and `Origin` (against *DNS rebinding* and outside
  pages), limits the size of messages and bodies, and sends security headers (strict CSP,
  `X-Frame-Options`, HSTS and others). With no signal from the phone for 6 s (it sends one ping per
  second), the connection is closed and all buttons are released.
- **Keyboard with permission and confirmation** (see above).

For dependencies, `make audit` checks the RustSec vulnerability database.

## Configuration files

Everything lives in `$XDG_CONFIG_HOME/vpad/` (or `~/.config/vpad/`):

| File | Contents |
| --- | --- |
| `dispositivos.json` | paired devices (id, name, token **hash**, validity, keyboard permission) |
| `dispositivos.vistos.json` | last use of each device |
| `teclado.json` | key map for keyboard mode |
| `layout.json` | button layout, shared by the paired devices |
| `tls/` | certificate authority and server certificate (`ca-chave.pem` is the CA key: **do not share it**) |

To start from scratch, delete the `vpad` folder (you will have to install the profile and pair again).

## Development

```sh
make test      # cargo test + npm test + the hook test
make hooks     # installs the hook that blocks direct commits to master
```

This section is only a summary: the project layout, the binary protocol, the translation
mechanisms and the whole contribution flow are in the [Portuguese README](README.md). In short:
one branch per feature, tests written first, and `master` only receives changes through a pull
request (`git push -u origin feature/<name>`, `gh pr create --fill`, `gh pr merge --merge`) with
the CI green.

## Common problems

| Symptom | What to do |
| --- | --- |
| `could not create the virtual controller` | load the module (`sudo modprobe uinput`) and give your user write permission on `/dev/uinput` |
| Safari warns that the connection is not secure | the certificate profile was not installed, or trust is still off in *Certificate Trust Settings* |
| the certificate stopped working after the PC changed IP | the server renews the certificate by itself on startup; if the CA was replaced, install the profile again |
| "pairing is closed" | in the PC terminal, press Enter to open it for 2 minutes |
| "too many wrong attempts" | wait for the lockout to end (the time appears in the message) |
| the first key after a pause arrives late | turn on **short ping** in the panel; it is the phone's Wi-Fi sleeping |
| the Home Screen app asks for pairing again | it has storage separate from Safari; pair once inside the app |
| the old icon is still on the Home Screen | iOS fixes the icon when it is added: remove the app and add it again |

## License

MIT. See the [LICENSE](LICENSE) file.
