> Cargo 依赖统一使用 Glory/Sapid 的 GitHub `.git` 地址和 `main` 分支，`Cargo.lock` 固定实际提交，默认构建使用同一 Git 依赖图与 D 盘缓存。详见[依赖来源与更新](docs/dependency-sources.md)。

> 2026-10-25 视觉修复：以实际旧页面为参照恢复布局，并使用官方 Glory/Sapid main。当前截图矩阵、版本和验收记录见[视觉修复报告](docs/visual-migration-2026-10-25.md)。

> 当前 UI 使用 Glory 与 Sapid；[分支集成记录](docs/glory-branch-integration.md)及 10 月 23 日报告描述早期迁移阶段。本地编译与测试输出均在 D 盘；Docker 构建器需使用 D 盘或远端存储。

# Palpo Admin

基于 Glory 构建的 [Palpo](https://github.com/palpo-im/palpo) Matrix 聊天服务器 Web 管理后台，编译为 WebAssembly 运行。

## 功能

- **仪表盘** - 服务器概览和统计数据
- **用户管理** - 查看、搜索和管理 Matrix 用户
- **房间管理** - 浏览和管理聊天房间
- **媒体管理** - 查看和管理上传的媒体文件
- **服务器状态** - 监控服务器健康状况和配置
- **注册令牌** - 创建和管理注册令牌
- **举报** - 审查用户/房间举报
- **服务器通知** - 发送全服务器公告
- **认证状态** - 查看认证和委托认证状态
- **联邦目标** - 管理联邦通信目标

## 技术栈

- **Glory** - Rust UI 框架，编译为 WebAssembly
- **Sapid** - 公共操作按钮，保留应用原有主题样式
- **[gloo](https://gloo-rs.web.app/)** - Rust/WASM 工具库（网络、存储、定时器）
- **[wasm-bindgen](https://rustwasm.github.io/docs/wasm-bindgen/)** - Rust/JavaScript 互操作
- **Nginx** - 静态文件服务器（Docker 中使用）

## 前置要求

- Python 3.11+
- [Rust](https://rustup.rs/)（最新稳定版）
- wasm-bindgen CLI 0.2.129（安装到 `D:/BuildCache/meldry-visual-20261025/padmin/tools`）
- `wasm32-unknown-unknown` 编译目标：`rustup target add wasm32-unknown-unknown`

## 开发

```bash
# 启动开发服务器（支持热重载）
python scripts/glory.py serve

# 生产环境构建
python scripts/glory.py build
```

开发服务器默认运行在 `http://localhost:8080`。

## Docker

```bash
# 构建 Docker 镜像
powershell -File scripts/package-container.ps1
# 先核验 Docker builder 的实际数据位置在 D 盘或远端：
docker build -t palpo-admin D:/BuildCache/meldry-visual-20261025/padmin/container

# 在 9090 端口运行
docker run -p 9090:8080 -e PALPO_URL=http://your-palpo:8008 -e PASION_URL=http://your-pasion:7080 palpo-admin
```

镜像使用在 D 盘预构建的 Glory JS/WASM，由非 root nginx 提供静态服务。
Nginx entrypoint 也会暴露 `/healthz`，方便容器与平台进行存活检查。

CI 按 Cargo.lock 获取 Glory、Sapid 的 Git 提交，在远端 Linux runner 构建并测试 WASM，保留原有 push/tag 触发、运行配置冒烟测试、双架构镜像以及 GHCR/可选 Docker Hub 发布。

## 完整技术栈示例

参见 [`examples/`](examples/) 目录，包含完整的 Docker Compose 配置，可同时运行 Palpo Admin、Palpo 服务器、Pasion 认证服务、Element Web 客户端和 PostgreSQL。

```bash
cd examples
docker compose up -d --build
# 访问 http://localhost:7060
```

Compose 示例统一使用 [`examples/README.zh.md`](examples/README.zh.md) 中的本地端口：

- `padmin`：`http://localhost:7060`
- `pasion`：`http://localhost:7080`
- `element`：`http://localhost:7070`

## 端到端测试

仓库现在只保留一套根目录 `e2e/` Playwright 工作区。

```bash
# padmin 主回归套件
npm test

# examples/compose.yml 示例栈 smoke 套件
npm run test:source-baseline:example-stack:fresh
```

示例栈 smoke 的运行方式、fixture 和限制见 [`e2e/example-stack/README.md`](e2e/example-stack/README.md)。

## 项目结构

```
playwright.config.ts               # padmin 主 Playwright 配置
playwright.example-stack.config.ts # 示例栈 smoke 配置
e2e/                              # 统一后的 Playwright 工作区
src/
  main.rs          # 应用入口
  router.rs        # 客户端路由
  api/             # Palpo 服务器 HTTP API 客户端
  components/      # 可复用 UI 组件
  pages/           # 页面组件（仪表盘、用户、房间等）
  types/           # 数据类型和 API 模型
  utils/           # 工具函数
  style.css        # 全局样式
examples/
  compose.yml      # 完整技术栈 Docker Compose
  palpo.toml       # Palpo 服务器配置
  pasion.yaml      # Pasion 认证配置
  README.md        # 示例栈部署说明
```

## 许可证

详见 [LICENSE](LICENSE)。

## 验收与运行

先设置 `WASM_BINDGEN=D:/BuildCache/meldry-visual-20261025/padmin/tools/bin/wasm-bindgen.exe`，运行 `python scripts/glory.py build --dev`；正式发布去掉 `--dev`。浏览器驱动安装到 D 盘：`npm install --prefix D:/BuildCache/meldry-visual-20261025/padmin/runtime --cache D:/BuildCache/meldry-visual-20261025/padmin/npm-cache --ignore-scripts playwright@1.63.0`。测试使用已安装的 Microsoft Edge，profile 和报告均在 D 盘。

- `npm test`：运行实际 WASM 的权限/协议、键盘/IME/可访问树、移动导航和所有领域写操作 fixture，空闲时自动启动 8789 静态服务。未注册写请求返回 501。
- `powershell -File scripts/start-real-stack.ps1`：复用已有 Pasion/PostgreSQL 程序与本机已缓存 Palpo 镜像，建立隔离真实验收服务；不构建、不拉镜像、不修改原项目。
- `node scripts/real-server.cjs` 后运行 `npm run test:real`：真实密码登录/同意/PKCE、Matrix/Pasion 业务写入、邮箱注册、两个用户收发消息及刷新后的举报状态。其他隔离账号可通过 `PADMIN_REAL_USERNAME`、`PADMIN_REAL_PASSWORD` 指定。
- `node scripts/nginx-acceptance.cjs` 验证生产入口与非 root nginx；`node scripts/native-nginx-acceptance.cjs --same-origin` 用 D 盘真实 JS/WASM 验证根路径、`/admin`、`/console/manage` 和同源 `/auth` 全链路。需准备隔离服务与 D 盘 nginx 1.31.2 程序。

验收器拒绝缺失/过期产物及失败 suite，并将实际 WASM SHA256、退出码和证据路径记录在 `D:/BuildCache/meldry-visual-20261025/padmin/acceptance-results.json`。原 18 suites 通过 `test:source-baseline` 保留为历史断言，原 DOM 选择器与 localStorage token 注入不再是活动测试入口；逐项迁移和三项明确的源基线/后端差异见行为映射。

`glory.py serve` 仅提供静态文件；真实同源 API 可通过 `real-server.cjs` 和 `PALPO_URL`/`PASION_URL` 配置。生产 nginx 的应用/认证前缀支持保持 CSP 禁止 `unsafe-inline`，独立的 Pasion 配置桥只解析公开 JSON，不执行代码。

`glory.py build/serve` 总会生成 `config.json`。可用 `--pasion-public-url`/`PASION_PUBLIC_URL`、`--oauth-client-id`/`PADMIN_OAUTH_CLIENT_ID`、`--palpo-admin-url`/`PALPO_ADMIN_URL` 配置；空 URL 采用同源默认，client 默认匹配示例注册。静态 server 不代理后端；真实登录需反向代理或使用 `real-server.cjs`。
