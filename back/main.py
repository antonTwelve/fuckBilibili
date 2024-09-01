import logging

from db import database_thread
from spider import bv_mid_getter
from flask import Flask, request, jsonify

user_database = database_thread()
user_database.start()
bv2mid = bv_mid_getter()
bv2mid.start()

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


@app.route("/remove", methods=["POST"])
def remove_user():
    mid = request.form.get("mid")
    if (mid is None) or (not mid.isdigit()):
        return "ERR1"
    if not user_database.delete(mid):
        return "ERR2"
    return "OK"


@app.route("/blockBV", methods=["GET"])
def is_block_bv():
    bv = request.args.get("bv")
    if bv is None:
        return "ERR bv"
    mid = bv2mid.get_mid_by_bv(bv)
    if mid is None:
        return jsonify({
            "msg": "just wait..."
        })
    ret_data = {
        "msg": "OK",
        "mid": mid,
        "result": is_mid_exist(mid)
    }
    return jsonify(ret_data)


app.run(port=22332)
user_database.close()
