# gateflow.net

`gateflow.net` 是一个多服务网关工作区，包含三个核心组件：

- `gateflow`（Rust）：网关核心（`HTTP 数据面`、`Admin gRPC`、`UDP 健康上报接收`）
- `healthd`（Go）：健康检查与 UDP 上报服务
- `client`（Go）：用于网关管理操作的 CLI

## 仓库结构

- `gateflow/`：Rust 服务、数据库迁移、网关文档
- `healthd/`：Go 健康探测守护进程
- `client/`：Go 命令行客户端

## 快速开始

1. 启动依赖（主要是 Postgres）并配置各服务参数。
2. 运行 `gateflow`：
```bash
cd gateflow
cargo run .
```
3. 运行 `client`（登录示例）：
```bash
cd client
APP_HOST=127.0.0.1:50051 go run . login --username <user> --password <pass>
```
4. 运行 `healthd`：
```bash
cd healthd
go run .
```

## 联调顺序（推荐）

建议按如下顺序完成全链路联调：

1. 先启动 `gateflow`（提供 Admin gRPC 与 UDP 接收）。
2. 使用 `client` 执行 `login`，拿到 `sessionToken`。
3. 将该 token 填入 `healthd/healthd.yaml` 的 `gateway_session_token`。
4. 启动 `healthd`，确认 `gateflow` 能接收健康上报。
5. 使用 `client app list/show` 验证控制面与健康链路是否打通。

快速冒烟命令：

```bash
# 1) gateflow
cd gateflow && cargo run .

# 2) client 登录
cd ../client
APP_HOST=127.0.0.1:50051 go run . login --username <user> --password <pass>

# 3) 配置 healthd token（编辑 healthd/healthd.yaml）后启动
cd ../healthd
go run .
```

## 测试命令

- Gateflow：
```bash
cd gateflow
cargo test
```

- healthd：
```bash
cd healthd
GOCACHE=/tmp/.gocache-healthd go test ./...
```

- client：
```bash
cd client
GOCACHE=/tmp/.gocache-client go test ./...
```

## 文档

- 网关概览与运维：
  - `gateflow/docs/README.md`
  - `gateflow/docs/OPERATIONS.md`
- Git 提交规范：
  - `gateflow/docs/git.md`

## 提交规范（摘要）

使用格式：

```text
<type>(<project>): <subject>
```

- `project`：`client` | `healthd` | `gateflow`
- 单个提交不要混合不同项目目录改动。

## 许可证

本仓库采用 Apache License 2.0。
详情见 `LICENSE` 与 `NOTICE`。
