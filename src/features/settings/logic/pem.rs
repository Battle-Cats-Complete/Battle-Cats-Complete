use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::{RsaPrivateKey, Pkcs1v15Sign};
use sha2::{Digest, Sha256};
use rasn_pkix::{Certificate, SubjectPublicKeyInfo};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

pub const DEFAULT_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCmBNx3G6wn5h63
9cvUxyul2ik3/a4uBBfmGAccldsdawLzg4X7y4nYvBjNo1KWWnKekIWnDHxULtH3
zwEwRAZPFmNPwKvJ3pwYlUE/RvunAVM3PuLGnAFSmDghE3Sylc02HitS0qrWuW/Z
wWPjLIUkmLD/CQnqA1eZL5io+KOZpYx6+iD9XW9aR2ANdHX/813tnGp0HPelUhBg
tFuDdjAJvuzXhhQWFncvYmD5u2wqDVES3o7PkitCFM9xCrg35bIvpTBvyltca2cw
uNmngx3sldU0G0MqmsCpwRvgun9f3vMtlQ3KXEzN+dYP6oYTOIlpUT9pjwCod9yk
CbtRXcn1AgMBAAECggEAH4K2sai/8Ua9N99gU7+F6lHRFv6AS92dB6Ax4VwUHa5M
/hlNmfAU9t0kvAsuxrjeHniB1aYKBxRn5+gTaqzOob43FVEVihhFemkB3FfFtfoL
aGX4NwgvPBUGOkjuEmNactYhFPRFVsIVl7gcFGdD0iFlHtMBXbhKrRmamR+wNZ4m
+dgOWCocvpCMz5/xtxapEfKL+PouHjOonWLLPET+Ire7k+AprW2z3Ww6eZvkc5OU
FJnOM22aznnloQV8rIfG3ZRF2znQQ5uUS8F7ER+OdAE0i5cAbWGGUQ3JGJFgrTMI
A572fhcz+un4/cqPJoC4fYSiNTgXyZ5vKWiqMN1qgQKBgQDaozQ8KVw7F5iBYl/Y
4ZXsWLUs7TIe2bKWhE+3huTyuPzeGZwQ7T8trxRR4DjCqRpdyGJCzXYFbHPaqfee
INEfoXiDJcfVBLGNPpEC/ahc/lPmB/XOsrLsVQ8+hXV32ohLfa3nE/YZsUtdQnyH
Zj1v1xNfo7zIyu3Wf8hU7omxlQKBgQDCY71pLSZVRzRokvxiMjennQUVZ6xSUKOO
AhvQcGOqhW0TLLl46JoDXEmIjFIxp3mYOAb40TxE3jzJ/hqzzhfGXpw8BlPegCYw
UKpiRMqwZJ9mNsEqiRyf3AJPfQMF3M+0ablgxM/RJZLAgnGeQU6DNKjwbOMNiZEB
WYAobZRe4QKBgGasyCYMomSZ0yPHyA04+0gv7H15stTsFUM8RZeBgNk/6HiA/Fqy
n73bf6ZnryAze89ZAFQw2uD3Kn0g3slizfKVyNuGDY9LEfqrzDvkVYG+ajYXvObh
4sa7t1n8IMs1VFZnYhintiYgrazRQVvwtp9kGJQMd+av7fuSrMi98On1AoGBALKx
Z0wJEiTwiM/c1p8aFKlDIYo0vGcK8962N4Vb23LEpqkqwvDPucx/CKW6gFBe6Nsy
Hc6a4TFZrj3tFfTV7msPS8Wt92khGnntnUMqg7y1MwaOLPICCss1PvZ9L8sy2ci6
K4w2P+e+B3JqNzHITPk17lrdbbdjD2ZTNQl0+iBhAoGAeitNc38UpvYWgmUZ1EJu
cpKtg2aQCvCLImnd7LyTu1sbbg00TFpQacSOEgeAcIWP6HfgtrnTX+OwyA5/yCHG
a89zCRmQCdo7kzdfJfDweN5ztCmgpfdLC+Q2kalcQfINyYBxOf+3UmoNTBlqSeCa
5sXXMkroiS5edT9nN7JoTW4=
-----END PRIVATE KEY-----
-----BEGIN CERTIFICATE-----
MIIDRzCCAi+gAwIBAgIUScYjHBliUxuB5JT9tECieV3ku5cwDQYJKoZIhvcNAQEL
BQAwMjELMAkGA1UEBhMCVVMxDzANBgNVBAoMBk9tb2NoaTESMBAGA1UEAwwJQkND
IERlYnVnMCAXDTI2MDUxNzIwNTAyMVoYDzIxMjYwNDIzMjA1MDIxWjAyMQswCQYD
VQQGEwJVUzEPMA0GA1UECgwGT21vY2hpMRIwEAYDVQQDDAlCQ0MgRGVidWcwggEi
MA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQCmBNx3G6wn5h639cvUxyul2ik3
/a4uBBfmGAccldsdawLzg4X7y4nYvBjNo1KWWnKekIWnDHxULtH3zwEwRAZPFmNP
wKvJ3pwYlUE/RvunAVM3PuLGnAFSmDghE3Sylc02HitS0qrWuW/ZwWPjLIUkmLD/
CQnqA1eZL5io+KOZpYx6+iD9XW9aR2ANdHX/813tnGp0HPelUhBgtFuDdjAJvuzX
hhQWFncvYmD5u2wqDVES3o7PkitCFM9xCrg35bIvpTBvyltca2cwuNmngx3sldU0
G0MqmsCpwRvgun9f3vMtlQ3KXEzN+dYP6oYTOIlpUT9pjwCod9ykCbtRXcn1AgMB
AAGjUzBRMB0GA1UdDgQWBBQ1jNEP84Ahqea+IGcTsLsrmKvIYDAfBgNVHSMEGDAW
gBQ1jNEP84Ahqea+IGcTsLsrmKvIYDAPBgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3
DQEBCwUAA4IBAQClgShVAxP5eeCgNvgOySVOFXDNhLRHKWWGOPNkVxb2j5nCMO+y
6LGsHdH1a/a9YsLyQ/08Prb6Q15cVZ3RwzwTCCnSote43i7hDhCWHrxLSTccCWl3
uosSA7VXy943j7l/goKhIkV01Vuful2/PkPCfh6u+yZ66fZe0E56TXY7Ei9znBfk
vna+hVemUkD1ezLTGjoT56Zd63zVF1YI66r37jZ1uEGpKeuFeG9ATgTce6rzWtWg
R8lCToYI1d9YTN3UwkzWp1Id0b6DLMrKznir6uiWsiOKc9s4fMILOK0ehSlZ6V6H
0JkeMoqTC9BNIOYSCKyFcUmGZ1YUhU8Mf4Si
-----END CERTIFICATE-----"#;

