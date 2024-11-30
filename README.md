# fuckBilibili
一个B站视频屏蔽脚本，用于屏蔽指定用户的视频。

演示视频：https://www.bilibili.com/video/BV1tQHie8EJL/

几年前几天时间写的，非常潦草，无力维护，闲来无事，开个源玩玩。

## 部署：

- 复制script目录下的js脚本，打开油猴脚本插件，新建脚本，粘贴进去
- 装好Python和Flask，到back目录下启动fuckbilibili.bat

运行时会产生一个db文件，这个是sqlite数据库，里面存的是屏蔽用户列表，可以用sqlitestudio打开。

还有一个是热门页视频查找用户UID的缓存文件。



## 环境需求

油猴脚本插件
Python
Flask
textual
