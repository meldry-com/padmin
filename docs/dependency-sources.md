# Cargo 依赖来源与更新

2026-10-26：修复 Dependabot 在读取仓库外 path 依赖时的失败。Docker 工作流原运行 40222459379 的 smoke-test 与镜像发布均成功；失败的是 Cargo 更新任务 40222575838、40222575889、40222570698，日志错误为 PathDependenciesNotReachable，涉及 Glory/Sapid 的仓库外相对路径。

## 两种入口

提交的 Cargo.toml / Cargo.lock 使用可独立获取的 Git 源：Glory 固定 b9d06545c701510ba18d5cdb7cd363c102cad906，Sapid 固定 c5c4e23aeb6d37b3f2537545116bac43dbb40a64。Sapid 当前清单引用相邻 Glory，因此 patch 表将其 Glory 依赖指向同一固定 Git 源，避免生成两份 Glory 类型。Dependabot 无需读取本机磁盘即可解析。

正常开发和 CI 的 python scripts/glory.py 仍默认使用本地检出：

- D:/Works/glory-rs/main/glory/crates/glory
- D:/Works/glory-rs/main/sapid

脚本需要 Python 3.11+。它在 PADMIN_BUILD_ROOT/local-manifest 生成私有 Cargo.toml，将这两项替为绝对 path，移除仅用于 Git 的 patch，并将 lib.path 指向仓库原始 src/lib.rs。没有复制或改写 Rust 源码；相对 include_str! 仍以原始源文件位置解析。该入口的 CARGO_MANIFEST_DIR 是私有目录，产品代码当前不依赖它；以后增加依赖该环境变量的资源时需同步评估路径。

仓库 Cargo.lock 只用作镜像锁的种子。Cargo 可将私有锁的框架来源改为本地 path，但脚本绝不将私有锁写回仓库；仓库清单或锁更新时，输入 SHA 会让镜像锁重新播种。其余注册表依赖沿用锁定版本。直接运行 cargo 则使用仓库的固定 Git 源，应先显式设置 D 盘 CARGO_HOME、CARGO_TARGET_DIR 和临时目录。

构建前通过 cargo metadata --no-deps 核验实际 target 位于指定 D 盘目录。本地源码摘要包含构建脚本、仓库清单/锁、原始应用与实际本地 Glory/Sapid 源码；正式构建前后摘要必须一致。CI 保留既有完整三仓库目录布局，新增 canonical metadata --locked 检查，然后按同一本地镜像入口构建；Docker 仍仅打包 dist 静态产物。

## 更新流程

Dependabot 的 Cargo、npm、GitHub Actions 更新仍启用。Glory/Sapid 两项暂不单独自动改 pin：升级时需一起调整依赖、Sapid patch、Docker workflow 的 checkout ref 和本地检出，并运行 UI 验收。其它 Cargo 依赖仍由 Dependabot 更新仓库清单和锁。人工 cargo update 也应在 D 盘环境下使用仓库清单，随后用构建脚本验证本地布局；不要用镜像锁覆盖仓库锁。

## 本轮验证

证据位于 D:/BuildCache/padmin-ci-20261026/。canonical-metadata.json 是仓库清单的 locked 完整解析，local-resolved-metadata.json 是镜像清单的 locked 完整解析。source-verification.json 核对两图的包名/版本与启用特性一致，各只有一个 Glory，原始 lib 路径一致，且本地图实际使用指定检出。check.log、test.log、build.log 记录本轮检查与构建，lock-integrity.json 记录构建前后仓库锁指纹。

旧远端任务原始日志保存在 logs/job-*.log。Dependabot 服务需在修复推送后再次运行才算远端更新验收；本地解析通过不宣称旧失败任务已变绿。没有在本机执行 Docker 构建。

本轮结果：native all-targets 与 wasm32 check 通过；25 项 Rust 单测通过；release 构建通过，前后源码指纹一致；真实 Edge 新产物哈希、fixture PKCE、用户创建页和中文资源冒烟通过，测试浏览器和临时服务已退出。独立 Git 清单的 serde_json 更新 dry-run 也通过。最终源码 SHA256 为 7668862e9eb2b8272c693e9607566dcc6baa7e34f3ae0951be3c256e6984bffb，WASM SHA256 为 3aed746899ef8159bee425bf6852dbc47c632711eaf536c9d4a1edc445fe9ca1。
