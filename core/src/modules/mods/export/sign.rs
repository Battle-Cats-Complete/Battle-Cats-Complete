use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use rasn_pkix::Certificate;
use rayon::prelude::*;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::{DecodePrivateKey, EncodePublicKey};
use rsa::{Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey};
use sha2::{Digest as _, Sha256};
use tracing::{debug, info, trace};

use crate::modules::settings::logic::pem::get_active_pem;

const APK_SIGNING_BLOCK_MAGIC: &[u8] = b"APK Sig Block 42";
const APK_SIGNING_BLOCK_V2_ID: u32 = 0x7109871a;
const RSA_PKCS1V15_SHA2_256: u32 = 0x0103;
const MAX_CHUNK_SIZE: usize = 1024 * 1024;

pub struct ZipInfo {
    pub central_directory_start: u64,
    pub eocd_start: u64,
}

impl ZipInfo {
    pub fn new<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        trace!("Scanning for ZIP End of Central Directory (EOCD)...");
        let mut eocd_magic = [0u8; 4];
        let file_length = reader.seek(SeekFrom::End(0))?;

        let mut search_position = file_length.saturating_sub(22);
        let mut magic_found = false;

        while search_position > 0 && search_position >= file_length.saturating_sub(0xFFFF + 22) {
            reader.seek(SeekFrom::Start(search_position))?;
            reader.read_exact(&mut eocd_magic)?;
            if eocd_magic == [0x50, 0x4b, 0x05, 0x06] {
                magic_found = true;
                break;
            }
            search_position -= 1;
        }

        anyhow::ensure!(magic_found, "End of Central Directory (EOCD) not found. Is this a valid ZIP?");
        debug!("EOCD located at offset: {}", search_position);

        reader.seek(SeekFrom::Start(search_position + 16))?;
        let central_directory_start = reader.read_u32::<LittleEndian>()? as u64;
        debug!("Central Directory start offset parsed as: {}", central_directory_start);

        Ok(ZipInfo {
            central_directory_start,
            eocd_start: search_position,
        })
    }
}

pub struct Signer {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
    certificate_der: Certificate,
}

impl Signer {
    pub fn new(pem_string: &str) -> Result<Self> {
        debug!("Parsing PEM string for Signer initialization.");
        let cert_start_tag = "-----BEGIN CERTIFICATE-----";
        let cert_end_tag = "-----END CERTIFICATE-----";

        let cert_start_index = pem_string.find(cert_start_tag).context("No BEGIN CERTIFICATE tag found in PEM")?;
        let cert_end_index = pem_string.find(cert_end_tag).context("No END CERTIFICATE tag found in PEM")?;

        let private_key_string = &pem_string[..cert_start_index].trim();

        trace!("Decoding RSA Private Key...");
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_string)
            .or_else(|_| RsaPrivateKey::from_pkcs1_pem(private_key_string))
            .context("Failed to parse RSA Private Key from PEM.")?;

        let public_key = private_key.to_public_key();

        let base64_certificate = &pem_string[cert_start_index + cert_start_tag.len()..cert_end_index]
            .replace(['\n', '\r'], "");

        trace!("Decoding Base64 Certificate and Parsing ASN.1 DER...");
        let raw_der_bytes = BASE64_STANDARD.decode(base64_certificate).context("Failed to base64 decode certificate")?;
        let certificate_der = rasn::der::decode::<Certificate>(&raw_der_bytes)
            .map_err(|error| anyhow::anyhow!("Failed to parse ASN.1 Certificate: {}", error))?;

        debug!("Signer successfully initialized with public key and certificate.");
        Ok(Self {
            private_key,
            public_key,
            certificate_der,
        })
    }

    pub fn cert(&self) -> &Certificate {
        &self.certificate_der
    }

    pub fn pubkey(&self) -> &RsaPublicKey {
        &self.public_key
    }

    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        trace!("Signing payload of length: {}", data.len());
        let digest = Sha256::digest(data);
        let padding = Pkcs1v15Sign::new::<Sha256>();
        self.private_key.sign(padding, &digest).map_err(|error| anyhow::anyhow!("RSA signing failed: {}", error))
    }
}

