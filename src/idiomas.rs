//! Idiomas da página de configuração do certificado (a que o iPhone abre para instalar o perfil)
//! e do perfil `.mobileconfig`. O terminal do PC continua em português: quem o lê é o dono do PC.
//!
//! O idioma vem do `?lang=` (a escolha explícita de quem está na página) ou do `Accept-Language`
//! do navegador; uma língua sem tradução recebe o inglês, e sem nenhuma informação vale o português.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Idioma {
    PtBr,
    En,
    Es,
    ZhCn,
    Ja,
    Ko,
}

pub const TODOS: [Idioma; 6] = [Idioma::PtBr, Idioma::En, Idioma::Es, Idioma::ZhCn, Idioma::Ja, Idioma::Ko];

impl Idioma {
    /// A etiqueta BCP 47 (vai no `lang` do HTML e no `?lang=`).
    pub fn codigo(self) -> &'static str {
        match self {
            Idioma::PtBr => "pt-BR",
            Idioma::En => "en",
            Idioma::Es => "es",
            Idioma::ZhCn => "zh-CN",
            Idioma::Ja => "ja",
            Idioma::Ko => "ko",
        }
    }

    /// O nome do idioma na própria língua.
    pub fn nome(self) -> &'static str {
        match self {
            Idioma::PtBr => "Português",
            Idioma::En => "English",
            Idioma::Es => "Español",
            Idioma::ZhCn => "中文",
            Idioma::Ja => "日本語",
            Idioma::Ko => "한국어",
        }
    }

    /// O idioma de uma etiqueta como `pt-BR`, `en-US` ou `zh-Hant-TW` (só o primeiro trecho conta).
    /// Etiquetas com qualquer coisa além de letras, dígitos e hífen são recusadas.
    pub fn por_etiqueta(etiqueta: &str) -> Option<Idioma> {
        let valida = !etiqueta.is_empty()
            && etiqueta.len() <= 16
            && etiqueta.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if !valida {
            return None;
        }
        match etiqueta.split('-').next()?.to_ascii_lowercase().as_str() {
            "pt" => Some(Idioma::PtBr),
            "en" => Some(Idioma::En),
            "es" => Some(Idioma::Es),
            "zh" => Some(Idioma::ZhCn), // todo chinês vira o simplificado: é o único que existe aqui
            "ja" => Some(Idioma::Ja),
            "ko" => Some(Idioma::Ko),
            _ => None,
        }
    }
}

/// Quantos idiomas do `Accept-Language` são considerados: um cabeçalho absurdo não vira trabalho absurdo.
const MAXIMO_DE_ITENS: usize = 32;

/// O primeiro idioma suportado do `Accept-Language`, pela prioridade `q=` (e, empatando, pela ordem).
fn do_cabecalho(cabecalho: &str) -> Idioma {
    let mut candidatos: Vec<(f32, Idioma)> = Vec::new();
    for item in cabecalho.split(',').take(MAXIMO_DE_ITENS) {
        let mut partes = item.split(';');
        let etiqueta = partes.next().unwrap_or("").trim();
        let q = partes
            .filter_map(|p| p.trim().to_ascii_lowercase().strip_prefix("q=").map(str::to_string))
            .find_map(|v| v.trim().parse::<f32>().ok())
            .unwrap_or(1.0);
        if q <= 0.0 {
            continue; // q=0: "não aceito"
        }
        if let Some(idioma) = Idioma::por_etiqueta(etiqueta) {
            candidatos.push((q, idioma));
        }
    }
    // a ordenação é estável: com a mesma prioridade, vale a ordem em que vieram
    candidatos.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    candidatos.first().map_or(Idioma::En, |(_, idioma)| *idioma)
}

/// Escolhe o idioma: o `?lang=` válido vence; depois o primeiro suportado do `Accept-Language`
/// (pela prioridade `q=`); língua sem tradução vira inglês; sem nenhuma informação, português.
pub fn escolher(consulta: Option<&str>, accept_language: Option<&str>) -> Idioma {
    if let Some(idioma) = consulta.and_then(Idioma::por_etiqueta) {
        return idioma;
    }
    accept_language.map_or(Idioma::PtBr, do_cabecalho)
}

