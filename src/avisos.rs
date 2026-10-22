//! Os avisos que o `main` mostra no terminal do dono do PC, em funções puras (para testar os dois
//! idiomas). O texto em português é o de sempre; o idioma vem de `mensagens`.

use crate::{
    mensagens::{em_portugues, txt},
    tr,
};

pub fn sem_home_para_aparelhos() -> String {
    txt("aviso: sem HOME, os aparelhos pareados não serão salvos", "warning: no HOME, paired devices will not be saved").to_string()
}

pub fn dispositivos_revogados(quantos: usize) -> String {
    tr!("{quantos} dispositivo(s) revogado(s)", "{quantos} device(s) revoked")
}

pub fn dispositivo_revogado(id: &str) -> String {
    tr!("dispositivo {id} revogado", "device {id} revoked")
}

pub fn nenhum_dispositivo_com_o_id(id: &str) -> String {
    tr!(
        "nenhum dispositivo com o id {id} (veja os ids com: vpad --dispositivos)",
        "no device with the id {id} (see the ids with: vpad --dispositivos)"
    )
}

pub fn modo_teclado_do_dispositivo(permitir: bool, id: &str) -> String {
    let estado = match (permitir, em_portugues()) {
        (true, true) => "liberado",
        (false, true) => "bloqueado",
        (true, false) => "allowed",
        (false, false) => "blocked",
    };
    tr!("modo teclado {estado} para o dispositivo {id}", "keyboard mode {estado} for the device {id}")
}

pub fn teclas_salvas_em(caminho: &str) -> String {
    tr!("teclas salvas em {caminho}", "keys saved in {caminho}")
}

pub fn layout_salvo_em(caminho: &str) -> String {
    tr!("layout dos botões salvo em {caminho}", "button layout saved in {caminho}")
}

/// As três linhas quando o controle virtual não pôde ser criado.
pub fn nao_criou_o_controle_virtual(erro: &str) -> [String; 3] {
    [
        tr!("não consegui criar o controle virtual: {erro}", "could not create the virtual controller: {erro}"),
        txt(
            "verifique se o módulo uinput está carregado (sudo modprobe uinput)",
            "check that the uinput module is loaded (sudo modprobe uinput)",
        )
        .to_string(),
        txt(
            "e se seu usuário tem permissão de escrita em /dev/uinput",
            "and that your user has write permission on /dev/uinput",
        )
        .to_string(),
    ]
}

pub fn sem_inotify(erro: &str, segundos: u64) -> String {
    tr!(
        "aviso: sem inotify ({erro}); a revogação leva até {segundos} s",
        "warning: no inotify ({erro}); revocation takes up to {segundos} s"
    )
}

pub fn sem_home_para_o_certificado() -> String {
    txt(
        "sem HOME não há onde guardar o certificado; defina HOME ou XDG_CONFIG_HOME",
        "no HOME, so there is nowhere to keep the certificate; set HOME or XDG_CONFIG_HOME",
    )
    .to_string()
}

pub fn linha_do_perfil(nome: &str, fabricante: u16, produto: u16) -> String {
    tr!("perfil: {nome} ({fabricante:04x}:{produto:04x})", "profile: {nome} ({fabricante:04x}:{produto:04x})")
}

pub fn controle_https(url: &str) -> String {
    tr!("controle (HTTPS): {url}", "controller (HTTPS): {url}")
}

/// As duas linhas quando a autoridade antiga (sem restrição de nomes) foi substituída.
pub fn autoridade_antiga_substituida() -> [String; 2] {
    [
        txt(
            "AUTORIDADE ANTIGA (sem restrição de nomes) SUBSTITUÍDA por uma que só vale para a rede local:",
            "OLD AUTHORITY (without name restrictions) REPLACED by one that is only valid for the local network:",
        )
        .to_string(),
        txt(
            "remova o perfil vpad antigo do celular e instale o novo",
            "remove the old vpad profile from the phone and install the new one",
        )
        .to_string(),
    ]
}

