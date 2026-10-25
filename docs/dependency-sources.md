# Cargo 依赖来源与更新

2026-10-26：Glory 与 Sapid 统一使用 GitHub 的 .git URL 和 main 分支。默认构建、直接 Cargo 命令、CI 和 Dependabot 使用同一份提交到仓库的清单与锁；没有本地 path 覆盖、镜像清单或镜像锁。

## 清单与锁

Cargo.toml 中的地址为 https://github.com/glory-rs/glory.git 与 https://github.com/sapid-ui/sapid.git，两项均指定 branch = "main"。Sapid 当前清单引用相邻 Glory，因此 patch 表将该依赖指向完全相同的 Glory Git URL/main 分支，避免两份框架类型。

Cargo.lock 记录实际 Git 提交：本轮 Glory 为 b9d06545c701510ba18d5cdb7cd363c102cad906，Sapid 为 c5c4e23aeb6d37b3f2537545116bac43dbb40a64。main 表示更新目标；日常 locked 构建仍使用锁定提交，不会每次构建自动漂移。

Dependabot 的 Cargo、npm、GitHub Actions 更新仍启用，Glory/Sapid 也可通过正常依赖 PR 提议更新。人工更新可在已设置 D 盘缓存的环境执行 cargo update，检查新的 Cargo.lock，再运行下述构建/测试和相应 UI 验收。不要通过本机路径或额外 Cargo patch 覆盖这些 Git 源。

## 构建入口

脚本需要 Python 3.11+。python scripts/glory.py 的 check、test、build 均执行仓库 Cargo 清单并使用 --locked。本机 CARGO_HOME、CARGO_TARGET_DIR 及临时输出固定在 D 盘；WSL 需要核验 D 挂载；远端 CI 使用显式 remote build root。执行前通过 cargo metadata --locked --no-deps --format-version 1 核验实际 target 目录。

源码摘要覆盖应用、构建脚本、Cargo.toml 与 Cargo.lock；锁中的 Git commit 身份是外部依赖构建输入，不再读取 D:/Works/glory-rs 的任何共享工作树。build-info.json 另记录 git_sources。构建前后摘要必须一致。Rust 源码仍直接位于仓库 src/，CARGO_MANIFEST_DIR 仍是原仓库目录。

CI 先运行完整 cargo metadata --locked，随后执行同一构建入口。依赖由 Cargo 获取至配置好的缓存，不再额外 checkout Glory/Sapid。Docker 只打包已生成的 dist 静态产物，保留运行配置冒烟、双架构、GHCR 和可选 Docker Hub 发布。

## 失败根因与历史验证

旧提交 9967cc7 的 Cargo 更新任务 40222575838、40222575889、40222570698 在文件读取阶段找不到仓库外 Glory/Sapid path；这不是 Rust 编译失败或外部服务故障。

过渡提交 1136a7c 的独立 Git 清单修复已在远端证明可行：Cargo 扫描 40224332753、npm 40224332965、GitHub Actions 40224333005 及 futures/serde_json/serde-saphyr 的三个重算任务均成功；Docker 40224325287 的 smoke-test、双架构构建和镜像发布均成功。该提交曾使用本地镜像构建入口，现已按统一 Git main 约束删除，不能将其旧证据冒充最终入口的远端验收。历史日志保留在 D:/BuildCache/padmin-ci-20261026/remote-1136a7c/。

当前 Git main 方案的本地复验证据位于 D:/BuildCache/padmin-ci-git-main-20261026/。后续远端验收需绑定本轮新提交 SHA。没有在本机执行 Docker 构建，也未修改共享 Glory/Sapid 工作树。

本轮 Git main 默认入口的本地结果：完整 locked metadata、native all-targets/wasm32 check、25 项 Rust 单测、release 和真实 Edge 冒烟均通过；冒烟核对实际 HTTP WASM、fixture PKCE、用户创建页及中文资源，临时浏览器/服务已退出。source-verification.json 确认单一 Glory/Sapid、没有 path 依赖或外部工作树摘要输入，且构建前后仓库锁不变。YAML/Python 与补丁空白检查亦通过。源码 SHA256：64755b59f3010cae7a3271aadcb31d63e20bdd52015cecbc334483ec9858513a；WASM SHA256：35bb19ac27ed20270e9e88b56da04e9cb11299660766d34ff2cab7166b10b251。
