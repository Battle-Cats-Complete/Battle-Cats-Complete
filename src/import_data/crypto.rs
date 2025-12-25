use aes::Aes128;
use aes::cipher::{BlockDecryptMut, KeyIvInit, KeyInit};
use block_padding::Pkcs7;
use md5;

// Type aliases
type Aes128Cbc = cbc::Decryptor<Aes128>;
type Aes128Ecb = ecb::Decryptor<Aes128>;

pub fn get_md5_key(text: &str) -> [u8; 16] {
    let digest = md5::compute(text.as_bytes());
    let mut key = [0u8; 16];
    let hex_string = hex::encode(&digest.0);
    // Use the first 16 bytes of the HEX string
    key.copy_from_slice(&hex_string.as_bytes()[0..16]);
    key
}

// Helper to decrypt standard padded data
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

// FIXED: Added 'pub' here so game_data.rs can see it
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

// The Main Function
pub fn decrypt_pack_chunk(data: &[u8], pack_filename: &str) -> Result<Vec<u8>, String> {
    let lower_name = pack_filename.to_lowercase();

    if lower_name.contains("imagedatalocal") {
         return Ok(data.to_vec());
    }

    // CASE A: Server Packs
    if lower_name.contains("server") {
        let key = get_md5_key("battlecats");
        return decrypt_ecb_with_key(data, &key);
    }

    // CASE B: Normal Packs (Global then JP)
    let global_key = hex::decode("").unwrap();
    let global_iv = hex::decode("").unwrap();
    let g_key_arr: [u8; 16] = global_key.try_into().unwrap();
    let g_iv_arr: [u8; 16] = global_iv.try_into().unwrap();

    if let Ok(result) = decrypt_cbc_with_key(data, &g_key_arr, &g_iv_arr) {
        return Ok(result);
    }

    let jp_key = hex::decode("").unwrap();
    let jp_iv = hex::decode("").unwrap();
    let j_key_arr: [u8; 16] = jp_key.try_into().unwrap();
    let j_iv_arr: [u8; 16] = jp_iv.try_into().unwrap();

    if let Ok(result) = decrypt_cbc_with_key(data, &j_key_arr, &j_iv_arr) {
        return Ok(result);
    }

    Err("Failed to decrypt: Key rejected or data corrupt".to_string())
}