/// Os textos da página de configuração. Os campos `*_html` têm marcação (`<b>`) escrita por nós.
pub struct TextosDaPagina {
    pub titulo: &'static str,
    pub cabecalho: &'static str,
    pub introducao: &'static str,
    pub baixar_perfil: &'static str,
    pub passo_permitir_html: &'static str,
    pub passo_instalar_html: &'static str,
    pub passo_confianca_html: &'static str,
    pub abrir_controle: &'static str,
    /// quando o endereço do PC não é conhecido; `{porta}` é a porta HTTPS
    pub abrir_sem_endereco_html: &'static str,
    pub outros_sistemas_html: &'static str,
    pub idioma: &'static str,
}

const PT_BR: TextosDaPagina = TextosDaPagina {
    titulo: "vpad · configurar HTTPS",
    cabecalho: "Configurar o HTTPS neste aparelho",
    introducao: "O vpad protege o acesso com HTTPS. Faça isto uma vez em cada celular.",
    baixar_perfil: "Baixar o perfil",
    passo_permitir_html: "Toque em <b>Permitir</b>.",
    passo_instalar_html: "Abra <b>Ajustes</b> → <b>Perfil baixado</b>, toque no certificado e em <b>Mais detalhes</b>, e confira se a <b>impressão digital SHA-256</b> é igual à mostrada no <b>terminal do PC</b>. Só então toque em <b>Instalar</b>. <b>Não confie</b> em nenhuma impressão digital exibida por esta página: quem estiver interceptando a rede poderia trocá-la.",
    passo_confianca_html: "Abra <b>Ajustes</b> → <b>Geral</b> → <b>Sobre</b> → <b>Ajustes de Confiança de Certificado</b> e ative a confiança em <b>vpad</b>.",
    abrir_controle: "Abrir o controle",
    abrir_sem_endereco_html: "Abra o endereço do PC com <b>https</b> e a porta <b>{porta}</b>.",
    outros_sistemas_html: r#"Android ou computador: baixe o <a href="/ca.crt">certificado</a> e instale-o como autoridade confiável."#,
    idioma: "Idioma",
};

const EN: TextosDaPagina = TextosDaPagina {
    titulo: "vpad · set up HTTPS",
    cabecalho: "Set up HTTPS on this device",
    introducao: "vpad protects access with HTTPS. Do this once on each phone.",
    baixar_perfil: "Download the profile",
    passo_permitir_html: "Tap <b>Allow</b>.",
    passo_instalar_html: "Open <b>Settings</b> → <b>Profile Downloaded</b>, tap the certificate and then <b>More Details</b>, and check that the <b>SHA-256 fingerprint</b> matches the one shown in the <b>PC terminal</b>. Only then tap <b>Install</b>. <b>Do not trust</b> any fingerprint shown by this page: someone intercepting the network could swap it.",
    passo_confianca_html: "Open <b>Settings</b> → <b>General</b> → <b>About</b> → <b>Certificate Trust Settings</b> and turn on trust for <b>vpad</b>.",
    abrir_controle: "Open the controller",
    abrir_sem_endereco_html: "Open the PC's address with <b>https</b> and port <b>{porta}</b>.",
    outros_sistemas_html: r#"Android or computer: download the <a href="/ca.crt">certificate</a> and install it as a trusted authority."#,
    idioma: "Language",
};

