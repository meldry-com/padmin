# Palpo Admin

基于 [Dioxus](https://dioxuslabs.com/) 构建的 [Palpo](https://github.com/palpo-im/palpo) Matrix 聊天服务器 Web 管理后台，编译为 WebAssembly 运行。

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

- **[Dioxus](https://dioxuslabs.com/)** - Rust UI 框架，编译为 WebAssembly
- **[gloo](https://gloo-rs.web.app/)** - Rust/WASM 工具库（网络、存储、定时器）
- **[wasm-bindgen](https://rustwasm.github.io/docs/wasm-bindgen/)** - Rust/JavaScript 互操作
- **Nginx** - 静态文件服务器（Docker 中使用）

## 前置要求

- [Rust](https://rustup.rs/)（最新稳定版）
- [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started)：`cargo install dioxus-cli`
- `wasm32-unknown-unknown` 编译目标：`rustup target add wasm32-unknown-unknown`

## 开发

```bash
# 启动开发服务器（支持热重载）
dx serve

# 生产环境构建
dx build --release
```

开发服务器默认运行在 `http://localhost:8080`。

## Docker

```bash
# 构建 Docker 镜像
docker build -t palpo-admin .

# 在 9090 端口运行
docker run -p 9090:80 palpo-admin
```

镜像采用多阶段构建：Rust/Dioxus 编译 WASM 应用，然后由 nginx 提供静态文件服务。

GitHub Actions 会将 `linux/amd64` 和 `linux/arm64` 多架构镜像发布到 GHCR：

```bash
docker pull ghcr.io/meldry-com/padmin:latest
```

## 完整技术栈示例

参见 [`examples/`](examples/) 目录，包含完整的 Docker Compose 配置，可同时运行 Palpo Admin、Palpo 服务器、Pasion 认证服务、Element Web 客户端和 PostgreSQL。

```bash
cd examples
docker compose up -d --build
# 访问 http://localhost:9090
```

## 项目结构

```
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
  ...
```

## 许可证

详见 [LICENSE](LICENSE)。
