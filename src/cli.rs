//! Texto que o programa mostra no terminal: o código de pareamento com a contagem
//! regressiva e a lista de aparelhos pareados.

use crate::dispositivos::Registro;

/// "123456" -> "123 456", mais fácil de ler e de digitar.
pub fn codigo_agrupado(codigo: &str) -> String {
    if codigo.len() != 6 || !codigo.is_ascii() {
        return codigo.to_string();
    }
    format!("{} {}", &codigo[..3], &codigo[3..])
}

/// A linha que se reescreve a cada segundo: limpa a linha e volta ao começo dela.
pub fn linha_do_codigo(codigo: &str, segundos_restantes: u64) -> String {
    format!(
        "\r\x1b[2Kcódigo de pareamento: \x1b[1m{}\x1b[0m   (muda em {segundos_restantes} s)",
        codigo_agrupado(codigo)
    )
}

/// "AAAA-MM-DD HH:MM" em UTC a partir de segundos desde 1970 (algoritmo de dias civis).
pub fn data_utc(segundos: u64) -> String {
    let dias = (segundos / 86_400) as i64;
    let resto = segundos % 86_400;
    let (hora, minuto) = (resto / 3600, (resto % 3600) / 60);

    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = doy - (153 * mp + 2) / 5 + 1;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let ano = yoe + era * 400 + i64::from(mes <= 2);
    format!("{ano:04}-{mes:02}-{dia:02} {hora:02}:{minuto:02}")
}

pub fn tabela_de_dispositivos(lista: &[Registro]) -> String {
    if lista.is_empty() {
        return "Nenhum dispositivo pareado.\nPara parear, abra a página no celular e digite o código de 6 dígitos \
                que o servidor mostra no terminal."
            .to_string();
    }
    let largura = lista.iter().map(|d| d.nome.chars().count()).max().unwrap_or(0).max(4);
    let mut linhas = vec![format!(
        "{:<8}  {:<largura$}  {:<16}  {:<16}  {:<20}  TECLADO",
        "ID", "NOME", "PAREADO EM (UTC)", "VALIDO ATE (UTC)", "VISTO POR ULTIMO (UTC)"
    )];
    for d in lista {
        let teclado = if d.teclado { "sim" } else { "não" };
        let visto = d.ultimo_visto.map_or_else(|| "nunca".to_string(), data_utc);
        linhas.push(format!(
            "{:<8}  {:<largura$}  {:<16}  {:<16}  {:<20}  {teclado}",
            d.id,
            d.nome,
            data_utc(d.criado),
            data_utc(d.expira),
            visto
        ));
    }
    linhas.join("\n")
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::dispositivos::Registro;

    #[test]
    fn o_codigo_aparece_em_dois_grupos_de_tres_digitos() {
        assert_eq!(codigo_agrupado("123456"), "123 456");
        assert_eq!(codigo_agrupado("005924"), "005 924");
    }

    #[test]
    fn codigo_de_tamanho_inesperado_sai_como_veio() {
        assert_eq!(codigo_agrupado("12"), "12");
    }

    #[test]
    fn a_linha_do_codigo_mostra_o_codigo_e_a_contagem_regressiva() {
        let linha = linha_do_codigo("123456", 21);
        assert!(linha.contains("123 456"), "{linha}");
        assert!(linha.contains("21 s"), "{linha}");
        assert!(linha.starts_with("\r\x1b[2K"), "limpa a linha antes de reescrever");
        assert!(!linha.ends_with('\n'), "fica na mesma linha para a contagem andar");
    }

    #[test]
    fn data_em_utc_a_partir_de_segundos() {
        assert_eq!(data_utc(0), "1970-01-01 00:00");
        assert_eq!(data_utc(86_399), "1970-01-01 23:59");
        assert_eq!(data_utc(951_782_400), "2000-02-29 00:00"); // ano bissexto
        assert_eq!(data_utc(1_700_000_000), "2023-11-14 22:13");
    }

    #[test]
    fn tabela_de_dispositivos_lista_id_nome_e_data() {
        let lista = vec![
            Registro { id: "ab12cd34".into(), nome: "iPhone do Wilian".into(), criado: 1_700_000_000, teclado: true, expira: 1_700_000_000 + 90 * 86_400, ultimo_visto: Some(1_700_000_500) },
            Registro { id: "ef56ab78".into(), nome: "iPad".into(), criado: 0, teclado: false, expira: 90 * 86_400, ultimo_visto: None },
        ];
        let t = tabela_de_dispositivos(&lista);
        assert!(t.contains("ab12cd34") && t.contains("iPhone do Wilian") && t.contains("2023-11-14 22:13"), "{t}");
        assert!(t.contains("ef56ab78") && t.contains("iPad"), "{t}");
        assert!(t.lines().count() >= 3, "cabeçalho e uma linha por aparelho");
    }

    #[test]
    fn a_tabela_mostra_quem_pode_usar_o_modo_teclado() {
        let lista = vec![
            Registro { id: "ab12cd34".into(), nome: "iPhone".into(), criado: 0, teclado: true, expira: 90 * 86_400, ultimo_visto: None },
            Registro { id: "ef56ab78".into(), nome: "iPad".into(), criado: 0, teclado: false, expira: 90 * 86_400, ultimo_visto: None },
        ];
        let t = tabela_de_dispositivos(&lista);
        assert!(t.lines().next().unwrap().contains("TECLADO"), "{t}");
        let linha = |id: &str| t.lines().find(|l| l.starts_with(id)).unwrap().to_string();
        assert!(linha("ab12cd34").trim_end().ends_with("sim"), "{t}");
        assert!(linha("ef56ab78").trim_end().ends_with("não"), "{t}");
    }

    #[test]
    fn tabela_vazia_explica_como_parear() {
        let t = tabela_de_dispositivos(&[]);
        assert!(t.to_lowercase().contains("nenhum"), "{t}");
        assert!(t.contains("código"), "{t}");
    }

    #[test]
    fn a_tabela_mostra_a_validade_e_o_ultimo_uso_de_cada_token() {
        let lista = vec![Registro {
            id: "ab12cd34".into(),
            nome: "iPhone".into(),
            criado: 1_700_000_000,
            teclado: false,
            expira: 1_700_000_000 + 90 * 86_400,
            ultimo_visto: Some(1_700_000_500),
        }];
        let t = tabela_de_dispositivos(&lista);
        let cabecalho = t.lines().next().unwrap();
        assert!(cabecalho.contains("VALIDO ATE"), "{t}");
        assert!(cabecalho.contains("VISTO POR ULTIMO"), "{t}");
        assert!(t.contains(&data_utc(1_700_000_000 + 90 * 86_400)), "{t}");
        assert!(t.contains(&data_utc(1_700_000_500)), "{t}");
    }
}