const ES: TextosDaPagina = TextosDaPagina {
    titulo: "vpad · configurar HTTPS",
    cabecalho: "Configurar HTTPS en este dispositivo",
    introducao: "vpad protege el acceso con HTTPS. Haz esto una vez en cada teléfono.",
    baixar_perfil: "Descargar el perfil",
    passo_permitir_html: "Toca <b>Permitir</b>.",
    passo_instalar_html: "Abre <b>Ajustes</b> → <b>Perfil descargado</b>, toca el certificado y luego <b>Más detalles</b>, y comprueba que la <b>huella SHA-256</b> coincide con la que se muestra en el <b>terminal del PC</b>. Solo entonces toca <b>Instalar</b>. <b>No te fíes</b> de ninguna huella que muestre esta página: quien intercepte la red podría cambiarla.",
    passo_confianca_html: "Abre <b>Ajustes</b> → <b>General</b> → <b>Información</b> → <b>Ajustes de confianza de certificados</b> y activa la confianza en <b>vpad</b>.",
    abrir_controle: "Abrir el mando",
    abrir_sem_endereco_html: "Abre la dirección del PC con <b>https</b> y el puerto <b>{porta}</b>.",
    outros_sistemas_html: r#"Android u ordenador: descarga el <a href="/ca.crt">certificado</a> e instálalo como autoridad de confianza."#,
    idioma: "Idioma",
};

const ZH_CN: TextosDaPagina = TextosDaPagina {
    titulo: "vpad · 设置 HTTPS",
    cabecalho: "在此设备上设置 HTTPS",
    introducao: "vpad 使用 HTTPS 保护访问。每台手机只需操作一次。",
    baixar_perfil: "下载描述文件",
    passo_permitir_html: "点按<b>允许</b>。",
    passo_instalar_html: "打开<b>设置</b> → <b>已下载描述文件</b>，点按证书，再点按<b>更多详细信息</b>，核对其中的 <b>SHA-256 指纹</b>是否与 <b>PC 终端</b>中显示的一致。确认后再点按<b>安装</b>。<b>不要信任</b>本页面显示的任何指纹：监听网络的人可能会替换它。",
    passo_confianca_html: "打开<b>设置</b> → <b>通用</b> → <b>关于本机</b> → <b>证书信任设置</b>，并为 <b>vpad</b> 开启信任。",
    abrir_controle: "打开控制器",
    abrir_sem_endereco_html: "请用 <b>https</b> 和端口 <b>{porta}</b> 打开 PC 的地址。",
    outros_sistemas_html: r#"Android 或电脑：下载<a href="/ca.crt">证书</a>并将其安装为受信任的证书颁发机构。"#,
    idioma: "语言",
};

const JA: TextosDaPagina = TextosDaPagina {
    titulo: "vpad · HTTPS の設定",
    cabecalho: "このデバイスで HTTPS を設定",
    introducao: "vpad は HTTPS でアクセスを保護します。各スマートフォンで一度だけ行ってください。",
    baixar_perfil: "プロファイルをダウンロード",
    passo_permitir_html: "<b>許可</b>をタップします。",
    passo_instalar_html: "<b>設定</b> → <b>プロファイルがダウンロードされました</b>を開き、証明書をタップして詳細を表示し、<b>SHA-256 フィンガープリント</b>が <b>PC のターミナル</b>に表示されているものと同じか確認してください。確認できてから<b>インストール</b>をタップします。このページに表示されるフィンガープリントは<b>信用しないでください</b>。ネットワークを傍受している人に差し替えられる可能性があります。",
    passo_confianca_html: "<b>設定</b> → <b>一般</b> → <b>情報</b> → <b>証明書信頼設定</b>を開き、<b>vpad</b> を信頼する設定をオンにします。",
    abrir_controle: "コントローラーを開く",
    abrir_sem_endereco_html: "PC のアドレスを <b>https</b>、ポート <b>{porta}</b> で開いてください。",
    outros_sistemas_html: r#"Android またはパソコン：<a href="/ca.crt">証明書</a>をダウンロードし、信頼できる認証局としてインストールしてください。"#,
    idioma: "言語",
};

