import logging
import threading
import time
import requests

from db import database_thread
from spider import bv_mid_getter
from flask import Flask, request, jsonify

from ui import Server_UI

PORT = 22332

user_database = database_thread()
user_database.start()
bv2mid = bv_mid_getter()
bv2mid.start()
request_count = {
    "mid_query": 0,
    "bv_query": 0,
    "bv_query_count": 0
}

app = Flask(__name__)
# 关闭日志
log = logging.getLogger('werkzeug')
log.disabled = True


@app.route("/block", methods=["POST"])
def add_user():
    mid = request.form.get("mid")
    username = request.form.get("username")
    if (mid is None) or (not mid.isdigit()):
        return "ERR1"
    if not user_database.insert(mid, username):
        return "ERR2"
    return "OK"


def is_mid_exist(mid):
    """
    判断一个mid是否已经被屏蔽
    :param mid: 数字字符串
    :return: 返回字符串"True", "False", 或者"None"
    """
    request_count["mid_query"] += 1
    if (mid is None) or (not mid.isdigit()):
        return "ERR1"
    ret = user_database.is_exist(mid)
    if ret is None:
        return "ERR2"
    return str(ret)


@app.route("/isExist", methods=["GET"])
def is_user_exist():
    mid = request.args.get("mid")
    return is_mid_exist(mid)


@app.route("/isExistS", methods=["POST"])
def is_user_exist_S():
    mids = request.form.get("mids").split(",")
    result = []
    for mid in mids:
        result.append(is_mid_exist(mid))
    return jsonify(result)


@app.route("/remove", methods=["POST"])
def remove_user():
    mid = request.form.get("mid")
    if (mid is None) or (not mid.isdigit()):
        return "ERR1"
    if not user_database.delete(mid):
        return "ERR2"
    return "OK"


# 查询一个BV列表中的每个视频是否被屏蔽
@app.route("/isBlockedBVS", methods=["POST"])
def is_blocked_bv():
    bvs = request.form.get("bvs").split(",")
    request_count["bv_query"] += 1
    request_count["bv_query_count"] += len(bvs)
    mids = []
    results = []
    for bv in bvs:
        mids.append(bv2mid.get_mid_by_bv(bv))
    for mid in mids:
        if mid is None:
            results.append("None")  # 暂时没有获取到ID
        else:
            results.append(is_mid_exist(mid))
    ret_data = {
        "msg": "OK",
        "mid": mids,
        "result": results
    }
    return jsonify(ret_data)


@app.route("/ok", methods=["GET"])
def is_alive():
    return "OK"


def flask_thread():
    app.run(port=PORT)
    user_database.close()


# 启动flask线程
threading.Thread(target=flask_thread, daemon=True).start()
# 等待flask启动完成, 直接启动TUI将导致flask无法正常启动
while True:
    # 验证http服务是否启用成功
    response = requests.get(f"http://127.0.0.1:{PORT}/ok")
    if response.status_code == 200:
        break
# 显示TUI
app = Server_UI(bv2mid, user_database, request_count)
app.run()
