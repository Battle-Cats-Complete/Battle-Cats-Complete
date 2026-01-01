use aes::Aes128;
use aes::cipher::{BlockDecryptMut, KeyIvInit, KeyInit};
use block_padding::Pkcs7;
use md5;

type Aes128Cbc = cbc::Decryptor<Aes128>;
type Aes128Ecb = ecb::Decryptor<Aes128>;

pub fn get_md5_key(text: &str) -> [u8; 16] {
    let digest = md5::compute(text.as_bytes());
    let mut key = [0u8; 16];
    let hex_string = hex::encode(&digest.0);
    key.copy_from_slice(&hex_string.as_bytes()[0..16]);
    key
}

fn decrypt_cbc_with_key(data: &[u8], key: &[u8; 16], iv: &[u8; 16]) -> Result<Vec<u8>, String> {
    let decryptor = Aes128Cbc::new(key.into(), iv.into());
    let mut buffer = data.to_vec();
    let len = decryptor
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|_| "Padding Error".to_string())?
        .len();
    buffer.truncate(len);
    Ok(buffer)
}

pub fn decrypt_ecb_with_key(data: &[u8], key: &[u8; 16]) -> Result<Vec<u8>, String> {
    let decryptor = Aes128Ecb::new(key.into());
    let mut buffer = data.to_vec();
    let len = decryptor
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|_| "Padding Error".to_string())?
        .len();
    buffer.truncate(len);
    Ok(buffer)
}

pub fn decrypt_pack_chunk(data: &[u8], _pack_filename: &str) -> Result<(Vec<u8>, String), String> {
    let keys = [
        ("", "", "JP"),
        ("", "", "EN"),
        ("", "", "TW"),
        ("", "", "KR"),
    ];

    for (k_hex, iv_hex, region) in keys.iter() {
        let key_bytes = hex::decode(k_hex).unwrap();
        let iv_bytes = hex::decode(iv_hex).unwrap();
        let key_arr: [u8; 16] = key_bytes.try_into().unwrap();
        let iv_arr: [u8; 16] = iv_bytes.try_into().unwrap();

        if let Ok(result) = decrypt_cbc_with_key(data, &key_arr, &iv_arr) {
            return Ok((result, region.to_string()));
        }
    }

    let server_key = get_md5_key("battlecats");
    if let Ok(result) = decrypt_ecb_with_key(data, &server_key) {
        return Ok((result, "Server".to_string()));
    }

    Ok((data.to_vec(), "None".to_string()))
}