const KO: TextosDaPagina = TextosDaPagina {
    titulo: "vpad · HTTPS 설정",
    cabecalho: "이 기기에서 HTTPS 설정",
    introducao: "vpad는 HTTPS로 접속을 보호합니다. 각 휴대폰에서 한 번만 하면 됩니다.",
    baixar_perfil: "프로파일 다운로드",
    passo_permitir_html: "<b>허용</b>을 탭하세요.",
    passo_instalar_html: "<b>설정</b> → <b>프로파일이 다운로드됨</b>을 열고 인증서를 탭하여 세부 정보를 확인한 뒤, <b>SHA-256 지문</b>이 <b>PC 터미널</b>에 표시된 것과 같은지 확인하세요. 그런 다음에만 <b>설치</b>를 탭하세요. 이 페이지에 표시되는 지문은 <b>믿지 마세요</b>. 네트워크를 가로채는 사람이 바꿀 수 있습니다.",
    passo_confianca_html: "<b>설정</b> → <b>일반</b> → <b>정보</b> → <b>인증서 신뢰 설정</b>을 열고 <b>vpad</b>에 대한 신뢰를 켜세요.",
    abrir_controle: "컨트롤러 열기",
    abrir_sem_endereco_html: "PC 주소를 <b>https</b>와 포트 <b>{porta}</b>로 여세요.",
    outros_sistemas_html: r#"Android 또는 컴퓨터: <a href="/ca.crt">인증서</a>를 다운로드하여 신뢰할 수 있는 인증 기관으로 설치하세요."#,
    idioma: "언어",
};

pub fn pagina(idioma: Idioma) -> &'static TextosDaPagina {
    match idioma {
        Idioma::PtBr => &PT_BR,
        Idioma::En => &EN,
        Idioma::Es => &ES,
        Idioma::ZhCn => &ZH_CN,
        Idioma::Ja => &JA,
        Idioma::Ko => &KO,
    }
}