pub fn autoridade_nova() -> String {
    txt(
        "AUTORIDADE DE CERTIFICAÇÃO NOVA: instale o perfil de novo em cada celular que já tinha o antigo",
        "NEW CERTIFICATE AUTHORITY: install the profile again on every phone that had the old one",
    )
    .to_string()
}

pub fn certificado_renovado() -> String {
    txt(
        "certificado do servidor renovado (IP novo ou perto de vencer); os celulares não precisam fazer nada",
        "server certificate renewed (new IP or close to expiring); phones do not need to do anything",
    )
    .to_string()
}

pub fn impressao_digital(primeira: &str, segunda: &str) -> String {
    tr!(
        "impressão digital (SHA-256) da autoridade, para conferir no celular antes de instalar o perfil:\n  {primeira}\n  {segunda}",
        "authority fingerprint (SHA-256), to check on the phone before installing the profile:\n  {primeira}\n  {segunda}"
    )
}

pub fn ip_fora_do_certificado(ip: &str) -> String {
    tr!(
        "aviso: o IP {ip} não é de rede local e ficou fora do certificado",
        "warning: the IP {ip} is not on a local network and was left out of the certificate"
    )
}

pub fn certificado_valido_para(lista: &str) -> String {
    tr!("certificado válido para: {lista}", "certificate valid for: {lista}")
}

pub fn aparelhos_pareados(quantos: usize) -> String {
    tr!(
        "aparelhos pareados: {quantos} (veja com: vpad --dispositivos)",
        "paired devices: {quantos} (see them with: vpad --dispositivos)"
    )
}

pub fn pagina_de_configuracao_aberta(segundos: u64, url: &str) -> String {
    tr!(
        "página de configuração do certificado aberta por {segundos} s: abra no celular {url}",
        "certificate setup page open for {segundos} s: open on the phone {url}"
    )
}

pub fn nao_abriu_a_pagina_de_configuracao(erro: &str) -> String {
    tr!("não consegui abrir a página de configuração: {erro}", "could not open the setup page: {erro}")
}

