//! Perfil de configuração do iOS (.mobileconfig) que instala a autoridade do vpad como raiz.
//! Depois de instalado, o iOS ainda exige que o usuário ative a confiança total em Ajustes →
//! Geral → Sobre → Ajustes de Confiança de Certificado: não há como automatizar isso.

use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};

fn xml(texto: &str) -> String {
    texto
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// UUID estável derivado do certificado: reinstalar o mesmo perfil o atualiza, não duplica.
fn uuid_de(finalidade: &str, der: &[u8]) -> String {
    let hash = Sha256::digest([finalidade.as_bytes(), der].concat());
    let h: String = hash[..16].iter().map(|b| format!("{b:02X}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// O `.mobileconfig` que instala `ca_der` (certificado da autoridade, em DER).
pub fn perfil_ios(ca_der: &[u8], nome_do_pc: &str) -> String {
    let uuid_certificado = uuid_de("certificado", ca_der);
    let uuid_perfil = uuid_de("perfil", ca_der);
    let nome = xml(nome_do_pc);
    let dados = STANDARD.encode(ca_der);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>PayloadContent</key>
  <array>
    <dict>
      <key>PayloadCertificateFileName</key>
      <string>vpad-ca.cer</string>
      <key>PayloadContent</key>
      <data>{dados}</data>
      <key>PayloadDescription</key>
      <string>Autoridade de certificação local do vpad, para abrir o controle por HTTPS na rede de casa.</string>
      <key>PayloadDisplayName</key>
      <string>vpad ({nome})</string>
      <key>PayloadIdentifier</key>
      <string>com.vpad.ca.{uuid_certificado}</string>
      <key>PayloadType</key>
      <string>com.apple.security.root</string>
      <key>PayloadUUID</key>
      <string>{uuid_certificado}</string>
      <key>PayloadVersion</key>
      <integer>1</integer>
    </dict>
  </array>
  <key>PayloadDescription</key>
  <string>Permite ao iPhone confiar no HTTPS do vpad em {nome}.</string>
  <key>PayloadDisplayName</key>
  <string>vpad ({nome})</string>
  <key>PayloadIdentifier</key>
  <string>com.vpad.perfil.{uuid_perfil}</string>
  <key>PayloadRemovalDisallowed</key>
  <false/>
  <key>PayloadType</key>
  <string>Configuration</string>
  <key>PayloadUUID</key>
  <string>{uuid_perfil}</string>
  <key>PayloadVersion</key>
  <integer>1</integer>
</dict>
</plist>
"#
    )
}

#[cfg(test)]
mod testes {
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD};

    const DER: &[u8] = &[0x30, 0x82, 0x01, 0x0a, 0xde, 0xad, 0xbe, 0xef, 0x00, 0x01, 0x02, 0x03];

    fn tags_equilibradas(xml: &str, tag: &str) -> bool {
        xml.matches(&format!("<{tag}>")).count() == xml.matches(&format!("</{tag}>")).count()
    }

    fn uuids(xml: &str) -> Vec<String> {
        xml.split("<key>PayloadUUID</key>")
            .skip(1)
            .map(|r| r.split("<string>").nth(1).unwrap().split("</string>").next().unwrap().to_string())
            .collect()
    }

    #[test]
    fn e_um_plist_que_instala_a_autoridade_como_raiz_confiavel() {
        let xml = perfil_ios(DER, "meu-pc");
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<plist version=\"1.0\">"));
        assert!(xml.contains("<string>com.apple.security.root</string>"));
        assert!(xml.contains("<string>Configuration</string>"));
    }

    #[test]
    fn leva_o_certificado_em_base64() {
        let xml = perfil_ios(DER, "meu-pc");
        let esperado = STANDARD.encode(DER);
        let limpo: String = xml.split_whitespace().collect();
        assert!(limpo.contains(&format!("<data>{esperado}</data>")), "{xml}");
    }

    #[test]
    fn estrutura_bem_formada() {
        let xml = perfil_ios(DER, "meu-pc");
        for tag in ["dict", "array"] {
            assert!(tags_equilibradas(&xml, tag), "{tag}");
        }
        assert_eq!(xml.matches("<plist ").count(), xml.matches("</plist>").count());
    }

    #[test]
    fn os_identificadores_sao_estaveis_para_a_mesma_autoridade() {
        // instalar de novo o mesmo perfil atualiza em vez de duplicar
        assert_eq!(perfil_ios(DER, "pc"), perfil_ios(DER, "pc"));
    }

    #[test]
    fn outra_autoridade_gera_outros_identificadores() {
        let outro = [DER, &[0x99]].concat();
        assert_ne!(uuids(&perfil_ios(DER, "pc")), uuids(&perfil_ios(&outro, "pc")));
    }

    #[test]
    fn os_uuids_tem_o_formato_certo_e_sao_distintos() {
        let ids = uuids(&perfil_ios(DER, "pc"));
        assert_eq!(ids.len(), 2, "um do perfil e um do certificado");
        assert_ne!(ids[0], ids[1]);
        for id in ids {
            let partes: Vec<_> = id.split('-').map(str::len).collect();
            assert_eq!(partes, [8, 4, 4, 4, 12], "{id}");
            assert!(id.chars().all(|c| c == '-' || c.is_ascii_hexdigit() && !c.is_ascii_lowercase()), "{id}");
        }
    }

    #[test]
    fn o_nome_do_pc_e_escapado_para_nao_quebrar_o_xml() {
        let xml = perfil_ios(DER, "pc <&> \"x\"");
        assert!(!xml.contains("pc <&>"));
        assert!(xml.contains("pc &lt;&amp;&gt; &quot;x&quot;"));
        assert!(tags_equilibradas(&xml, "dict"));
    }

    #[test]
    fn nao_impede_o_usuario_de_remover_o_perfil() {
        let xml = perfil_ios(DER, "pc");
        assert!(xml.contains("<key>PayloadRemovalDisallowed</key>"));
        assert!(xml.contains("<false/>"));
        assert!(!xml.contains("<true/>"));
    }
}
