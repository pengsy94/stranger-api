# Axum Api Core

使用 axum 0.8.8 搭建的 `web` 基础服务, 由于阿里云等在线服务器价格有点吃不消, 所以用 `rust` 搞一个内存占用极少的方案

### 关于 sea-orm

对于繁琐的数据库表实体类，可通过sea-orm的命令工具自动去生成

```shell
  # 先安装自动生成实体工具cli
  cargo install sea-orm-cli
  # 执行生成实体命令 -o 指定生成实体的文件目录 --with-serde 指定Model允许序列化和反序列化
  sea-orm-cli generate entity -u mysql://root:root@127.0.0.1:3306/axum_db --with-serde both -o database/src/entity 
```

### 关于 websocket

初始化链接到ws的时候，会将自身的 `client_id` 返回给客户端, 并且会说明当前在线客户端数量

```json
{"type": "connected", "client_id": "04a56e58-798d-4111-970b-07ed9fafeea2", "online_count": 2}
```

目前支持ws支持的操作有

- 获取用户列表

```json 
{"type": "list"} 
```

- 心跳包

```json
{"type": "ping"}
```

- 一对一私聊

```json
{"type": "private", "to": "目标ID", "message": "你好"}
```

- 广播信息

```json
{"type": "broadcast", "message": "大家好"}
```

