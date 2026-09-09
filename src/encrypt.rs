use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
use aes::Aes128;
use base64::{engine::general_purpose::STANDARD, Engine};

/// 与 Python 版 encrypt.AES_encrypt 保持一致：
/// 密钥 MWMqg2tPcDkxcm11，AES-128-ECB，PKCS7 填充，base64 输出。
const KEY: &[u8; 16] = b"MWMqg2tPcDkxcm11";

pub fn aes_encrypt(plaintext: &str) -> String {
    let cipher = Aes128::new_from_slice(KEY).expect("AES key must be 16 bytes");

    let mut data = plaintext.as_bytes().to_vec();
    let padding = 16 - (data.len() % 16);
    data.extend(std::iter::repeat_n(padding as u8, padding));

    let mut encrypted = Vec::with_capacity(data.len());
    for chunk in data.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        encrypted.extend_from_slice(&block);
    }
    STANDARD.encode(encrypted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        let cases = [
            ("zyx/020305", "5ZTBUxmD+OY7LL1nzUUz+g=="),
            ("123456", "OSfRhnd673K1Lp6cP4L6nA=="),
            ("", "mkpyTarWC0ro2N4QUBrjAQ=="),
            ("a", "hj0T3YA9PCHq7PAsPke1mQ=="),
            (
                "password12345678",
                "bLI9j6Y3UbBljQs6YgYfnJpKck2q1gtK6NjeEFAa4wE=",
            ),
            ("中文密码abc", "0sLEDhp4IRDqt9Y0z1Flvw=="),
        ];
        for (plain, expected) in cases {
            assert_eq!(aes_encrypt(plain), expected, "plaintext: {plain:?}");
        }
    }
}
