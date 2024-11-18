import sqlite3
import logging
import threading


class block_user_database:
    def __init__(self):
        self.db_logging = logging.getLogger("database_logging")
        logging.basicConfig(
            format='%(asctime)s - %(name)s - %(levelname)s in function:%(funcName)s msg: %(message)s',
            level=logging.ERROR)
        self.database = sqlite3.connect("blocked_users.db")
        self.__create_table()

    def __create_table(self):
        cursor = self.database.cursor()
        cursor.execute("""
        CREATE TABLE IF NOT EXISTS users (
                mid      INTEGER PRIMARY KEY
                                 UNIQUE
                                 NOT NULL,
                username TEXT
            );
        """)
        cursor.close()

    def insert_data(self, mid, username=None):
        cursor = self.database.cursor()
        try:
            if username is None:
                cursor.execute("INSERT INTO users (mid) VALUES(?)", (mid,))
            else:
                cursor.execute("INSERT INTO users VALUES(?, ?)", (mid, username))
        except Exception as e:
            cursor.close()
            self.db_logging.error(e)
            return False
        cursor.close()
        self.database.commit()
        return True

    def is_user_exist(self, mid):
        cursor = self.database.cursor()
        is_exist = False
        try:
            cursor.execute("SELECT 1 FROM users WHERE mid=?", (mid,))
            if not (cursor.fetchone() is None):
                is_exist = True
        except Exception as e:
            cursor.close()
            self.db_logging.error(e)
            return None
        cursor.close()
        return is_exist

    def delete(self, mid):
        cursor = self.database.cursor()
        try:
            cursor.execute(f"DELETE FROM users WHERE mid=?", (mid,))
        except Exception as e:
            cursor.close()
            self.db_logging.error(e)
            return False
        cursor.close()
        self.database.commit()
        return True

    def close(self):
        self.database.close()


class database_thread(threading.Thread):
    def __init__(self):
        super().__init__(daemon=True)
        self.lock = threading.Lock()
        self.query_se = threading.Semaphore(0)
        self.ret_se = threading.Semaphore(0)
        self.running = True
        # 任务类型, 0插入, 1查询是否存在, 2为退出唤醒信号, 3为删除用户
        self.task_type = 0
        # 传入的参数
        self.task_data = None
        # 返回值
        self.ret_value = None

    def run(self):
        db = block_user_database()
        while self.running:
            self.query_se.acquire()
            if self.task_type == 0:
                self.ret_value = db.insert_data(self.task_data["mid"], self.task_data["username"])
            elif self.task_type == 1:
                self.ret_value = db.is_user_exist(self.task_data["mid"])
            elif self.task_type == 2:
                break
            elif self.task_type == 3:
                self.ret_value = db.delete(self.task_data["mid"])
            self.ret_se.release()
        db.close()

    def insert(self, mid, username=None):
        with self.lock:
            self.task_type = 0
            self.task_data = {"mid": mid, "username": username}
            self.query_se.release()
            self.ret_se.acquire()
            # 返回值只有True,False 故直接返回, 不拷贝
            return self.ret_value

    def is_exist(self, mid):
        with self.lock:
            self.task_type = 1
            self.task_data = {"mid": mid}
            self.query_se.release()
            self.ret_se.acquire()
            # 返回值有True, False, None, 直接返回即可
            return self.ret_value

    def delete(self, mid):
        with self.lock:
            self.task_type = 3
            self.task_data = {"mid": mid}
            self.query_se.release()
            self.ret_se.acquire()
            # 返回值为True, False
            return self.ret_value

    def close(self):
        self.running = False
        # 释放退出信号, 唤醒数据库操作进程
        with self.lock:
            self.task_type = 2
            self.query_se.release()
