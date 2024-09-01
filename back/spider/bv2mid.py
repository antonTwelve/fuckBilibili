import re
import os
import time
import pickle
import datetime
import threading
import requests
import queue


class bv_mid_getter(threading.Thread):
    def __init__(self):
        super().__init__()
        self.cache = {}
        self.tasks = queue.Queue()
        self.semaphore = threading.Semaphore(0)
        self.cache_updated = False  # 是否有新cache加入, 用于判定是否需要保存cache
        self.last_cache_write_time = 0  # 上次cache写入的时间
        self.min_cache_write_interval = 60  # 两次cache保存的最小时间间隔(s)
        self.cache_file_name = "bvcache"

        self.last_get_time = 0  # 上次请求时间, 用于限速
        self.min_get_interval = 2  # 两次请求的最小时间间隔(s)
        self.cache_del_time = 60 * 60 * 24 * 14  # cache过期时间(s)
        self.cache_clear_size = 10000  # cache大于该值时触发清理
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
        if len(self.cache) < self.cache_clear_size:
            return
        self.__del_cache_by_time()

    def __get_mid_by_bv(self, bv: str):
        self.__cache_clear()
        self.__save_cache()
        if bv in self.cache:
            return self.cache[bv]["mid"]
        # 开始查询BV号所属用户ID
        print(f"{datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')} 查询{bv}")
        result = requests.get(url=f"https://www.bilibili.com/video/{bv}/", headers={
            'User-Agent': "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0"
        })
        self.last_get_time = time.time()
        text = result.content.decode(encoding="utf-8")
        search_result = re.findall(r'"mid":([0-9]*),', text)
        self.cache[bv] = {
            "mid": search_result[0],
            "time": time.time()
        }
        self.cache_updated = True
        return search_result[0]

    def run(self):
        while True:
            self.semaphore.acquire()
            # 离上次请求太近, 直接休眠n秒
            if time.time() - self.last_get_time < self.min_get_interval:
                time.sleep(self.min_get_interval)
            BVID = self.tasks.get()
            try:
                # 防止断网报错
                self.__get_mid_by_bv(BVID)
            except:
                print(f"{datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')} 查询错误")

    def get_mid_by_bv(self, bv: str):
        """
        根据BV号获取用户ID
        注意外部只能调用该方法, 其他方法未适配多线程, 可能报错
        """
        if bv in self.cache:
            return self.cache[bv]["mid"]
        self.tasks.put(bv)
        self.semaphore.release()
        return None
