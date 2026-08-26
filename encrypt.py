from Crypto.Cipher import AES
import base64


class Encrypt:
    def __init__(self, key: str) -> None:
        self.key = key.encode('utf-8')

    @staticmethod
    def pkcs7_padding(text: str) -> str:
        """明文使用 PKCS7 填充。"""
        bs = 16
        padding = bs - len(text.encode('utf-8')) % bs
        return text + chr(padding) * padding

    def aes_encrypt(self, content: str) -> str:
        """AES-ECB 加密，返回 base64 字符串。"""
        cipher = AES.new(self.key, AES.MODE_ECB)
        content_padding = self.pkcs7_padding(content)
        encrypt_bytes = cipher.encrypt(content_padding.encode('utf-8'))
        return str(base64.b64encode(encrypt_bytes), encoding='utf-8')


def aes_encrypt(text: str) -> str:
    """用内置密钥对明文做 AES 加密（登录密码用）。"""
    key = 'MWMqg2tPcDkxcm11'  # 密钥
    a = Encrypt(key=key)
    return a.aes_encrypt(text)
