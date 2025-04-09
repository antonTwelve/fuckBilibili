import os
import pickle
import queue
import threading
import time
from datetime import datetime

from spider.asyncQuery import async_query

CACHE_CLEAR_SIZE = 10000  # cache大于该值时触发清理


class bv_mid_getter(threading.Thread):
    def __init__(self):
        super().__init__(daemon=True)
        self.cache = {}  # 不加锁, 理论上不安全, 实际上问题不大
        self.tasks = queue.Queue()
        self.task_queue_size_disp = 0  # 用于显示的队列数量
        self.task_set = set()  # 记录所有接受过的bv号
        self.task_set_lock = threading.Lock()
        self.semaphore = threading.Semaphore(0)
        self.request_count = 0  # BV转换mid查询请求发送次数统计
        self.bv2mid_query_fail_count = 0  # BV转换mid查询失败次数
        self.cache_updated = False  # 是否有新cache加入, 用于判定是否需要保存cache
        self.last_cache_write_time = 0  # 上次cache保存的时间
        self.min_cache_write_interval = 60  # 两次cache保存的最小时间间隔(s)
        self.cache_file_name = "bvcache"

        self.cache_del_time = 60 * 60 * 24 * 7  # cache过期时间(s)
        self.__load_cache()

    def __save_cache(self):
        """
        保存缓存
        """
        # 没有数据需要更新或者距离上次写入时间过近
        if (not self.cache_updated) or (time.time() - self.last_cache_write_time < self.min_cache_write_interval):
            return
        with open(self.cache_file_name, "wb") as file:
            pickle.dump(self.cache, file)
        self.cache_updated = False

    def __load_cache(self):
        if not os.path.exists(self.cache_file_name):
            return
        with open(self.cache_file_name, "rb") as file:
            self.cache = pickle.load(file)
        self.last_cache_write_time = time.time()
        # 先清一下过期缓存
        self.__del_cache_by_time()
        self.cache_updated = True

    def __del_cache_by_time(self):
        # 删除过期的数据
        current_time = time.time()
        del_list = []
        for bv in self.cache:
            if current_time - self.cache[bv]["time"] > self.cache_del_time:
                del_list.append(bv)
        for bv in del_list:
            del self.cache[bv]

    def __cache_clear(self):
        """
        清除过期cache, 使用时注意申请lock
        """
        # 先判断当前cache数量是否过多
        if len(self.cache) < CACHE_CLEAR_SIZE:
            return
        self.__del_cache_by_time()

    def run(self):
        while True:
            self.semaphore.acquire()
            query_bv_list = []
            # 单次请求数量不做限制, 实测(2025.4.4)在获取热门页500个视频后不会触发B站拦截
            while self.tasks.qsize() != 0:  # and len(query_bv_list) < 300:
                bv = self.tasks.get()
                if bv not in self.cache and bv != '':  # 代码bug, 存在bv为空的情况, 正在排查
                    # 检查是否是查询过的bv, 不是则加入查询队列
                    query_bv_list.append(bv)
            # queue在查询时被清零, 在查询前后记录数量用于显示
            self.task_queue_size_disp = len(query_bv_list) + self.tasks.qsize()  # 更新显示数量
            if len(query_bv_list) == 0:
                # 没有需要查询的数据, 本次循环只是消耗信号量
                continue
            # 发起请求
            query_result = async_query(query_bv_list)
            # 处理返回数据
            # 检查是否需要清理以及保存缓存
            self.__cache_clear()
            self.__save_cache()
            failed_bv_exist = False
            for result in query_result:
                self.request_count += 1
                if result["mid"] is not None:
                    # 请求成功
                    self.cache[result["bv"]] = {
                        "mid": result["mid"],
                        "time": time.time()
                    }
                    self.cache_updated = True
                else:
                    with open("debug.log", "a") as log_file:
                        log_file.write(f"{datetime.now().strftime('%Y-%m-%d %H:%M:%S')} query failed: {str(result)}\n")
                    # 请求失败
                    self.tasks.put(result["bv"])  # 失败, 放回重来
                    failed_bv_exist = True
                    self.bv2mid_query_fail_count += 1
            self.task_queue_size_disp = self.tasks.qsize()  # 再次更新显示数量
            if failed_bv_exist:
                # 本次查询中存在失败的请求, 激活查询
                self.semaphore.release()  # 信号量加一
                time.sleep(1)  # 请求失败表示可能速度过快, 被拦截了...休眠一下
                # FIXME: B站部分视频由于版权在部分地区能看, 部分地区不能(BV1XNfTYfEky)
                # 热门中出现此类视频且启用代理时可能造成无限循环

    def get_mid_by_bv(self, bv: str):
        """
        根据BV号获取用户ID
        注意外部只能调用该方法, 其他方法未适配多线程, 可能报错
        """
        if bv in self.cache:
            # 已经有查询结果, 返回
            return self.cache[bv]["mid"]
        with self.task_set_lock:
            if bv in self.task_set:
                # 没有查询结果, 但有查询记录, 表示正在等待查询
                return None
            if len(self.task_set) > 100000:
                self.task_set = set()
            self.task_set.add(bv)
        self.tasks.put(bv)
        self.semaphore.release()
        return None
