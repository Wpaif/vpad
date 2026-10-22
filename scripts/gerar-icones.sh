#!/bin/sh
# Gera os PNGs do ícone a partir de web/public/icon.svg (precisa de rsvg-convert e magick).
# Os PNGs saem em RGB sem canal alfa: o iOS pinta a transparência de preto na Tela de Início.
set -e
cd "$(dirname "$0")/../web/public"
gerar() { # tamanho arquivo
  rsvg-convert -w "$1" -h "$1" icon.svg -o ".tmp-$2"
  magick ".tmp-$2" -alpha off -depth 8 "PNG24:$2"
  rm -f ".tmp-$2"
}
gerar 512 icon-512.png
gerar 192 icon-192.png
gerar 180 apple-touch-icon.png
gerar 32 favicon-32.png
