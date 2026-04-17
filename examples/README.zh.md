# Palpo 技术栈示例

完整的本地开发环境，包含 Matrix 聊天服务器、OAuth/OIDC 认证服务、管理后台和 Element Web 客户端。

## 服务列表

| 服务 | 说明 | 主机端口 | 容器端口 |
|------|------|----------|----------|
| **postgres** | PostgreSQL 数据库（palpo 和 pasion 共用） | `15432` | `5432` |
| **palpo** | Matrix 聊天服务器（Client-Server API） | `8008`, `8448` | `8008`, `8448` |
| **pasion** | OAuth 2.0 / OpenID Connect 认证服务 | `7080` | `7080` |
| **padmin** | 管理后台 Web UI（nginx + Dioxus WASM） | `7060` | `80` |
| **element** | Element Web Matrix 客户端 | `7070` | `80` |

## 快速开始

```bash
# 1. 根据需要调整配置文件
#    - palpo.toml    （Matrix 服务器配置）
#    - pasion.yaml   （OAuth/OIDC 配置）
#    - element-config.json （Element Web 配置）

# 2. 构建并启动所有服务
docker compose up -d --build

# 3. 在浏览器中访问
#    管理后台：    http://localhost:7060
#    认证服务：    http://localhost:7080
#    Element 客户端：http://localhost:7070
#    Matrix API：  http://localhost:8008
```

## Smoke 测试

示例栈 smoke 测试已经并入仓库根目录的 Playwright 工作区，不再单独放在
`examples/e2e/` 下。

请在仓库根目录执行：

```bash
npm install
npx playwright install chromium
npm run test:example-stack:fresh
```

如果需要单独观察重置流程：

```bash
npm run stack:reset
npm run test:example-stack
```

具体实现和限制说明见 [`../e2e/example-stack/README.md`](../e2e/example-stack/README.md)。

## 架构

```
浏览器
  |
  +---> :7070  Element Web  ----+
  +---> :7080  Pasion (认证) ---+--> :8008 Palpo (Matrix) --> :5432 PostgreSQL
  +---> :7060  Padmin (管理) ---+                                    |
                                                                     |
         Pasion (认证) --------------------------------------------->+
```

- **Palpo** 是 Matrix 聊天服务器，处理 Client-Server API 和联邦协议。
- **Pasion** 提供 OAuth 2.0 / OIDC 认证服务。Palpo 通过 `palpo.toml` 中的 `delegated_auth` 配置将认证委托给 Pasion。
- **Padmin** 是基于 Dioxus WASM 的静态管理后台应用，由 nginx 提供服务。
- **Element** 是标准的 Matrix Web 客户端，已预配置连接到本地 Palpo 实例。
- **PostgreSQL** 托管两个数据库：`palpo`（服务器数据）和 `pasion`（认证数据），由 `init-db.sh` 初始化。

## 配置文件

| 文件 | 用途 |
|------|------|
| `palpo.toml` | Palpo 服务器设置：服务器名称、数据库、联邦、委托认证 |
| `pasion.yaml` | Pasion OAuth/OIDC：数据库、Matrix 集成、密码策略、上游提供商、邮件 |
| `pasion-signing-key.pem` | Pasion JWT 令牌签名密钥 |
| `element-config.json` | Element Web 客户端配置：服务器 URL 和服务器名称 |
| `init-db.sh` | PostgreSQL 入口脚本，用于创建 `pasion` 数据库 |
| `nginx.conf` | Dioxus WASM 应用的 nginx 参考配置（默认未挂载） |

## 数据库

- **用户名**：`palpo`
- **密码**：`changeme`
- **主机**：`localhost:15432`（从宿主机访问）或 `postgres:5432`（从容器内访问）
- **数据库**：`palpo`、`pasion`

从宿主机连接：

```bash
psql -h localhost -p 15432 -U palpo -d palpo
psql -h localhost -p 15432 -U palpo -d pasion
```

## 默认凭据和密钥

> **警告**：以下为开发环境默认值，非本地部署前请务必修改。

| 设置 | 值 | 文件 |
|------|-----|------|
| Postgres 密码 | `changeme` | `compose.yml`、`palpo.toml`、`pasion.yaml` |
| MAS 共享密钥 | `replace-with-a-random-secret` | `palpo.toml`、`pasion.yaml` |
| 加密密钥 | `0a1b2c...`（十六进制字符串） | `pasion.yaml` |

## 上游 OAuth 提供商

Pasion 已预配置 GitHub OAuth 提供商。使用步骤：

1. 前往 <https://github.com/settings/developers> 创建 GitHub OAuth App。
2. 将 **Authorization callback URL** 设置为：
   ```
   http://localhost:7080/upstream/callback/<provider_id>
   ```
   其中 `<provider_id>` 对应 `pasion.yaml` 中该提供商的 `id` 字段（例如 `01KMQDVNFWTRF9K8FV8FCFKARM`）。
3. 将 `pasion.yaml` 中的 `client_id` 和 `client_secret` 替换为你自己应用的凭据。

> **注意**：`pasion.yaml` 中 `Palpo Admin Dashboard` 客户端的 `redirect_uris` 必须与 padmin 实际暴露的端口一致（`http://localhost:7060/oauth/callback`），不能写 Pasion 的端口。

## 数据卷

| 卷名 | 用途 |
|------|------|
| `postgres_data` | PostgreSQL 数据目录 |
| `palpo_media` | 上传的媒体文件 |

重置所有数据：

```bash
docker compose down -v
```

## 常用命令

```bash
# 查看指定服务的日志
docker compose logs -f pasion

# 重新构建单个服务
docker compose build padmin

# 重启单个服务
docker compose restart palpo

# 停止所有服务
docker compose down

# 停止并删除所有数据
docker compose down -v
```
