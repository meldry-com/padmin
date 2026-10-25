# Glory 分支集成

2026-10-25：在 `D:/Works/meldry/padmin` 创建 `glory` 分支。以当前 `55eb50ec07f4fd867ad8255a7f6e279061eaea95` 为基线，逐一核对参考改写记录的 98 个源 Rust 文件，SHA256 全部一致后导入已完成的语义改写。保留原有未跟踪 `rustfmt.toml`、`typos.toml`。

应用使用 `../../glory-rs/glory/crates/glory` 和 `../../glory-rs/sapid`，metadata 实际解析到用户指定的本地目录。公共 action 按钮由 `sapid::Button` 实现，选择其公开 `unstyled()` 选项保留应用明暗主题与高对比样式。路由、请求、授权和业务数据仍由应用管理；没有 RSX、旧运行时或 Signal 兼容层。适配当前 Glory 的 `widget` 和只读 `LiveDescriptor::view()` API。

构建入口 `python scripts/glory.py` 显式设置 D 盘 target、Cargo home、编译临时目录，并核对 `cargo metadata`。正式 WASM 的 build-info 指纹覆盖应用源文件、locale、清单/锁文件、宿主入口以及本地 Glory/Sapid 源码；浏览器验收在执行前重新计算指纹并核验所服务 WASM，避免误用旧产物。

本地构建与验收输出使用 `D:/BuildCache/padmin-glory-branch`，复用已有 D 盘 wasm-bindgen 与 Playwright 安装。远端 CI 通过显式 `PADMIN_REMOTE_BUILD=1` 和绝对 `PADMIN_BUILD_ROOT` 设置远端缓存，检出与本机使用版本一致的 Glory/Sapid。保留原有 main/tag/PR 触发、运行配置冒烟、多架构和双镜像仓库发布能力。未执行本机 Docker 构建。

## 验证

本轮结果见下方更新记录。历史 `glory-migration.md`、`glory-completion-audit.md`、`acceptance-summary.json` 和旧 e2e 映射描述 2026-10-23 参考目录的验收，不能代替本分支的实际构建/浏览器验证。其“源目录未变”的叙述属于当时的独立副本工作流；本分支按本轮用户指示修改原 Git 仓库。

| 本轮检查 | 结果 |
| --- | --- |
| native all-targets / wasm32 check | 通过 |
| Rust 单测 | 25 通过；不可变源码快照再次执行 25 通过 |
| rustfmt / git diff --check | 通过；仅继承配置的 stable/nightly 选项提示 |
| release WASM + wasm-bindgen | 通过，构建前后输入指纹相同 |
| 实际 Edge 浏览器协议/交互 fixture | 8 套、69 组全部通过，8 个 suite 及总 runner 均真实 exit 0 |
| entrypoint 有/无 Pasion 配置 | 两项通过；仅重定向生成路径并省略 nginx 启动，不称为镜像验收 |
| 容器运行时打包 | `D:/BuildCache/padmin-glory-branch/container`；未执行 Docker build/发布 |

外部 Glory/Sapid 同时被其它任务修改，前两次 release 在编译成功后由输入指纹保护拒绝验收。最终使用 `D:/BuildCache/padmin-glory-branch/source-snapshot` 下的不可变应用/库源码构建；保留 `meldry/padmin` 与 `glory-rs/{glory,sapid}` 布局，原仓库清单仍直接指向用户指定目录。快照捕获时间和逐文件 SHA256 在 `snapshot-hashes.json`。本轮完整浏览器报告为 `acceptance-fixtures.json`，同时记录实际 `source_root` 与 `test_root`，不拼接以前成功结果。外部目录后续又发生变化，因此这些结果对应明确记录的快照，后续库变更需要重新构建。

源码输入 SHA256：`177e85232b1bf4631209cab5992fad3ad3c1f94d801228ab68ec0a40c792b509`。

正式 WASM SHA256：`fc952a37f89b8ac247d5017a00d8fd7845ffc33a4b80b1fd82fd23c2ffbf5553`。

Windows Edge 的 SDK 优雅关闭确认曾阻塞验收。fixture 现在使用自有 BrowserServer，并仅对其明确 PID 树执行 native teardown，随后正向确认 PID 已消失。完整 async 流程、所有原断言与实际回收结束后 CLI 才退出；没有把超时或未执行断言记为通过。默认 locator 等待设为 20 秒，heartbeat 场景保留原显式 36 秒上限。远程连接的下载改用 `saveAs` 到 D 盘唯一目录，保留原文件名与 YAML 内容断言。

本轮未重新运行真实 Pasion/Palpo/Matrix 外部栈验收；69 组属于真实 WASM/浏览器执行、状态化 API 协议 fixture，不替代真实外部服务验收。历史真实服务结果仍仅作历史证据。