fn get_pem_path() -> PathBuf {
    let mut path = crate::global::io::json::get_app_data_dir();
    path.push("identity.pem");
    path
}

pub fn get_active_pem() -> (String, bool) {
    let path = get_pem_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if content.contains("-----BEGIN PRIVATE KEY-----") && content.contains("-----BEGIN CERTIFICATE-----") {
            return (content, true);
        }
    }
    (DEFAULT_PEM.to_string(), false)
}

pub fn save_pem(pem_content: &str) -> Result<()> {
    let path = get_pem_path();
    fs::write(path, pem_content).context("Failed to write identity.pem")
}

pub fn delete_pem() {
    let path = get_pem_path();
    let _ = fs::remove_file(path);
}

pub fn generate_pem() -> Result<String> {
    let mut hardware_rng = rsa::rand_core::OsRng;
    let private_key = RsaPrivateKey::new(&mut hardware_rng, 2048).context("Failed to generate RSA Key")?;

    let private_pem_string = private_key.to_pkcs8_pem(LineEnding::LF)?.to_string();

    let public_key = private_key.to_public_key();
    let public_der = public_key.to_public_key_der()?;
    let spki_object: SubjectPublicKeyInfo = rasn::der::decode(public_der.as_ref())?;

    let cert_start_tag = "-----BEGIN CERTIFICATE-----";
    let cert_end_tag = "-----END CERTIFICATE-----";
    let cert_start_index = DEFAULT_PEM.find(cert_start_tag).context("No cert start")?;
    let cert_end_index = DEFAULT_PEM.find(cert_end_tag).context("No cert end")?;

    let base64_certificate = &DEFAULT_PEM[cert_start_index + cert_start_tag.len()..cert_end_index]
        .replace('\n', "")
        .replace('\r', "");

    let raw_der_bytes = BASE64_STANDARD.decode(base64_certificate)?;
    let mut certificate_template: Certificate = rasn::der::decode(&raw_der_bytes)?;

    certificate_template.tbs_certificate.subject_public_key_info = spki_object;

    let tbs_der = rasn::der::encode(&certificate_template.tbs_certificate)?;

    let digest = Sha256::digest(&tbs_der);
    let padding = Pkcs1v15Sign::new::<Sha256>();
    let signature = private_key.sign(padding, &digest).context("Failed to sign certificate")?;

    certificate_template.signature_value = rasn::types::BitString::from_vec(signature);

    let final_certificate_der = rasn::der::encode(&certificate_template)?;
    let base64_final_certificate = BASE64_STANDARD.encode(&final_certificate_der);
    let estimated_capacity = private_pem_string.len() + base64_final_certificate.len() + 100;
    let mut final_combined_pem = String::with_capacity(estimated_capacity);

    final_combined_pem.push_str(private_pem_string.trim());
    final_combined_pem.push_str("\n-----BEGIN CERTIFICATE-----\n");

    for chunk in base64_final_certificate.as_bytes().chunks(64) {
        final_combined_pem.push_str(std::str::from_utf8(chunk).unwrap());
        final_combined_pem.push('\n');
    }
    final_combined_pem.push_str("-----END CERTIFICATE-----\n");

    Ok(final_combined_pem)
}