pub fn sign(apk_path: &Path, custom_signer: Option<Signer>) -> Result<()> {
    info!("Starting APK signature process for: {:?}", apk_path);
    let identity = custom_signer.map(Ok).unwrap_or_else(|| {
        trace!("No custom signer provided, retrieving active PEM.");
        let (active_pem, _) = get_active_pem();
        Signer::new(&active_pem)
    })?;

    debug!("Reading target APK bytes into memory...");
    let apk_bytes = std::fs::read(apk_path)?;
    let mut reader = Cursor::new(&apk_bytes);

    let block_info = parse_apk_signing_block(&mut reader)?;
    trace!("Signing Block Info: start={}, cd_start={}, eocd_start={}",
           block_info.signing_block_start, block_info.central_directory_start, block_info.eocd_start);

    let zip_hash = compute_digest_parallel(
        &apk_bytes,
        block_info.signing_block_start,
        block_info.central_directory_start,
        block_info.eocd_start
    )?;

    debug!("Computed master parallel zip hash.");

    let mut new_signature_block = vec![];
    let mut writer = Cursor::new(&mut new_signature_block);
    write_apk_signing_block(&mut writer, zip_hash, &identity)?;

    trace!("Re-assembling signed APK stream...");
    let mut output_file = File::create(apk_path)?;

    output_file.write_all(&apk_bytes[..(block_info.signing_block_start as usize)])?;
    output_file.write_all(&new_signature_block)?;
    let new_cd_start_offset = output_file.stream_position()?;

    output_file.write_all(&apk_bytes[(block_info.central_directory_start as usize)..(block_info.eocd_start as usize)])?;
    let new_eocd_offset = output_file.stream_position()?;

    output_file.write_all(&apk_bytes[(block_info.eocd_start as usize)..])?;

    debug!("Patching new EOCD Central Directory offset.");
    output_file.seek(SeekFrom::Start(new_eocd_offset + 16))?;
    output_file.write_u32::<LittleEndian>(new_cd_start_offset as u32)?;

    info!("APK signature applied successfully.");
    Ok(())
}

fn compute_digest_parallel(
    apk_bytes: &[u8],
    signing_block_start: u64,
    central_directory_start: u64,
    eocd_start: u64,
) -> Result<[u8; 32]> {
    trace!("Preparing chunks for parallel hashing...");
    let mut final_hasher = Sha256::new();

    let contents_bytes = &apk_bytes[..signing_block_start as usize];
    let cd_bytes = &apk_bytes[(central_directory_start as usize)..(eocd_start as usize)];

    let mut eocd_buffer = apk_bytes[(eocd_start as usize)..].to_vec();
    let mut eocd_cursor = Cursor::new(&mut eocd_buffer);
    eocd_cursor.seek(SeekFrom::Start(16))?;
    eocd_cursor.write_u32::<LittleEndian>(signing_block_start as u32)?;

    let mut all_chunks: Vec<&[u8]> = Vec::new();
    all_chunks.extend(contents_bytes.chunks(MAX_CHUNK_SIZE));
    all_chunks.extend(cd_bytes.chunks(MAX_CHUNK_SIZE));
    all_chunks.extend(eocd_buffer.chunks(MAX_CHUNK_SIZE));

    debug!("Dispatching {} chunks to parallel iterators for hashing.", all_chunks.len());

    let hash_chunks: Vec<[u8; 32]> = all_chunks
        .into_par_iter()
        .map(|chunk| {
            let mut chunk_hasher = Sha256::new();
            chunk_hasher.update([0xa5]);
            chunk_hasher.update((chunk.len() as u32).to_le_bytes());
            chunk_hasher.update(chunk);
            chunk_hasher.finalize().into()
        })
        .collect();

    trace!("Combining chunk hashes into final master hash...");
    final_hasher.update([0x5a]);
    final_hasher.update((hash_chunks.len() as u32).to_le_bytes());

    for chunk_hash in &hash_chunks {
        final_hasher.update(chunk_hash);
    }

    Ok(final_hasher.finalize().into())
}

#[derive(Debug, Default)]
struct Digest {
    pub algorithm: u32,
    pub digest: Vec<u8>,
}

impl Digest {
    fn new(hash: [u8; 32]) -> Self {
        Self {
            algorithm: RSA_PKCS1V15_SHA2_256,
            digest: hash.to_vec(),
        }
    }

    fn size(&self) -> u32 {
        self.digest.len() as u32 + 12
    }

    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_u32::<LittleEndian>(self.digest.len() as u32 + 8)?;
        writer.write_u32::<LittleEndian>(self.algorithm)?;
        writer.write_u32::<LittleEndian>(self.digest.len() as u32)?;
        writer.write_all(&self.digest)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct SignedData {
    pub digests: Vec<Digest>,
    pub certificates: Vec<Vec<u8>>,
    pub additional_attributes: Vec<(u32, Vec<u8>)>,
}

impl SignedData {
    fn new(hash: [u8; 32], signer: &Signer) -> Result<Self> {
        Ok(Self {
            digests: vec![Digest::new(hash)],
            certificates: vec![
                rasn::der::encode(signer.cert()).map_err(|error| anyhow::anyhow!("{}", error))?
            ],
            additional_attributes: vec![],
        })
    }

    fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_u32::<LittleEndian>(self.digests.iter().map(|digest| digest.size()).sum())?;
        for digest in &self.digests { digest.write(writer)?; }

        writer.write_u32::<LittleEndian>(self.certificates.iter().map(|cert| cert.len() as u32 + 4).sum())?;
        for certificate in &self.certificates {
            writer.write_u32::<LittleEndian>(certificate.len() as u32)?;
            writer.write_all(certificate)?;
        }

