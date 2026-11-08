//! regulation.bin 外层加密：AES-256-CBC，文件头 16 字节为 IV。用 Windows 自带的 BCrypt，不额外引入加密库。
//! 解密不去填充（内层 DCX 头里记录了真实长度）；加密用随机 IV + PKCS7 填充，与 SoulsFormats 写出的格式相同。

use windows_sys::Win32::Security::Cryptography::{
    BCRYPT_AES_ALGORITHM, BCRYPT_ALG_HANDLE, BCRYPT_BLOCK_PADDING, BCRYPT_CHAIN_MODE_CBC, BCRYPT_CHAINING_MODE,
    BCRYPT_KEY_HANDLE, BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptCloseAlgorithmProvider, BCryptDecrypt, BCryptDestroyKey,
    BCryptEncrypt, BCryptGenRandom, BCryptGenerateSymmetricKey, BCryptOpenAlgorithmProvider, BCryptSetProperty,
};

/// 黑夜君临 regulation 密钥（与 SoulsFormats 的 NR_REGULATION_KEY 相同）
const KEY: [u8; 32] = [
    0x9a, 0x8e, 0xe9, 0x0c, 0x4c, 0x01, 0xa4, 0x31, 0x68, 0xa1, 0x7d, 0x9d, 0x75, 0xe4, 0xa7, 0xd0, 0x21, 0x07, 0xeb, 0xcf,
    0x43, 0xd5, 0xac, 0xb0, 0x55, 0x4f, 0x94, 0x16, 0x01, 0xb5, 0x79, 0x18,
];
const IV_LEN: usize = 16;

struct Aes {
    alg: BCRYPT_ALG_HANDLE,
    key: BCRYPT_KEY_HANDLE,
}

impl Aes {
    fn new() -> Result<Self, String> {
        let mut aes = Self { alg: std::ptr::null_mut(), key: std::ptr::null_mut() };
        unsafe {
            check(BCryptOpenAlgorithmProvider(&mut aes.alg, BCRYPT_AES_ALGORITHM, std::ptr::null(), 0), "open AES")?;
            let mode = wide_bytes(BCRYPT_CHAIN_MODE_CBC);
            check(BCryptSetProperty(aes.alg, BCRYPT_CHAINING_MODE, mode.as_ptr(), mode.len() as u32, 0), "set CBC")?;
            check(
                BCryptGenerateSymmetricKey(aes.alg, &mut aes.key, std::ptr::null_mut(), 0, KEY.as_ptr(), KEY.len() as u32, 0),
                "import key",
            )?;
        }
        Ok(aes)
    }
}

impl Drop for Aes {
    fn drop(&mut self) {
        unsafe {
            if !self.key.is_null() {
                BCryptDestroyKey(self.key);
            }
            if !self.alg.is_null() {
                BCryptCloseAlgorithmProvider(self.alg, 0);
            }
        }
    }
}

pub fn decrypt(file: &[u8]) -> Result<Vec<u8>, String> {
    if file.len() <= IV_LEN || (file.len() - IV_LEN) % 16 != 0 {
        return Err("not an encrypted regulation (bad length)".into());
    }
    let aes = Aes::new()?;
    let mut iv: [u8; IV_LEN] = file[..IV_LEN].try_into().unwrap();
    let input = &file[IV_LEN..];
    let mut out = vec![0u8; input.len()];
    let mut written = 0u32;
    unsafe {
        check(
            BCryptDecrypt(aes.key, input.as_ptr(), input.len() as u32, std::ptr::null(), iv.as_mut_ptr(), IV_LEN as u32,
                out.as_mut_ptr(), out.len() as u32, &mut written, 0),
            "decrypt",
        )?;
    }
    out.truncate(written as usize);
    Ok(out)
}

pub fn encrypt(plain: &[u8]) -> Result<Vec<u8>, String> {
    let aes = Aes::new()?;
    let mut iv = [0u8; IV_LEN];
    unsafe { check(BCryptGenRandom(std::ptr::null_mut(), iv.as_mut_ptr(), IV_LEN as u32, BCRYPT_USE_SYSTEM_PREFERRED_RNG), "random IV")? };
    let mut out = vec![0u8; IV_LEN + plain.len() + 16];
    out[..IV_LEN].copy_from_slice(&iv);
    let mut written = 0u32;
    // BCrypt 会改写传入的 IV，用副本
    let mut chain = iv;
    unsafe {
        check(
            BCryptEncrypt(aes.key, plain.as_ptr(), plain.len() as u32, std::ptr::null(), chain.as_mut_ptr(), IV_LEN as u32,
                out[IV_LEN..].as_mut_ptr(), (out.len() - IV_LEN) as u32, &mut written, BCRYPT_BLOCK_PADDING),
            "encrypt",
        )?;
    }
    out.truncate(IV_LEN + written as usize);
    Ok(out)
}

fn check(status: i32, what: &str) -> Result<(), String> {
    if status >= 0 { Ok(()) } else { Err(format!("BCrypt {what} failed: {status:#x}")) }
}

/// 以 0 结尾的宽字符串常量 -> 含结尾 0 的字节
fn wide_bytes(s: *const u16) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    loop {
        let c = unsafe { *s.add(i) };
        out.extend_from_slice(&c.to_le_bytes());
        if c == 0 {
            return out;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_then_decrypt_round_trips() {
        let plain: Vec<u8> = (0..100u8).collect();
        let enc = encrypt(&plain).unwrap();
        assert_eq!(enc.len(), 16 + 112);
        let dec = decrypt(&enc).unwrap();
        assert_eq!(&dec[..100], &plain[..]);
        assert!(dec[100..].iter().all(|&b| b == 12));
    }
}
