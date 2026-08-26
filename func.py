import logging
import requests
import json
import base64
import ddddocr
import time
from typing import Any, Optional
from encrypt import aes_encrypt


logger = logging.getLogger("xdxxk")


_ocr = None


def ocr_captcha(img: bytes) -> str:
    """
    :return: 识别到的验证码
    """
    global _ocr
    if _ocr is None:
        _ocr = ddddocr.DdddOcr()
    return _ocr.classification(img)


def get_captcha(conf: dict) -> tuple:
    """
    :param conf: conf.json
    :return: code & uuid
    """
    url = "https://xk.xidian.edu.cn/xsxk/auth/captcha"
    result = requests.post(url)
    p = result.json()
    if conf['debug'] == '1':
        with open("captcha_pac.json", "wb") as f:
            f.write(result.content)      # 字节形式写入，保存为json文件

    logger.info("验证码接口状态：%s", p['msg'])

    pic = p['data']['captcha'].replace("data:image/png;base64,", "")
    b = base64.b64decode(pic)       # 用于ddddocr识别

    if conf["ocr_captcha"] == "1":      # 默认，自动识别验证码
        code = ocr_captcha(b)
        logger.info("验证码为: %s", code)
    else:                               # 手动输入
        with open("captcha.png", "wb") as f:
            f.write(b)
        logger.info("验证码图片已保存为 captcha.png，请打开查看")
        code = input("请输入验证码:")

    return code, p['data']['uuid']


def login(conf: dict) -> tuple:
    """
    :param conf: conf.json
    :return: 登录后返回的json
    """
    url = "https://xk.xidian.edu.cn/xsxk/auth/login"

    header = {
        "Connection": "keep-alive",
        "User-Agent": ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                       "(KHTML, like Gecko) Chrome/103.0.5060.66 Safari/537.36 Edg/103.0.1264.44"),
    }

    form = dict(conf["data"])
    if conf["data"]["loginname"] == "" or conf["data"]["password"] == "":   # 如果缺少 用户名 和 密码
        form["loginname"] = input("学号：")
        form["password"] = input("密码：")
    form["password"] = aes_encrypt(form["password"])
    form["captcha"], form["uuid"] = get_captcha(conf)   # 构造表单

    result = requests.post(url, headers=header, params=form)
    data = result.json()
    if conf['debug'] == '1':
        try:
            debug_data = json.loads(result.text)  # 独立解析，避免影响返回值
            if isinstance(debug_data, dict) and isinstance(debug_data.get("data"), dict):
                debug_data["data"].pop("token", None)
            with open("login_pac.json", "w", encoding="utf-8") as f:
                json.dump(debug_data, f, ensure_ascii=False, indent=2)
        except Exception:
            with open("login_pac.json", "wb") as f:
                f.write(result.content)

    return data, requests.utils.dict_from_cookiejar(result.cookies)
    # return result.json()


def show_msg(resp: dict, batch: Optional[str] = None) -> str:
    """
    :param resp:  登录成功后返回的json
    :param batch: 期望的选课批次名称（子串匹配）；为空时自动选择第一个可选批次
    :return: 选课批次 code（未找到时返回空字符串）
    """
    batch_code = ''
    try:
        logger.info("姓名：%s", resp["data"]["student"]["XM"])
        logger.info("专业：%s", resp["data"]["student"]["ZYMC"])
        logger.info("班级：%s", resp["data"]["student"]["schoolClass"])
        lst = resp["data"]["student"]["electiveBatchList"]
        first_available = ''
        matched = False
        for i in lst:
            logger.info("选课批次：%s 是否可选：%s", i["name"], i["canSelect"])
            if str(i["canSelect"]) == "1":
                if not first_available:
                    first_available = i["code"]
                if batch and batch in i["name"]:
                    matched = True
                    if not batch_code:
                        batch_code = i["code"]
        if batch and not matched:
            logger.warning("未找到名称包含“%s”的可选批次，将使用第一个可选批次", batch)
        if not batch_code:
            batch_code = first_available
    except (TypeError, KeyError):
        logger.error("登录响应异常：%s", resp.get("msg") if isinstance(resp, dict) else resp)
    return batch_code