        writer.write_u32::<LittleEndian>(self.additional_attributes.iter().map(|(_, val)| val.len() as u32 + 8).sum())?;
        for (identifier, value) in &self.additional_attributes {
            writer.write_u32::<LittleEndian>(value.len() as u32 + 4)?;
            writer.write_u32::<LittleEndian>(*identifier)?;
            writer.write_all(value)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ApkSignatureBlockV2 {
    pub signers: Vec<ApkSigner>,
}

#[derive(Debug)]
struct ApkSigner {
    pub signed_data: Vec<u8>,
    pub signatures: Vec<ApkSignature>,
    pub public_key: Vec<u8>,
}

#[derive(Debug)]
struct ApkSignature {
    pub algorithm: u32,
    pub signature: Vec<u8>,
}

impl ApkSignatureBlockV2 {
    fn new(hash: [u8; 32], signer: &Signer) -> Result<Self> {
        trace!("Constructing APK Signature Block V2 from payload and signer.");
        let mut signed_data = vec![];
        SignedData::new(hash, signer)?.write(&mut signed_data)?;
        let signature = signer.sign(&signed_data)?;

        Ok(Self {
            signers: vec![ApkSigner {
                signed_data,
                signatures: vec![ApkSignature {
                    algorithm: RSA_PKCS1V15_SHA2_256,
                    signature,
                }],
                public_key: signer.pubkey().to_public_key_der()?.as_ref().to_vec(),
            }],
        })
    }

    fn write(&self, writer: &mut impl Write) -> Result<()> {
        let mut buffer = vec![];
        for signer in &self.signers {
            let mut signer_buffer = vec![];
            signer_buffer.write_u32::<LittleEndian>(signer.signed_data.len() as u32)?;
            signer_buffer.write_all(&signer.signed_data)?;

            let mut signature_buffer = vec![];
            for sig in &signer.signatures {
                signature_buffer.write_u32::<LittleEndian>(sig.signature.len() as u32 + 8)?;
                signature_buffer.write_u32::<LittleEndian>(sig.algorithm)?;
                signature_buffer.write_u32::<LittleEndian>(sig.signature.len() as u32)?;
                signature_buffer.write_all(&sig.signature)?;
            }
            signer_buffer.write_u32::<LittleEndian>(signature_buffer.len() as u32)?;
            signer_buffer.write_all(&signature_buffer)?;

            signer_buffer.write_u32::<LittleEndian>(signer.public_key.len() as u32)?;
            signer_buffer.write_all(&signer.public_key)?;

            buffer.write_u32::<LittleEndian>(signer_buffer.len() as u32)?;
            buffer.write_all(&signer_buffer)?;
        }
        writer.write_u32::<LittleEndian>(buffer.len() as u32)?;
        writer.write_all(&buffer)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct ApkSignatureBlock {
    pub signing_block_start: u64,
    pub central_directory_start: u64,
    pub eocd_start: u64,
}

fn write_apk_signing_block<W: Write + Seek>(
    writer: &mut W,
    hash: [u8; 32],
    signer: &Signer,
) -> Result<()> {
    trace!("Writing custom v2 signature block to output buffer.");
    let mut buffer = vec![];
    ApkSignatureBlockV2::new(hash, signer)?.write(&mut buffer)?;

    let block_size = buffer.len() as u64 + 36;
    writer.write_u64::<LittleEndian>(block_size)?;
    writer.write_u64::<LittleEndian>(buffer.len() as u64 + 4)?;
    writer.write_u32::<LittleEndian>(APK_SIGNING_BLOCK_V2_ID)?;
    writer.write_all(&buffer)?;
    writer.write_u64::<LittleEndian>(block_size)?;
    writer.write_all(APK_SIGNING_BLOCK_MAGIC)?;

    Ok(())
}

fn parse_apk_signing_block<R: Read + Seek>(reader: &mut R) -> Result<ApkSignatureBlock> {
    trace!("Scanning for existing APK Signature Block...");
    let zip_info = ZipInfo::new(reader)?;
    let mut block = ApkSignatureBlock {
        eocd_start: zip_info.eocd_start,
        central_directory_start: zip_info.central_directory_start,
        ..Default::default()
    };

    reader.seek(SeekFrom::Start(block.central_directory_start - 16 - 8))?;
    let remaining_size = reader.read_u64::<LittleEndian>()?;
    let mut magic_buffer = [0; 16];
    reader.read_exact(&mut magic_buffer)?;

    if magic_buffer != APK_SIGNING_BLOCK_MAGIC {
        debug!("No pre-existing APK Signature Block found. Injecting brand new block.");
        block.signing_block_start = block.central_directory_start;
        return Ok(block);
    }

    debug!("Pre-existing APK Signature Block discovered. Readying to strip.");
    let current_position = reader.seek(SeekFrom::Current(-(remaining_size as i64)))?;
    block.signing_block_start = current_position - 8;

    Ok(block)
}