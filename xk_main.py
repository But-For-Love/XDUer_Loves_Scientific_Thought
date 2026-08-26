# -*- coding: utf-8 -*-
import json
import logging
import sys
from typing import Optional
from func import login, show_msg, get_class, add, delete

try:
    sys.stdout.reconfigure(encoding='utf-8')
    sys.stderr.reconfigure(encoding='utf-8')
except Exception:
    pass

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger("xdxxk")


def _build_tasks(conf: dict) -> Optional[list]:
    """把配置归一化为任务列表 [(cat, action, kch, kxh), ...]；配置错误时返回 None。"""
    raw = conf.get("tasks")
    if raw:
        tasks = []
        for t in raw:
            ttype = t.get("type")
            if ttype in ("bx", 0):
                cat = 0
            elif ttype in ("xx", 1):
                cat = 1
            else:
                cat = None
            action = t.get("action", "add")
            kch = t.get("KCH")
            kxh = t.get("KXH", "")
            if cat is None or action not in ("add", "del", "query") or not kch or (cat == 0 and not kxh):
                logger.error("配置错误：tasks 项不合法 -> %s", t)
                return None
            tasks.append((cat, action, kch, kxh))
        return tasks

    # 兼容旧的单类别配置
    try:
        cat = int(conf.get("bx_or_xx", 0))
    except (TypeError, ValueError):
        logger.error("配置错误：bx_or_xx 必须是数字")
        return None
    if cat not in (0, 1):
        logger.error("配置错误：bx_or_xx 应为 0（必修）或 1（选修）")
        return None
    action = conf.get("action", "add")
    if action not in ("add", "del", "query"):
        logger.error("配置错误：action 仅支持 add / del / query")
        return None
    courses = conf.get("bx", []) if cat == 0 else conf.get("xx", [])
    if not courses:
        logger.error("配置错误：课程列表为空，请在 conf.json 中填写 bx / xx")
        return None
    tasks = []
    for kch in courses:
        if cat == 0 and (not kch.get("KCH") or not kch.get("KXH")):
            logger.error("配置错误：必修课必填课程号 KCH 和课序号 KXH -> %s", kch)
            return None
        tasks.append((cat, action, kch.get("KCH"), kch.get("KXH", "")))
    return tasks


def _execute(action: str, cat: int, login_resp: dict, class_dict: dict, cookies: dict, batch: str, always: int) -> None:
    """按 action 执行单个任务（query / del / add）。"""
    if action == "query":
        if cat == 0:
            logger.info(
                "匹配到必修课：%s %s 课序号 %s",
                class_dict.get("KCH", ""), class_dict.get("KCM", ""), class_dict.get("KXH", ""),
            )
        else:
            logger.info("匹配到选修课：%s %s", class_dict.get("KCH", ""), class_dict.get("KCM", ""))
    elif action == "del":
        delete(login_resp, class_dict, cookie=cookies, batch=batch, category=cat, always=always)
    else:
        add(login_resp, class_dict, cookie=cookies, batch=batch, category=cat, always=always)


def main() -> None:
    with open("conf.json", 'r', encoding="utf-8") as f:
        conf = json.load(f)  # 加载配置

    # always：连续选/退课开关（全局）
    try:
        always = int(conf.get("always", 1))
    except (TypeError, ValueError):
        logger.error("配置错误：always 必须是数字")
        return
    if always not in (0, 1):
        logger.error("配置错误：always 应为 1（连续）或 0（仅一次）")
        return

    tasks = _build_tasks(conf)
    if tasks is None:
        return

    login_resp, cookies = login(conf)

    # 显示个人信息并获取选课批次
    batch = show_msg(login_resp, conf.get("batch", ""))
    if not batch:
        logger.error("未找到可用的选课批次，请检查 conf.json 中的 batch 配置")
        return

    # 按类别拉取课程列表（每类只拉一次）
    rows_by_cat = {}
    for cat in sorted({t[0] for t in tasks}):
        lst = get_class(login_resp, conf, batch=batch, category=cat)
        if "data" not in lst or "rows" not in lst.get("data", {}):
            logger.error("获取课程列表失败：%s", lst.get("msg", "未知错误"))
            return
        rows_by_cat[cat] = lst["data"]["rows"]

    # 逐任务执行
    for cat, action, kch, kxh in tasks:
        matched = False
        for row in rows_by_cat.get(cat, []):
            if row["KCH"] != kch:
                continue
            if cat == 0:
                for tc in row["tcList"]:
                    if tc["KXH"] == kxh:
                        _execute(action, cat, login_resp, tc, cookies, batch, always)
                        matched = True
                        break
            else:
                _execute(action, cat, login_resp, row, cookies, batch, always)
                matched = True
                break
        if not matched:
            logger.warning("未找到课程：%s%s", kch, (" 课序号 " + kxh) if cat == 0 else "")


if __name__ == '__main__':
    print('{:-^30}'.format(""))
    print('{: ^30}'.format("Welcome"))
    print('{:-^30}'.format(""))
    main()
    print("Done")
