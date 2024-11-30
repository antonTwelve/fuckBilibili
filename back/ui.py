import time
from datetime import datetime

from textual.app import App
from textual.widgets import Label
from textual.containers import Horizontal, Vertical

from db import database_thread
from spider import bv_mid_getter


class Server_UI(App):
    CSS_PATH = "ui_style.css"

    def __init__(self, bv2mid_info: bv_mid_getter, db_info: database_thread, request_count):
        super().__init__()
        self.bv2mid_info = bv2mid_info
        self.db_info = db_info
        self.request_count = request_count
        self.start_time = time.time()

    def compose(self):
        yield Label(id="timestr")
        with Horizontal(id="timeContainer"):
            yield Label("启动时间: ")
            yield Label(id="runTime")
        with Vertical(id="debugInfoWrapper"):
            with Horizontal():
                yield Label("bv->mid查询等待队列长度: ")
                yield Label(id="bv2midQueueSize")
            with Horizontal():
                yield Label("bv->mid转换数据缓存数量: ")
                yield Label(id="bv2midCacheSize")
            with Horizontal():
                yield Label("bv->mid查询次数: ")
                yield Label(id="bv2midRequestCount")
            with Horizontal():
                yield Label("数据库互斥锁状态: ")
                yield Label(id="dblockStatus")
            with Horizontal():
                yield Label("用户查询累计次数: ")
                yield Label(id="midQueryCount")
            with Horizontal():
                yield Label("BV查询累计次数: ")
                yield Label(id="BVQueryCount")
            with Horizontal():
                yield Label("BV查询数量: ")
                yield Label(id="BVQueryBVCount")

    def on_mount(self):
        self.set_interval(0.5, self.update)
        self.update()
        self.query_one("#debugInfoWrapper").border_title = "debug"

    def update(self):
        self.query_one("#timestr", Label).update(datetime.now().strftime("%Y-%m-%d %H:%M:%S"))
        self.query_one("#runTime", Label).update(seconds_to_hms(time.time() - self.start_time))
        self.query_one("#bv2midQueueSize", Label).update(str(self.bv2mid_info.tasks.qsize()))
        self.query_one("#bv2midCacheSize", Label).update(str(len(self.bv2mid_info.cache)))
        self.query_one("#bv2midRequestCount", Label).update(str(self.bv2mid_info.request_count))
        # 查询数据库互斥锁状态
        db_locked = is_locked(self.db_info.lock)
        if db_locked:
            self.query_one("#dblockStatus", Label).styles.color = "red"
            self.query_one("#dblockStatus", Label).update("锁定")
        else:
            self.query_one("#dblockStatus", Label).styles.color = "green"
            self.query_one("#dblockStatus", Label).update("未锁定")
        self.query_one("#midQueryCount", Label).update(str(self.request_count["mid_query"]))
        self.query_one("#BVQueryCount", Label).update(str(self.request_count["bv_query"]))
        self.query_one("#BVQueryBVCount", Label).update(str(self.request_count["bv_query_count"]))


def is_locked(lock):
    """
    查询一个锁是否处于锁定状态
    :param lock: 需要查询的锁
    :return: True表示锁定
    """
    result = not lock.acquire(blocking=False)
    if not result:
        lock.release()
    return result


def seconds_to_hms(seconds):
    hours, remainder = divmod(seconds, 3600)
    minutes, seconds = divmod(remainder, 60)
    return f"{int(hours):02}:{int(minutes):02}:{int(seconds):02}"
