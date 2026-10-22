# vpad

Controle virtual para o PC, no celular. O iPhone abre uma página (ou um app na Tela de Início),
manda os toques pela rede local, e um servidor em Rust no PC cria um **gamepad ou teclado virtual**
(uinput, Linux) que qualquer jogo ou programa enxerga como um dispositivo de verdade.

- **Perfis**: Xbox 360, DualShock 4, Switch Pro e teclado.
- **Multi-toque**, analógicos virtuais, gatilhos e direcional, em paisagem e tela cheia.
- **Latência à vista**: medidor com ping/pong e, se o Wi-Fi do celular "dormir", um modo de ping curto.
- **Layout dos botões editável**: mover, redimensionar, ocultar e soltar botões um a um.
- **Seguro para a rede local**: HTTPS com autoridade própria, pareamento por código e aprovação no PC.

> Alvo atual: **Linux** (testado no Manjaro) e **iPhone** (Safari / app da Tela de Início).
> Windows não é suportado.

## Sumário

- [Requisitos](#requisitos)
- [Instalação](#instalação)
- [Primeiro uso](#primeiro-uso)
- [Uso no dia a dia](#uso-no-dia-a-dia)
- [Opções de linha de comando](#opções-de-linha-de-comando)
- [Layout dos botões](#layout-dos-botões)
- [Modo teclado](#modo-teclado)
- [Segurança](#segurança)
- [Arquivos de configuração](#arquivos-de-configuração)
- [Estrutura do projeto](#estrutura-do-projeto)
- [Desenvolvimento](#desenvolvimento)
- [Problemas comuns](#problemas-comuns)

## Requisitos

No PC:

- Linux com o módulo `uinput` e permissão de escrita em `/dev/uinput`.
- Rust (edição 2024, ou seja, 1.85 ou mais novo) para compilar o servidor.
- Node.js e npm para compilar a página (Vite + Svelte).

No celular:

- iPhone com Safari. O PC e o celular precisam estar na **mesma rede local**.

Para liberar o `uinput`:

```sh
sudo modprobe uinput
# permissão para o seu usuário (exemplo com ACL):
sudo setfacl -m u:$USER:rw /dev/uinput
```

Para o módulo carregar sozinho a cada boot, crie `/etc/modules-load.d/uinput.conf` com a linha
`uinput`. A permissão por ACL (ou uma regra do udev) também precisa persistir se você quiser
que sobreviva a reiniciar o PC.

## Instalação

Na pasta do projeto. A página fica **embutida no binário** (em compilação de release; em debug o
servidor lê `web/dist` do disco), então compile a página antes do servidor:

```sh
cd web && npm install && npm run build && cd ..
cargo build --release
```

O binário sai em `target/release/vpad`. Se você mudar qualquer coisa em `web/`, rode
`npm run build` e `cargo build --release` de novo.

## Primeiro uso

1. **Inicie o servidor** no PC:

   ```sh
   ./target/release/vpad
   ```

   O terminal mostra um QR code, o endereço do controle (`https://IP-DO-PC:8443`), a impressão
   digital SHA-256 da autoridade de certificação e os IPs cobertos pelo certificado.

2. **Instale o certificado no iPhone** (uma vez só). Na primeira execução o servidor cria uma
   autoridade de certificação própria e abre por 5 minutos uma página de configuração em
   HTTP (`http://IP-DO-PC:8080`). Para abri-la de novo mais tarde, digite `c` no terminal.
   No iPhone:

   1. Abra a página de configuração e toque em **Baixar o perfil** → **Permitir**.
   2. Em **Ajustes → Perfil baixado**, toque no certificado e em **Mais detalhes**, e confira se
      a **impressão digital SHA-256** é igual à do **terminal do PC**. Só então toque em **Instalar**.
   3. Em **Ajustes → Geral → Sobre → Ajustes de Confiança de Certificado**, ative a confiança em
      **vpad**.

3. **Pareie o aparelho.** Por padrão o pareamento fica **fechado**. No terminal do PC, pressione
   **Enter**: ele abre o pareamento por 2 minutos e mostra um código de 6 dígitos que muda a cada
   30 segundos. No iPhone, abra `https://IP-DO-PC:8443` e digite o código. O PC pede a sua
   aprovação: **`s`** aprova, **`t`** aprova e libera também o [modo teclado](#modo-teclado),
   qualquer outra tecla nega.

4. **Instale como app** (opcional, recomendado): no Safari, **Compartilhar → Adicionar à Tela de
   Início**. O app abre em tela cheia e na horizontal.

> O app da Tela de Início tem **armazenamento próprio**, separado do Safari: ele precisa ser
> pareado por conta própria. Se você reinstalar o app, é preciso parear de novo; o layout dos
> botões volta sozinho, porque fica guardado também no PC.

## Uso no dia a dia

Com o servidor rodando, abra o app no iPhone. Ele reconecta sozinho se o Wi-Fi cair.

- **Pílula de status** (no topo): mostra a latência. Tocar nela abre o painel com a escolha do
  perfil, as estatísticas (último, média, mínimo, máximo, jitter, perda), o botão de ping curto
  e o acesso ao editor de layout.
- **Olho** (topo): oculta ou mostra os controles.
- **Ping curto**: se a latência sobe quando você fica sem tocar, é o Wi-Fi do iPhone entrando em
  economia de energia. Ligar o *ping curto* manda um aviso a cada 100 ms e mantém o rádio
  acordado (gasta mais bateria do celular).

Comandos do terminal do servidor (digite e tecle Enter):

| Comando | O que faz |
| --- | --- |
| `Enter` | abre o pareamento por 2 minutos e mostra o código |
| `f` | fecha o pareamento agora |
| `c` | abre por 5 minutos a página que instala o certificado |
| `?` | ajuda |
| `s` / `t` / outra | responde a um pedido de aprovação: sim / sim e liberar teclado / negar |

## Opções de linha de comando

```
vpad [--perfil <xbox360|ds4|switch-pro|teclado>] [--porta <1-65535>]
     [--porta-https <1-65535>] [--parear]
vpad --dispositivos
vpad --revogar <id|todos>
vpad --permitir-teclado <id> | --negar-teclado <id>
```

| Opção | Significado |
| --- | --- |
| `--perfil` | perfil inicial (o celular pode trocar depois). Padrão: `xbox360` |
| `--porta` | porta HTTP da página de configuração do certificado. Padrão: `8080` |
| `--porta-https` | porta HTTPS do controle. Padrão: `8443` |
| `--parear` | abre o pareamento ao iniciar (útil quando o servidor roda sem terminal) |
| `--dispositivos` | lista os aparelhos pareados (id, nome, validade, último uso, teclado) |
| `--revogar` | revoga um aparelho pelo id, ou todos. Vale na hora, mesmo com o servidor rodando |
| `--permitir-teclado` / `--negar-teclado` | libera ou bloqueia o modo teclado de um aparelho |

## Layout dos botões

Toque na pílula de status e em **Editar layout…**. O painel fecha, os grupos ganham contorno e
aparece uma barra no centro da tela.

- **Mover**: arraste um grupo. Ele não sai da tela, respeita as áreas seguras do iPhone e
  gruda (com uma linha de guia) nas bordas e centros dos outros grupos.
- **Redimensionar**: arraste a alça redonda do canto de baixo à direita. A escala vai de 0,5× a
  2×, o grupo não cresce além da borda e o menor botão nunca fica abaixo de 44 px.
- **Ocultar**: o botão **Botões** abre uma lista com um interruptor por controle. Os nomes
  seguem o perfil em uso (Cruz, Círculo… no DualShock; as teclas no modo teclado).
- **Soltar um grupo**: o cadeado do canto superior direito de cada grupo (com mais de um botão)
  o destrava para mover cada botão individualmente. Grupo travado se move em bloco.
- **Restaurar / Cancelar / Salvar**: só **Salvar** grava. **Restaurar** volta ao layout padrão
  (dentro do rascunho) e **Cancelar** descarta o que foi feito.

Durante a edição os toques **não** vão para o PC. O layout é guardado no aparelho e também no
PC (`layout.json`), compartilhado entre os aparelhos pareados: sobrevive a reinstalar o app e à
diferença entre Safari e app da Tela de Início. Sem conexão com o PC, o layout fica só no
aparelho e é enviado depois; o painel mostra em qual situação ele está.

## Modo teclado

No perfil **Teclado** os botões digitam no PC. Como isso é poderoso, tem duas travas:

1. O aparelho precisa ter a **permissão de teclado**, dada na aprovação do pareamento (`t`) ou
   depois com `vpad --permitir-teclado <id>`.
2. Cada vez que o modo é ativado, o **PC pede confirmação** no terminal.

O botão **Editar teclas…** do painel escolhe a tecla de cada botão; o mapa fica em
`teclado.json`.

## Segurança

O vpad é pensado para uma rede local doméstica, mas trata a rede como não confiável:

- **HTTPS com autoridade própria.** A autoridade de certificação (CA) é criada no PC e só vale
  para nomes e IPs de rede local (restrição de nomes). A chave dela nunca sai do PC. A impressão
  digital SHA-256 aparece no terminal para você conferir antes de instalar o perfil.
- **Pareamento fechado por padrão.** O código tem 6 dígitos e muda a cada 30 s; erros seguidos
  geram bloqueios crescentes (60 s, 300 s, 1800 s) e alertas com o IP no terminal. Mesmo com o
  código certo, **quem está no PC precisa aprovar**.
- **Um token por aparelho.** O servidor guarda só o hash SHA-256; o token vale 90 dias e cada
  aparelho pode ser revogado sem afetar os outros.
- **Sem token na URL.** O WebSocket recebe o token pelo subprotocolo, e a API pelo cabeçalho
  `Authorization`.
- **Barreiras no servidor.** Confere `Host` e `Origin` (contra *DNS rebinding* e páginas de fora),
  limita o tamanho das mensagens e dos corpos, e envia cabeçalhos de segurança (CSP restritiva,
  `X-Frame-Options`, HSTS e outros). Sem sinal do celular por 6 s (ele manda um ping por
  segundo), a conexão é encerrada e todos os botões são soltos.
- **Teclado com permissão e confirmação** (veja acima).

Para dependências, `make audit` consulta o banco de vulnerabilidades do RustSec.

## Arquivos de configuração

Tudo fica em `$XDG_CONFIG_HOME/vpad/` (ou `~/.config/vpad/`):

| Arquivo | Conteúdo |
| --- | --- |
| `dispositivos.json` | aparelhos pareados (id, nome, **hash** do token, validade, permissão de teclado) |
| `dispositivos.vistos.json` | último uso de cada aparelho |
| `teclado.json` | mapa de teclas do modo teclado |
| `layout.json` | layout dos botões, compartilhado pelos aparelhos pareados |
| `tls/` | autoridade de certificação e certificado do servidor (`ca-chave.pem` é a chave da CA: **não compartilhe**) |

Para começar do zero, apague a pasta `vpad` (você precisará instalar o perfil e parear de novo).

## Estrutura do projeto

```
src/                 servidor (Rust, axum + rustls + evdev)
  main.rs            inicialização, console do terminal, CLI
  lib.rs             rotas, WebSocket, porteiro (Host/Origin), cabeçalhos de segurança
  api.rs             /api/parear, /api/teclado, /api/layout
  servico.rs         ponte entre os eventos do celular e o dispositivo virtual
  dispositivo.rs     gamepad e teclado virtuais (uinput)
  perfil.rs          perfis (Xbox 360, DualShock 4, Switch Pro, teclado) e opções
  acesso.rs, pareamento.rs, dispositivos.rs, aprovacao.rs, alerta.rs, totp.rs
                     controle de acesso: código, aprovação, tokens, alertas
  tls.rs, https.rs, configuracao.rs, mobileconfig.rs
                     CA própria, HTTPS e a página que instala o certificado no iPhone
  layout.rs, armazem.rs, mapa.rs
                     armazenamento do layout e do mapa de teclas
tests/               testes de integração (API, HTTPS, uinput, página)
web/                 página (Svelte 5 + TypeScript + Vite + Vitest)
  src/Pad.svelte     o controle
  src/controller.ts  toques → eventos; src/connection.ts: WebSocket e ping
  src/layout*.ts     modelo, editor, geometria e sincronização do layout
scripts/             auditar.sh (RustSec) e gerar-icones.sh (ícones a partir do SVG)
```

Protocolo binário, pelo WebSocket (detalhes em `src/protocolo.rs`):

| Mensagem | Sentido | Conteúdo |
| --- | --- | --- |
| `[1, id, down]` | celular → PC | botão (0 a 9 são botões, 10 a 13 o direcional) |
| `[2, id, lo, hi]` | celular → PC | eixo, i16 little endian (analógicos e gatilhos) |
| `[3, …8 bytes]` | celular → PC | ping, devolvido igual ao celular |
| `[4, índice]` | nos dois sentidos | o celular pede a troca de perfil; o servidor informa o perfil ativo |
| `[5, 1]` | PC → celular | acesso recusado (token ausente, inválido ou revogado) |
| `[6, motivo]` | PC → celular | situação do pedido de ativar o modo teclado |

## Desenvolvimento

```sh
make test      # cargo test + npm test
make audit     # dependências Rust contra o RustSec (precisa de: cargo install cargo-audit --locked)
make check     # test + audit
```

- Depois de mudar a página, rode `npm run build` em `web/` e reinicie o servidor (`cargo run`
  em debug lê `web/dist` do disco).
- Verificação de tipos: `cd web && npm run check`.
- Os testes que injetam entrada de verdade na sessão ficam marcados com `#[ignore]` e usam só
  teclas modificadoras: `cargo test -- --ignored`.
- Para testar sem mexer na sua configuração, use portas e pasta próprias:
  `XDG_CONFIG_HOME=/tmp/vpad-teste ./target/debug/vpad --porta 18080 --porta-https 18443`.
- Para regenerar os ícones depois de editar `web/public/icon.svg`: `./scripts/gerar-icones.sh`
  (precisa de `rsvg-convert` e `magick`).

Fluxo de trabalho do projeto: uma branch por funcionalidade (`feature/<nome>`), testes escritos
antes da implementação, e merge na `master` só com os testes passando.

## Problemas comuns

| Sintoma | O que fazer |
| --- | --- |
| `não consegui criar o controle virtual` | carregue o módulo (`sudo modprobe uinput`) e dê permissão de escrita em `/dev/uinput` ao seu usuário |
| o Safari avisa que a conexão não é segura | o perfil do certificado não foi instalado, ou falta ativar a confiança em *Ajustes de Confiança de Certificado* |
| o certificado deixou de valer depois de o PC mudar de IP | o servidor renova o certificado sozinho ao iniciar; se a CA foi trocada, instale o perfil de novo |
| "o pareamento está fechado" | no terminal do PC, pressione Enter para abrir por 2 minutos |
| "muitas tentativas erradas" | espere o bloqueio acabar (o tempo aparece na mensagem) |
| a primeira tecla depois de uma pausa chega atrasada | ligue o **ping curto** no painel; é o Wi-Fi do celular dormindo |
| o app da Tela de Início pede o pareamento de novo | ele tem armazenamento separado do Safari; pareie uma vez dentro do app |
| o ícone antigo continua na Tela de Início | o iOS fixa o ícone ao adicionar: remova o app e adicione de novo |
