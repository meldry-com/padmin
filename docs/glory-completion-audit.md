> Historical reference migration record from 2026-10-23. Current repository/branch integration and verification: [glory-branch-integration.md](glory-branch-integration.md).

# 改写完整性与 Dioxus 清理复核

日期：2026-10-23。结论：**应用改写与最终验收已完成，旧参考源码及活动依赖已清理。** 外部后端能力和环境验收边界在下文单列，没有以模拟成功代替实际支持。

## 最终验收

所有活动源文件和 locale 对应同一份正式 release WASM，验收器同时核对本地文件与实际 HTTP 服务的 SHA256，且检查结果生成时间，拒绝旧产物/旧报告。

| 检查 | 结果 |
|---|---|
| cargo metadata | target 为 `D:/BuildCache/padmin-glory/target` |
| native 全 targets + wasm32 check | 通过 |
| Rust 单测 | 25 通过，0 失败 |
| rustfmt check | 通过；stable 对继承的 nightly 选项发出提示 |
| release WASM + wasm-bindgen | 通过，实际浏览器执行 |
| 状态化协议/交互 fixture | 8 套、68 组通过 |
| 真实 Pasion/Palpo/Matrix | 6 套、59 组通过，无 response interception 或 token 注入 |
| 部署/干净静态启动 | 4 套通过；含实际 nginx root、/admin、/console/manage 和同源 /auth |
| 原 e2e 迁移映射 | 77 测试、307 行原断言保留，待核对 0 |
| 原项目完整性 | 98 个源 Rust 文件哈希均未变；Git 工作树仍仅原有 2 个未跟踪配置文件 |

WASM SHA256：`258938a5bb72b5f13a344fdfa92c1df53303cfddaf5217d0dacf9752c3e5f3f1`。

活动源码 SHA256：`57cb486530aa9aecdbab8fffb2ebe599775fee897a83a93347560abbd76b3e8c`。

逐套命令、案例、退出码、报告位置和构建输入见 [acceptance-summary.json](acceptance-summary.json)。完整运行日志在 `D:/BuildCache/padmin-glory/` 的 check.log、test.log、build.log、fmt.log、acceptance-fixtures.log、acceptance-real.log，详细请求结果保存在各 suite 的 JSON。

最终回归修正了三处测试屏障：等待真实 token 刷新响应、筛选后的实际记录集合、原生确认取消及路由重挂完成；保留全部业务断言。完整 8 套入口在修正后重新执行通过，未以单独成功结果覆盖失败汇总。

## 源语义复核

33 个具体 route 与 fallback 均有实际实现。用户/房间/Pasion 与运营/Appservice 两组独立源复核已完成，shell/auth/deployment 由主控及验收代理另行核对。过程中补齐举报 moderation、房间批量操作、CSV 导出防护、广播/历史、audit 过滤、原 JSON 参数保真和分页恢复等早期遗漏；没有停在首版。

[原测试映射](e2e-behavior-map.json)区分 74 个已执行的语义等价测试、2 个按当前后端契约执行的调整及 1 个过时源断言。后者请求的 /pasion/policy-data 在源 router 和页面树中不存在；不是删除未迁移功能。旧 table/select/modal 选择器改为 semantic 控件的实际可访问名称和记录；原断言文本保留，不声称旧选择器原样通过。

源业务调用映射见 [domain-api-source-map.json](domain-api-source-map.json)，具体实现与协议差异见三个 coverage 文档和[总报告](glory-migration.md)。

## 清理及最终残留扫描

在上述全部通过且各领域明确允许删除后，以逐文件补丁删除 `migration/source-ui` 全部 **69 个文件**，并删除空目录。原 main/router、页面、组件、hooks/RSX/global signal 参考实现不再留在目标中。

[reference-removal.json](reference-removal.json)记录每个被删文件的路径、字节数、SHA256、删除范围和对应验收产物；没有将旧源码挪到别处规避扫描。原项目本身完整保留在用户指定的源位置。

- `cargo tree --locked --offline`：已解析依赖树无 Dioxus 包。
- src、Cargo.toml、Cargo.lock、bootstrap、宿主桥、scripts、e2e、Dockerfile、entrypoint、CI 和 examples：不区分大小写搜索 Dioxus/Dixous、RSX、GlobalSignal、use_signal/use_resource 及旧 CLI 调用，**0 命中**。
- 全目录余下的旧框架名称仅在迁移报告/历史证据中，用于说明这次清理。没有旧运行时、源码、构建入口或兼容别名。
- 删除参考后重新核对活动构建输入 SHA256，仍与已通过的 release 相同；清理没有改动已验收的生产源文件。
- 不复制示例私钥，不改原项目；所有编译缓存和浏览器测试产物在 D 盘。

## 外部验收边界

当前 Palpo 的 experimental-features、quarantine/protect 不存在，UI 呈现实际不支持响应；keep_profiles 不被静默忽略。Appservice 只对实际支持的 URL CAS 更新，其余不可变字段说明与精确 namespace 预检限制见总报告。

隔离 stack 未提供 sidecar 或远端联邦实体，其完整流程由状态化协议 fixture 验证，不能称为真实 OS 命令/计划调度或跨服务器恢复验收。未构建/发布 Docker 镜像，原因是 builder 的 C 盘数据存储不符合用户约束；实际复用镜像运行、非 root nginx 以及 D 盘完整 WASM nginx 已验收。DOM/AX 与合成 composition 也不冒充 OS IME 或真实屏幕阅读器验收。