pub fn sem_entrada_de_terminal() -> String {
    txt(
        "\r\x1b[2Kaviso: sem entrada de terminal; o pareamento só abre com a opção --parear",
        "\r\x1b[2Kwarning: no terminal input; pairing only opens with the --parear option",
    )
    .to_string()
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::{idiomas::Idioma, mensagens::com_idioma};

    fn em(idioma: Idioma, f: impl FnOnce() -> String) -> String {
        com_idioma(idioma, f)
    }

    /// (função em português, função em inglês) devolvem exatamente estes textos.
    #[test]
    fn avisos_do_aparelho_e_do_modo_teclado() {
        let pt = Idioma::PtBr;
        let en = Idioma::En;
        assert_eq!(em(pt, sem_home_para_aparelhos), "aviso: sem HOME, os aparelhos pareados não serão salvos");
        assert_eq!(em(en, sem_home_para_aparelhos), "warning: no HOME, paired devices will not be saved");
        assert_eq!(em(pt, || dispositivos_revogados(3)), "3 dispositivo(s) revogado(s)");
        assert_eq!(em(en, || dispositivos_revogados(3)), "3 device(s) revoked");
        assert_eq!(em(pt, || dispositivo_revogado("ab12")), "dispositivo ab12 revogado");
        assert_eq!(em(en, || dispositivo_revogado("ab12")), "device ab12 revoked");
        assert_eq!(
            em(pt, || nenhum_dispositivo_com_o_id("zz")),
            "nenhum dispositivo com o id zz (veja os ids com: vpad --dispositivos)"
        );
        assert_eq!(
            em(en, || nenhum_dispositivo_com_o_id("zz")),
            "no device with the id zz (see the ids with: vpad --dispositivos)"
        );
        assert_eq!(em(pt, || modo_teclado_do_dispositivo(true, "ab")), "modo teclado liberado para o dispositivo ab");
        assert_eq!(em(pt, || modo_teclado_do_dispositivo(false, "ab")), "modo teclado bloqueado para o dispositivo ab");
        assert_eq!(em(en, || modo_teclado_do_dispositivo(true, "ab")), "keyboard mode allowed for the device ab");
        assert_eq!(em(en, || modo_teclado_do_dispositivo(false, "ab")), "keyboard mode blocked for the device ab");
    }

    #[test]
    fn avisos_de_arquivos_e_do_controle_virtual() {
        let (pt, en) = (Idioma::PtBr, Idioma::En);
        assert_eq!(em(pt, || teclas_salvas_em("/x/teclado.json")), "teclas salvas em /x/teclado.json");
        assert_eq!(em(en, || teclas_salvas_em("/x/teclado.json")), "keys saved in /x/teclado.json");
        assert_eq!(em(pt, || layout_salvo_em("/x/layout.json")), "layout dos botões salvo em /x/layout.json");
        assert_eq!(em(en, || layout_salvo_em("/x/layout.json")), "button layout saved in /x/layout.json");

        let pt3 = com_idioma(pt, || nao_criou_o_controle_virtual("permissão negada"));
        assert_eq!(pt3[0], "não consegui criar o controle virtual: permissão negada");
        assert_eq!(pt3[1], "verifique se o módulo uinput está carregado (sudo modprobe uinput)");
        assert_eq!(pt3[2], "e se seu usuário tem permissão de escrita em /dev/uinput");
        let en3 = com_idioma(en, || nao_criou_o_controle_virtual("permission denied"));
        assert_eq!(en3[0], "could not create the virtual controller: permission denied");
        assert_eq!(en3[1], "check that the uinput module is loaded (sudo modprobe uinput)");
        assert_eq!(en3[2], "and that your user has write permission on /dev/uinput");

        assert_eq!(em(pt, || sem_inotify("x", 2)), "aviso: sem inotify (x); a revogação leva até 2 s");
        assert_eq!(em(en, || sem_inotify("x", 2)), "warning: no inotify (x); revocation takes up to 2 s");
        assert_eq!(
            em(pt, sem_home_para_o_certificado),
            "sem HOME não há onde guardar o certificado; defina HOME ou XDG_CONFIG_HOME"
        );
        assert_eq!(
            em(en, sem_home_para_o_certificado),
            "no HOME, so there is nowhere to keep the certificate; set HOME or XDG_CONFIG_HOME"
        );
    }

    #[test]
    fn avisos_do_certificado_e_da_partida() {
        let (pt, en) = (Idioma::PtBr, Idioma::En);
        assert_eq!(em(pt, || linha_do_perfil("vpad Xbox", 0x045e, 0x028e)), "perfil: vpad Xbox (045e:028e)");
        assert_eq!(em(en, || linha_do_perfil("vpad Xbox", 0x045e, 0x028e)), "profile: vpad Xbox (045e:028e)");
        assert_eq!(em(pt, || controle_https("https://1.2.3.4:8443/")), "controle (HTTPS): https://1.2.3.4:8443/");
        assert_eq!(em(en, || controle_https("https://1.2.3.4:8443/")), "controller (HTTPS): https://1.2.3.4:8443/");

        let antiga_pt = com_idioma(pt, autoridade_antiga_substituida);
        assert_eq!(antiga_pt[0], "AUTORIDADE ANTIGA (sem restrição de nomes) SUBSTITUÍDA por uma que só vale para a rede local:");
        assert_eq!(antiga_pt[1], "remova o perfil vpad antigo do celular e instale o novo");
        let antiga_en = com_idioma(en, autoridade_antiga_substituida);
        assert_eq!(antiga_en[0], "OLD AUTHORITY (without name restrictions) REPLACED by one that is only valid for the local network:");
        assert_eq!(antiga_en[1], "remove the old vpad profile from the phone and install the new one");

        assert_eq!(
            em(pt, autoridade_nova),
            "AUTORIDADE DE CERTIFICAÇÃO NOVA: instale o perfil de novo em cada celular que já tinha o antigo"
        );
        assert_eq!(
            em(en, autoridade_nova),
            "NEW CERTIFICATE AUTHORITY: install the profile again on every phone that had the old one"
        );
        assert_eq!(
            em(pt, certificado_renovado),
            "certificado do servidor renovado (IP novo ou perto de vencer); os celulares não precisam fazer nada"
        );
        assert_eq!(
            em(en, certificado_renovado),
            "server certificate renewed (new IP or close to expiring); phones do not need to do anything"
        );
        assert_eq!(
            em(pt, || impressao_digital("AA:BB", "CC:DD")),
            "impressão digital (SHA-256) da autoridade, para conferir no celular antes de instalar o perfil:\n  AA:BB\n  CC:DD"
        );
        assert_eq!(
            em(en, || impressao_digital("AA:BB", "CC:DD")),
            "authority fingerprint (SHA-256), to check on the phone before installing the profile:\n  AA:BB\n  CC:DD"
        );
        assert_eq!(
            em(pt, || ip_fora_do_certificado("8.8.8.8")),
            "aviso: o IP 8.8.8.8 não é de rede local e ficou fora do certificado"
        );
        assert_eq!(
            em(en, || ip_fora_do_certificado("8.8.8.8")),
            "warning: the IP 8.8.8.8 is not on a local network and was left out of the certificate"
        );
        assert_eq!(em(pt, || certificado_valido_para("1.1.1.1, 2.2.2.2")), "certificado válido para: 1.1.1.1, 2.2.2.2");
        assert_eq!(em(en, || certificado_valido_para("1.1.1.1, 2.2.2.2")), "certificate valid for: 1.1.1.1, 2.2.2.2");
        assert_eq!(em(pt, || aparelhos_pareados(2)), "aparelhos pareados: 2 (veja com: vpad --dispositivos)");
        assert_eq!(em(en, || aparelhos_pareados(2)), "paired devices: 2 (see them with: vpad --dispositivos)");
        assert_eq!(
            em(pt, || pagina_de_configuracao_aberta(300, "http://1.2.3.4:8080/")),
            "página de configuração do certificado aberta por 300 s: abra no celular http://1.2.3.4:8080/"
        );
        assert_eq!(
            em(en, || pagina_de_configuracao_aberta(300, "http://1.2.3.4:8080/")),
            "certificate setup page open for 300 s: open on the phone http://1.2.3.4:8080/"
        );
        assert_eq!(
            em(pt, || nao_abriu_a_pagina_de_configuracao("porta ocupada")),
            "não consegui abrir a página de configuração: porta ocupada"
        );
        assert_eq!(
            em(en, || nao_abriu_a_pagina_de_configuracao("port in use")),
            "could not open the setup page: port in use"
        );
        assert_eq!(
            em(pt, sem_entrada_de_terminal),
            "\r\x1b[2Kaviso: sem entrada de terminal; o pareamento só abre com a opção --parear"
        );
        assert_eq!(
            em(en, sem_entrada_de_terminal),
            "\r\x1b[2Kwarning: no terminal input; pairing only opens with the --parear option"
        );
    }

    #[test]
    fn a_pagina_do_certificado_tem_o_mesmo_texto_do_console() {
        // o `c` do console e a abertura no começo usam a mesma frase (não podem divergir)
        let do_console = com_idioma(Idioma::En, || crate::tr!(
            "página de configuração do certificado aberta por {} s: abra no celular {}",
            "certificate setup page open for {} s: open on the phone {}",
            300,
            "http://x/"
        ));
        assert_eq!(do_console, em(Idioma::En, || pagina_de_configuracao_aberta(300, "http://x/")));
    }
}
