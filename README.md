# Stranger API - 基于Rust的高性能WebSocket聊天服务

## 项目介绍

Stranger API 是一个使用 Rust 语言开发的轻量级、高性能 WebSocket 聊天服务，专为陌生人随机匹配聊天场景设计。利用 Rust 语言的内存安全特性和高性能优势，实现了低内存占用、高并发支持的聊天服务。

web页面：[https://github.com/pengsy94/stranger](https://github.com/pengsy94/stranger)

## 核心功能

### 1. WebSocket 实时通信
- 支持多用户同时在线
- 连接时返回客户端ID和在线人数
- 提供完整的心跳检测机制

### 2. 陌生人匹配聊天
- 支持用户点击匹配按钮寻找聊天对象
- **优先匹配不同性别的用户**
- 无异性时按加入顺序匹配
- **10秒超时退出机制**，避免用户长时间等待
- 支持用户取消匹配或断开连接

### 3. 消息类型支持
- **文本消息**：支持普通文本聊天
- **阅后即焚消息**：发送后被阅读立即销毁
- **系统消息**：系统通知和状态更新
- **错误消息**：操作失败时的错误提示

### 4. 通信功能
- **一对一私聊**：支持用户之间的私密聊天
- **广播消息**：向所有在线用户发送消息
- **用户列表**：获取当前在线用户列表

## 技术栈

### 后端框架
- **Axum 0.8.8**：高性能 Rust Web 框架，用于处理 HTTP 请求和 WebSocket 连接
- **Tokio**：异步运行时，提供高效的异步IO和并发支持

### 数据库
- **Sea-ORM**：Rust 的 ORM 框架，支持自动生成数据库实体

### WebSocket 支持
- **axum-ws**：Axum 内置的 WebSocket 支持
- **serde**：用于 JSON 序列化和反序列化

### 其他技术
- **tracing**：高性能日志系统
- **uuid**：生成唯一客户端ID
- **redis**：可选的分布式支持

## 安装与运行

### 环境要求
- Rust 1.70+
- Cargo

### 运行步骤

1. 克隆项目
```shell
git clone <repository-url>
cd stranger-api
```

2. 安装依赖
```shell
cargo build
```

3. 启动服务
```shell
cargo run
```

## 数据库设置

### 自动生成实体类
对于数据库表实体类，可通过 sea-orm 的命令工具自动生成：

```shell
# 先安装自动生成实体工具cli
cargo install sea-orm-cli
# 执行生成实体命令 -o 指定生成实体的文件目录 --with-serde 指定Model允许序列化和反序列化
sea-orm-cli generate entity -u mysql://root:root@127.0.0.1:3306/axum_db --with-serde both -o database/src/entity 
```

## WebSocket API

### 连接初始化

连接到 WebSocket 服务时，服务器会返回客户端 ID 和在线人数：

```json
{"type": "connected", "data": {"client_id": "04a56e58-798d-4111-970b-07ed9fafeea2", "online_count": 2}}
```

### 消息格式

#### 客户端发送消息格式：
```json
{"type": "消息类型", "参数1": "值1", "参数2": "值2"}
```

#### 服务器响应格式：
```json
{"type": "消息类型", "data": {"参数1": "值1", "参数2": "值2"}}
```

### 支持的消息类型

#### 1. 匹配聊天
```json
{"type": "meet", "user_key": "用户标识", "age_index": 2, "sex_index": 1, "location": "北京"}
```

#### 2. 一对一私聊
```json
{"type": "private", "to": "目标用户ID", "message": {"type": 1, "burnAfterRead": false, "text": "你好"}}
```

#### 3. 获取用户列表
```json
{"type": "list"}
```

#### 4. 心跳检测
```json
{"type": "ping"}
```

#### 5. 广播消息
```json
{"type": "broadcast", "message": "大家好"}
```

#### 6. 离开聊天
```json
{"type": "depart", "to": "对方用户ID"}
```

## 项目结构

```
stranger-api/
├── app/                     # 主应用代码
│   ├── src/                 # 源代码
│   │   ├── websocket/       # WebSocket 相关代码
│   │   │   ├── handler.rs   # WebSocket 请求处理
│   │   │   └── types.rs     # 消息类型定义
│   │   └── main.rs          # 应用入口
├── database/                # 数据库相关
│   └── src/                 # 数据库实体和操作
├── common/                  # 通用代码
├── kernel/                  # 核心功能
├── Cargo.toml               # 项目依赖
└── README.md                # 项目说明
```

## 性能特点

- **低内存占用**：利用 Rust 语言特性，内存占用极低
- **高性能**：支持高并发连接和消息处理
- **安全可靠**：Rust 的内存安全保证避免了常见的安全问题
- **稳定运行**：异步设计确保系统稳定运行

## 应用场景

- 陌生人随机聊天平台
- 实时客服系统
- 在线教育即时通信
- 多人协作工具

## 开发说明

### 日志
使用 tracing 系统记录日志，可通过环境变量配置日志级别：

```shell
RUST_LOG=info cargo run
```

### 测试
```shell
cargo test
```

## 许可证

[MIT](LICENSE)