def get_class(login_resp: dict, conf: dict, batch: str, category: int = 0) -> dict:
    """拉取指定类别的课程列表。"""
    url = "https://xk.xidian.edu.cn/xsxk/elective/clazz/list"
    header = {
        "Connection": "keep-alive",
        "Content-Type": "application/json;charset=UTF-8",
        "batchId": batch,
        "Authorization": login_resp["data"]["token"],
        "User-Agent": ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                       "(KHTML, like Gecko) Chrome/103.0.5060.66 Safari/537.36 Edg/103.0.1264.44"),
    }
    cat = ["FANKC", "XGKC"]
    form: dict[str, Any] = {
            "teachingClassType": cat[category],
            "pageNumber": 1,
            "pageSize": 300,
            "orderBy": "",
            "campus": "S"
    }
    a = requests.post(url, json=form, headers=header)

    if conf['debug'] == '1':
        with open("classlist.json", "wb") as f:
            f.write(a.content)  # 字节形式写入，保存为json文件
    # print(a.text)
    return a.json()


def add(login_resp: dict, class_dict: dict, cookie: dict, batch: str, always: int = 1, category: int = 0) -> None:
    """选课。"""

    header = {
        "User-Agent": ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                       "(KHTML, like Gecko) Chrome/103.0.5060.66 Safari/537.36 Edg/103.0.1264.44"),
        "batchId": batch,
        "Authorization": login_resp["data"]["token"]
    }

    url = 'https://xk.xidian.edu.cn/xsxk/elective/clazz/add'

    if category == 0:        # 必修
        form = {
            "clazzType": "FANKC",
            "clazzId": class_dict["JXBID"],
            "secretVal": class_dict["secretVal"],
            "chooseVolunteer": "1"
        }
    elif category == 1:      # 选修
        form = {
            "clazzType": "XGKC",
            "clazzId": class_dict["JXBID"],
            "secretVal": class_dict["secretVal"],
            "chooseVolunteer": "1"
        }

    cookie["Authorization"] = login_resp["data"]["token"]
    if always == 1:
        msg = ''
        while msg not in ['该课程已在选课结果中', '所选课程与已选课程冲突']:
            r = requests.post(url, params=form, headers=header, cookies=cookie)
            msg = r.json()["msg"]
            logger.info("%s %s 选课 %s", class_dict["KCH"], class_dict["KCM"], msg)
            time.sleep(1)
    else:
        r = requests.post(url, params=form, headers=header, cookies=cookie)
        msg = r.json()["msg"]
        logger.info("%s %s 选课 %s", class_dict["KCH"], class_dict["KCM"], msg)


def delete(login_resp: dict, class_dict: dict, cookie: dict, batch: str, always: int = 1, category: int = 0) -> None:
    """退课。"""

    header = {
        "User-Agent": ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                       "(KHTML, like Gecko) Chrome/103.0.5060.66 Safari/537.36 Edg/103.0.1264.44"),
        "batchId": batch,
        "Authorization": login_resp["data"]["token"]
    }

    url = 'https://xk.xidian.edu.cn/xsxk/elective/clazz/del'

    if category == 0:        # 必修
        form = {
            "clazzType": "TJKC",
            "clazzId": class_dict["JXBID"],
            "secretVal": class_dict["secretVal"]
        }
    elif category == 1:      # 选修
        form = {
            "clazzType": "XGKC",
            "clazzId": class_dict["JXBID"],
            "secretVal": class_dict["secretVal"],
            "chooseVolunteer": "1"
        }

    cookie["Authorization"] = login_resp["data"]["token"]

    if always == 1:
        msg = ''
        while msg not in ['所选课程与已选课程冲突', '操作成功']:
            r = requests.post(url, params=form, headers=header, cookies=cookie)
            msg = r.json()["msg"]
            logger.info("%s %s %s 退课 %s", class_dict["KCH"], class_dict["KCM"], class_dict["SKJS"], msg)
    else:
        r = requests.post(url, params=form, headers=header, cookies=cookie)
        msg = r.json()["msg"]
        logger.info("%s %s %s 退课 %s", class_dict["KCH"], class_dict["KCM"], class_dict["SKJS"], msg)