/// Descrições que o iPhone mostra ao instalar o perfil: a da autoridade e a do perfil
/// (`{nome}` é o nome do PC).
pub fn descricoes_do_perfil(idioma: Idioma) -> (&'static str, &'static str) {
    match idioma {
        Idioma::PtBr => (
            "Autoridade de certificação local do vpad, para abrir o controle por HTTPS na rede de casa.",
            "Permite ao iPhone confiar no HTTPS do vpad em {nome}.",
        ),
        Idioma::En => (
            "vpad local certificate authority, to open the controller over HTTPS on your home network.",
            "Lets the iPhone trust vpad's HTTPS on {nome}.",
        ),
        Idioma::Es => (
            "Autoridad de certificación local de vpad, para abrir el mando por HTTPS en la red de casa.",
            "Permite que el iPhone confíe en el HTTPS de vpad en {nome}.",
        ),
        Idioma::ZhCn => (
            "vpad 本地证书颁发机构，用于在家庭网络中通过 HTTPS 打开控制器。",
            "允许 iPhone 信任 {nome} 上 vpad 的 HTTPS。",
        ),
        Idioma::Ja => (
            "家庭内ネットワークで HTTPS 経由でコントローラーを開くための vpad のローカル認証局です。",
            "iPhone が {nome} 上の vpad の HTTPS を信頼できるようにします。",
        ),
        Idioma::Ko => (
            "집 네트워크에서 HTTPS로 컨트롤러를 열기 위한 vpad 로컬 인증 기관입니다.",
            "iPhone이 {nome}의 vpad HTTPS를 신뢰하도록 합니다.",
        ),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cada_idioma_tem_codigo_e_nome_na_propria_lingua() {
        let pares: Vec<_> = TODOS.iter().map(|i| (i.codigo(), i.nome())).collect();
        assert_eq!(
            pares,
            [
                ("pt-BR", "Português"),
                ("en", "English"),
                ("es", "Español"),
                ("zh-CN", "中文"),
                ("ja", "日本語"),
                ("ko", "한국어"),
            ]
        );
    }

    #[test]
    fn etiquetas_comuns_viram_o_idioma_certo() {
        for (etiqueta, esperado) in [
            ("pt-BR", Idioma::PtBr),
            ("pt", Idioma::PtBr),
            ("PT-br", Idioma::PtBr),
            ("pt-PT", Idioma::PtBr),
            ("en-US", Idioma::En),
            ("en-GB", Idioma::En),
            ("es-MX", Idioma::Es),
            ("es", Idioma::Es),
            ("zh-CN", Idioma::ZhCn),
            ("zh-Hans-CN", Idioma::ZhCn),
            ("zh-TW", Idioma::ZhCn),
            ("ja-JP", Idioma::Ja),
            ("ko-KR", Idioma::Ko),
            ("KO", Idioma::Ko),
        ] {
            assert_eq!(Idioma::por_etiqueta(etiqueta), Some(esperado), "{etiqueta}");
        }
    }

    #[test]
    fn etiqueta_desconhecida_ou_estranha_nao_vira_nada() {
        for etiqueta in ["fr-FR", "de", "", "*", "../etc/passwd", "<script>", "pt BR", &"a".repeat(500)] {
            assert_eq!(Idioma::por_etiqueta(etiqueta), None, "{etiqueta:?}");
        }
    }

    #[test]
    fn a_consulta_valida_vence_o_cabecalho() {
        assert_eq!(escolher(Some("ko"), Some("en-US")), Idioma::Ko);
        assert_eq!(escolher(Some("ja-JP"), None), Idioma::Ja);
    }

    #[test]
    fn consulta_invalida_e_ignorada_e_vale_o_cabecalho() {
        assert_eq!(escolher(Some("xx"), Some("es-ES")), Idioma::Es);
        assert_eq!(escolher(Some("<script>"), Some("ja")), Idioma::Ja);
        assert_eq!(escolher(Some(""), Some("ko")), Idioma::Ko);
    }

    #[test]
    fn o_cabecalho_respeita_a_prioridade_q() {
        assert_eq!(escolher(None, Some("en;q=0.5, ja;q=0.9")), Idioma::Ja);
        assert_eq!(escolher(None, Some("fr, ko;q=0.8, es;q=0.7")), Idioma::Ko);
        assert_eq!(escolher(None, Some("ja-JP,ja;q=0.9,en-US;q=0.8,en;q=0.7")), Idioma::Ja);
    }

    #[test]
    fn na_falta_de_q_vale_a_ordem_em_que_vieram() {
        assert_eq!(escolher(None, Some("es, en")), Idioma::Es);
        assert_eq!(escolher(None, Some("fr, de, zh-CN")), Idioma::ZhCn);
    }

    #[test]
    fn q_zero_significa_nao_aceito() {
        assert_eq!(escolher(None, Some("ja;q=0, en")), Idioma::En);
    }

    #[test]
    fn lingua_sem_traducao_cai_para_o_ingles() {
        assert_eq!(escolher(None, Some("fr-FR, de;q=0.8")), Idioma::En);
        assert_eq!(escolher(None, Some("*")), Idioma::En);
        assert_eq!(escolher(None, Some("")), Idioma::En);
    }

    #[test]
    fn sem_cabecalho_nem_consulta_vale_o_portugues() {
        assert_eq!(escolher(None, None), Idioma::PtBr);
    }

    #[test]
    fn cabecalho_gigante_ou_malformado_nao_quebra() {
        let enorme = "x;q=abc,".repeat(5000);
        assert_eq!(escolher(None, Some(&enorme)), Idioma::En);
        assert_eq!(escolher(None, Some(";;;,,,q=,;q=;;")), Idioma::En);
        let com_japones_no_fim = format!("{}ja", "fr,".repeat(5000));
        // só os primeiros itens são considerados: um cabeçalho absurdo não vira trabalho absurdo
        assert_eq!(escolher(None, Some(&com_japones_no_fim)), Idioma::En);
    }

    #[test]
    fn todo_idioma_tem_todos_os_textos_preenchidos() {
        for i in TODOS {
            let t = pagina(i);
            for (nome, texto) in [
                ("titulo", t.titulo),
                ("cabecalho", t.cabecalho),
                ("introducao", t.introducao),
                ("baixar_perfil", t.baixar_perfil),
                ("passo_permitir_html", t.passo_permitir_html),
                ("passo_instalar_html", t.passo_instalar_html),
                ("passo_confianca_html", t.passo_confianca_html),
                ("abrir_controle", t.abrir_controle),
                ("abrir_sem_endereco_html", t.abrir_sem_endereco_html),
                ("outros_sistemas_html", t.outros_sistemas_html),
                ("idioma", t.idioma),
            ] {
                assert!(!texto.trim().is_empty(), "{:?}: {nome}", i);
            }
        }
    }

    #[test]
    fn a_marcacao_dos_textos_esta_balanceada_e_so_tem_o_permitido() {
        for i in TODOS {
            let t = pagina(i);
            for texto in [t.passo_permitir_html, t.passo_instalar_html, t.passo_confianca_html, t.abrir_sem_endereco_html, t.outros_sistemas_html] {
                assert_eq!(texto.matches("<b>").count(), texto.matches("</b>").count(), "{i:?}: {texto}");
                let sem_permitidas = texto.replace("<b>", "").replace("</b>", "").replace(r#"<a href="/ca.crt">"#, "").replace("</a>", "");
                assert!(!sem_permitidas.contains('<') && !sem_permitidas.contains('>'), "{i:?}: marcação inesperada em {texto}");
            }
        }
    }

    #[test]
    fn o_passo_sem_endereco_leva_a_porta_e_o_dos_outros_sistemas_leva_o_link_do_certificado() {
        for i in TODOS {
            let t = pagina(i);
            assert!(t.abrir_sem_endereco_html.contains("{porta}"), "{i:?}");
            assert!(t.abrir_sem_endereco_html.contains("https"), "{i:?}");
            assert!(t.outros_sistemas_html.contains(r#"<a href="/ca.crt">"#), "{i:?}");
        }
    }

    #[test]
    fn cada_idioma_usa_os_nomes_do_ios_na_propria_lingua() {
        for (i, trecho) in [
            (Idioma::PtBr, "Ajustes de Confiança de Certificado"),
            (Idioma::En, "Certificate Trust Settings"),
            (Idioma::Es, "Ajustes de confianza de certificados"),
            (Idioma::ZhCn, "证书信任设置"),
            (Idioma::Ja, "証明書信頼設定"),
            (Idioma::Ko, "인증서 신뢰 설정"),
        ] {
            assert!(pagina(i).passo_confianca_html.contains(trecho), "{i:?}");
        }
    }

    #[test]
    fn o_passo_da_impressao_digital_manda_conferir_no_terminal_e_nao_confiar_na_pagina() {
        for (i, terminal, nao_confiar) in [
            (Idioma::PtBr, "terminal do PC", "Não confie"),
            (Idioma::En, "PC terminal", "Do not trust"),
            (Idioma::Es, "terminal del PC", "No te fíes"),
            (Idioma::ZhCn, "PC 终端", "不要信任"),
            (Idioma::Ja, "PC のターミナル", "信用しないでください"),
            (Idioma::Ko, "PC 터미널", "믿지 마세요"),
        ] {
            let passo = pagina(i).passo_instalar_html;
            assert!(passo.contains(terminal), "{i:?}");
            assert!(passo.contains(nao_confiar), "{i:?}");
            assert!(passo.contains("SHA-256"), "{i:?}");
        }
    }

    #[test]
    fn as_descricoes_do_perfil_existem_nos_seis_idiomas() {
        for i in TODOS {
            let (autoridade, perfil) = descricoes_do_perfil(i);
            assert!(autoridade.contains("vpad") && perfil.contains("vpad"), "{i:?}");
            assert!(perfil.contains("{nome}"), "{i:?}");
            assert!(!autoridade.contains('<') && !perfil.contains('<'), "{i:?}");
        }
    }
}
