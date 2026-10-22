# Changelog

Todas as mudanças importantes deste projeto ficam neste arquivo.

O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e o projeto usa
[versionamento semântico](https://semver.org/lang/pt-BR/).

## [Não lançado]

## [0.1.0] - 2026-10-22

Primeira versão pública: um controle virtual no iPhone que cria um gamepad ou teclado virtual no PC.

### Adicionado

- **Servidor em Rust** (axum, rustls, evdev) que cria o dispositivo virtual por uinput, com os perfis
  Xbox 360, DualShock 4, Switch Pro e teclado.
- **Página no iPhone** (Svelte 5 + TypeScript), em Safari ou como app da Tela de Início: multi-toque,
  analógicos virtuais, gatilhos, direcional, tela cheia, ícone próprio, medidor de latência e modo de ping
  curto para o Wi-Fi do celular não "dormir".
- **Layout dos botões editável**: mover (com limites e ímã), redimensionar, ocultar, soltar os botões de um
  grupo um a um e ajustar a opacidade. O layout fica guardado no aparelho e também no PC, compartilhado
  pelos aparelhos pareados, e sobrevive a reinstalar o app.
- **Modo teclado** com editor de teclas, permissão por aparelho e confirmação no PC a cada ativação.
- **Segurança para a rede local**: HTTPS com autoridade de certificação própria (restrita à rede local e com
  impressão digital para conferir), pareamento fechado por padrão com código de 6 dígitos que muda a cada
  30 s, aprovação no PC, bloqueios crescentes e alertas com o IP, um token por aparelho (só o hash fica
  guardado), revogação imediata, conferência de `Host` e `Origin`, limites de tamanho e cabeçalhos de segurança.
- **Seis idiomas**: português, inglês, espanhol, chinês (simplificado), japonês e coreano, na página, nos
  erros da API (por código estável), na página do certificado, no perfil do iPhone e nos nomes das teclas.
- **Terminal do PC** em português ou em inglês, conforme o idioma do sistema (`VPAD_LANG` força um deles);
  as respostas aceitam `s`/`y` e `t`/`k`, e `--revogar` aceita `todos` ou `all`.
- **Projeto**: licença MIT, README nos seis idiomas, CI no GitHub Actions (`web`, `rust` e `hooks`),
  proteção da `master`, hook que bloqueia commit direto nela e testes que travam a consistência da
  documentação e das versões.

### Limitações conhecidas

- Só **Linux** (testado no Manjaro) e **iPhone** (Safari ou app da Tela de Início); Windows não é suportado.
- Não há binário pré-compilado: é preciso compilar o servidor (Rust 1.85 ou mais novo) e a página (Node).
- As traduções em chinês, japonês e coreano e os nomes dos menus do iOS em cada idioma não passaram por
  revisão de falante nativo.
- O layout dos botões é um só para todos os aparelhos pareados.
- O terminal do PC só tem português e inglês.
- Os testes que injetam entrada de verdade (uinput) só rodam localmente (`cargo test -- --ignored`); no CI
  eles se pulam por falta de `/dev/uinput`.

[Não lançado]: https://github.com/Wpaif/vpad/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Wpaif/vpad/releases/tag/v0.1.0
