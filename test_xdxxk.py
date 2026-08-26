# -*- coding: utf-8 -*-
import sys
import unittest
from unittest import mock

try:
    sys.stdout.reconfigure(encoding='utf-8')
    sys.stderr.reconfigure(encoding='utf-8')
except Exception:
    pass

import encrypt
import func
import xk_main


class TestEncrypt(unittest.TestCase):
    def test_aes_encrypt_known_values(self):
        cases = {
            'zyx/020305': '5ZTBUxmD+OY7LL1nzUUz+g==',
            '123456': 'OSfRhnd673K1Lp6cP4L6nA==',
            '': 'mkpyTarWC0ro2N4QUBrjAQ==',
            'a': 'hj0T3YA9PCHq7PAsPke1mQ==',
            'password12345678': 'bLI9j6Y3UbBljQs6YgYfnJpKck2q1gtK6NjeEFAa4wE=',
            '中文密码abc': '0sLEDhp4IRDqt9Y0z1Flvw==',
        }
        for plain, expected in cases.items():
            self.assertEqual(encrypt.aes_encrypt(plain), expected, plain)

    def test_pkcs7_padding_block_aligned(self):
        enc = encrypt.Encrypt('MWMqg2tPcDkxcm11')
        for s in ['', 'a', '123456', 'password12345678', '中文密码abc', 'x' * 15, 'x' * 16, 'x' * 17]:
            padded = enc.pkcs7_padding(s)
            self.assertEqual(len(padded.encode('utf-8')) % 16, 0, repr(s))


class TestShowMsg(unittest.TestCase):
    def make_resp(self, batches):
        return {
            "data": {
                "student": {
                    "XM": "张三",
                    "ZYMC": "计算机",
                    "schoolClass": "2000910",
                    "electiveBatchList": batches,
                }
            }
        }

    def test_match_specified_batch(self):
        resp = self.make_resp([
            {"name": "2019级", "canSelect": "0", "code": "b19"},
            {"name": "2020级", "canSelect": "1", "code": "b20"},
            {"name": "2021级", "canSelect": "1", "code": "b21"},
        ])
        self.assertEqual(func.show_msg(resp, batch="2020级"), "b20")

    def test_fallback_when_batch_not_found(self):
        resp = self.make_resp([
            {"name": "2019级", "canSelect": "0", "code": "b19"},
            {"name": "2021级", "canSelect": "1", "code": "b21"},
        ])
        self.assertEqual(func.show_msg(resp, batch="2020级"), "b21")

    def test_auto_select_first_available(self):
        resp = self.make_resp([
            {"name": "2019级", "canSelect": "0", "code": "b19"},
            {"name": "2021级", "canSelect": "1", "code": "b21"},
        ])
        self.assertEqual(func.show_msg(resp, batch=""), "b21")

    def test_login_failure_returns_empty(self):
        self.assertEqual(func.show_msg({"msg": "验证码错误"}), "")


class TestBuildTasks(unittest.TestCase):
    def test_legacy_required(self):
        conf = {"bx_or_xx": 0, "action": "add", "bx": [{"KCH": "TE", "KXH": "01"}]}
        self.assertEqual(xk_main._build_tasks(conf), [(0, "add", "TE", "01")])

    def test_legacy_elective(self):
        conf = {"bx_or_xx": 1, "action": "del", "xx": [{"KCH": "FL"}]}
        self.assertEqual(xk_main._build_tasks(conf), [(1, "del", "FL", "")])

    def test_task_list(self):
        conf = {"tasks": [
            {"type": "bx", "action": "add", "KCH": "TE", "KXH": "01"},
            {"type": "xx", "action": "del", "KCH": "FL"},
        ]}
        self.assertEqual(xk_main._build_tasks(conf), [(0, "add", "TE", "01"), (1, "del", "FL", "")])

    def test_invalid_category(self):
        conf = {"bx_or_xx": 2, "bx": [{"KCH": "TE", "KXH": "01"}]}
        self.assertIsNone(xk_main._build_tasks(conf))

    def test_invalid_task_missing_kch(self):
        conf = {"tasks": [{"type": "bx", "action": "add"}]}
        self.assertIsNone(xk_main._build_tasks(conf))


class TestRequestConstruction(unittest.TestCase):
    def _login_resp(self):
        return {"data": {"token": "TOKEN123"}}

    def test_get_class_request(self):
        with mock.patch("func.requests.post") as post:
            post.return_value.json.return_value = {"data": {"rows": []}}
            func.get_class(self._login_resp(), {"debug": "0"}, batch="B1", category=1)
            args, kwargs = post.call_args
            self.assertEqual(args[0], "https://xk.xidian.edu.cn/xsxk/elective/clazz/list")
            self.assertEqual(kwargs["json"]["teachingClassType"], "XGKC")
            self.assertEqual(kwargs["json"]["pageSize"], 300)
            self.assertEqual(kwargs["headers"]["Authorization"], "TOKEN123")
            self.assertEqual(kwargs["headers"]["batchId"], "B1")

    def test_add_request(self):
        with mock.patch("func.requests.post") as post:
            post.return_value.json.return_value = {"msg": "ok"}
            cls = {"JXBID": "J1", "secretVal": "S1", "KCH": "TE", "KCM": "课"}
            func.add(self._login_resp(), cls, cookie={}, batch="B1", always=0, category=0)
            args, kwargs = post.call_args
            self.assertEqual(args[0], "https://xk.xidian.edu.cn/xsxk/elective/clazz/add")
            self.assertEqual(kwargs["params"]["clazzType"], "FANKC")
            self.assertEqual(kwargs["params"]["clazzId"], "J1")
            self.assertEqual(kwargs["params"]["secretVal"], "S1")
            self.assertEqual(kwargs["headers"]["Authorization"], "TOKEN123")

    def test_delete_request(self):
        with mock.patch("func.requests.post") as post:
            post.return_value.json.return_value = {"msg": "操作成功"}
            cls = {"JXBID": "J1", "secretVal": "S1", "KCH": "TE", "KCM": "课", "SKJS": "老师"}
            func.delete(self._login_resp(), cls, cookie={}, batch="B1", always=0, category=1)
            args, kwargs = post.call_args
            self.assertEqual(args[0], "https://xk.xidian.edu.cn/xsxk/elective/clazz/del")
            self.assertEqual(kwargs["params"]["clazzType"], "XGKC")
            self.assertEqual(kwargs["params"]["clazzId"], "J1")
            self.assertEqual(kwargs["headers"]["Authorization"], "TOKEN123")


if __name__ == '__main__':
    unittest.main()
