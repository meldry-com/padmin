# 运营领域改写与逐源验收

更新：2026-10-23。活动实现为 `src/ui/pages/operations.rs`、`src/ui/pages/appservices.rs` 和相应 typed API；页面使用 Glory Cage/Bond 与有限语义 builders，源项目未修改。

## 当前行为清单

| 路由 | 已实现的行为 |
|---|---|
| `/media` | 服务端搜索/排序/25 条分页、已提交过滤/方向/页码恢复、用户跳转；local bulk cleanup、remote purge、单媒体删除/隔离/保护；参数校验、危险确认、失败提示、写后统计自动回读 |
| `/reports` | 服务端时间排序/反转/分页、返回状态恢复、加载/空/失败/重试、report_filed root 通知与详情导航 |
| `/reports/:report_id` | 完整 room/event/reporter/sender/reason/score/JSON；3 审核状态与回读；关联实体真 Link；reported-event redact、sender ban、room block；确认删除和列表刷新 |
| `/destinations` | 服务端排序/分页、状态恢复、失败重试、编码身份详情导航 |
| `/destinations/:destination_id` | retry/failure/stream ordering、确认 reset、结果反馈和成功后的详情回读 |
| `/registration-tokens` | 搜索/全部/有效/失效，额度耗尽与时间到期；本地每页至多 25 项；指定或自动 token、次数/长度/日期校验、复制/确认删除、列表更新；截止日期保持原源 UTC 23:59:59 |
| `/appservices` | 原 12 模板/说明/类别/搜索/唯一 ID；注册/读取/URL CAS 编辑/启停/删除；不可原地修改字段明确只读；namespace/protocols/flags 保真；YAML/JSON 粘贴或文件导入、下载/复制、secure token、显式 reveal；精确 namespace 重复预检 |
| `/auth-status` | version/flows/issuer/OIDC 4 endpoints/scopes、site-config 公开字段；login/well-known/delegated auth/register/consent 探测；保留 endpoint、原始 HTTP 状态和错误来源，注册 5xx 显示失败 |
| `/server-status` | sidecar 配置/持久化、host/health/maintenance、process/lock；component label/category/reason/url/help、真实 Help Link、刷新/失败重试 |
| `/server-actions` | 命令目录/说明/参数/锁、确认执行；一次性/周期计划的读/增/改/删；辅助字段与未编辑 args JSON 类型保真；创建成功后回填 ID 并进入编辑，ID 编辑锁定 |
| `/server-notices` | 单发验证/正文保留/真实 event ID；全用户逐页广播、并发至多 8、进度/部分失败计数；原键 server_notice_history 持久化、至多 50 条、离页重挂与确认清空 |
| `/server-notifications` | sidecar list/refresh/empty/error/确认清空；root 应用通知的读取/已读/移除/清空 |
| `/settings/notifications` | 原 5 类偏好与存储键、直接更新 root preferences Cage、立即影响通知收集 |

## 生命周期、数据和交互

- `widgets::submit` 归属页面 Owner；busy 阻止重复提交，future 不持有 Scope，离页取消后不写 retired Cage。普通受控输入逐字符更新不会重建整页。
- 媒体统计回读请求若遇列表仍在加载，会保留一次刷新意图；列表成功写后刷新不依赖用户再次点击。翻页/反转排序不在同一 Cage 写借用内重入读取。
- 文件选择器是独立浏览器 Promise 服务，2 MiB 上限；JS 只回传文本，由页面任务在 host turn 应用，不向语义树注入外来 DOM。
- token 使用真实 Web Crypto；失败不回退为伪随机值。默认 Password purpose，显式 reveal 修改原节点 purpose；复制等待 Clipboard promise 并展示拒绝错误。用户内容只作为文本呈现。
- 下载使用 Blob URL 并释放；namespaces 保持 JSON 结构和表达式原文，完整正则语法由服务器 engine 判断，避免 regex-lite 错拒合法 Unicode 类。
- protocols 导入后未编辑时保留完整 Option/Vec（包括字符串中的逗号）；args 未编辑时保留原 JSON Value，String("42")、String("{}") 和 Some(空串) 不自动变成 Number/Object/None。实际编辑才解析 JSON-or-text。
- 广播每次仅持有一页最多 500 个用户、8 个发送 future 和 50 条历史，无所有访问对象的缓存；每个成功/失败完成后推进计数，部分失败不报告全成功。
- 表单有可见标签与可访问名称；静态文本及 Check 名称绑定 root 语言，运营文案位于 `locales/operations-zh-CN.json`。危险操作采用原生 confirm，取消不发送写请求。

## 原源 API / 行为缺陷修正

### Appservice 编辑

真实 Palpo 与其路由源码均不支持原源 `PUT /appservices/{id}`，实测 405。活动 API 使用真实 `PUT /appservices/{id}/url`，body 严格只有 `url` 与编辑开始时读取的 `expected_url`（包括显式 null）。409 保留输入并要求回读，不能假成功。tokens/sender/namespaces/protocols/flags/disabled 原地只读；轮换凭据需要独立确认删除后新建，禁止隐式 unregister/register 模拟更新。新建 disabled 注册会明确调用 disable endpoint，不依赖服务器忽略的 body 字段。

### 媒体清理

真实服务拒绝原源 `POST /media/{server_name}/delete`（405）。现为 `POST /media/delete?before_ts=...&size_gt=...`，数值校验 i64 上界。`POST /purge_media_cache?before_ts=...` 读取实际 `deleted`，避免将缺失 total 错报为零。

该 Palpo 不支持 keep_profiles；默认与实际服务一致包含 profile media，页面明确说明。勾选保留会拒绝提交，不悄悄丢弃选择。隔离/保护仍调用独立 typed endpoint，404/405 明确显示服务器不支持，不伪造成功状态。

### namespace conflict

原 `e2e/appservices.spec.ts` 要求不同 ID 的相同 exclusive regex 被拒绝；真实 Palpo 原本允许两次注册均 200。迁移补应用精确重复预检：读取当前列表，以最多 8 并发逐项读取既有注册；任一读取失败即中止，不默认“无冲突”。users/aliases/rooms 各自同类 regex 字符串完全相同且任一方 exclusive 时，本地 `HttpError::message("Namespace conflict: ...")`，原注册保留、第二 POST 不发出。

此检查不具备服务端原子性，也不判断不同 regex 的任意语言重叠；完整并发 registry 约束仍需服务器实现。没有制造 HTTP 409 或将本地校验冒称服务器拒绝。

### 逐源审查追加修复

逐行对照源 API 调用/分支和原 e2e 后，补齐原首版遗漏的举报 moderation 三动作及实体链接、notice 广播与历史；恢复令牌 UTC 日期结束时间，并修复 schedule 二次保存新建、args/protocols 改值、component category/Help 丢失。此次结论不以编译或早期 suite 通过替代源语义审查。

## 参考源码逐语义映射

| 原页面 / 组件 | 当前迁移与证据 |
|---|---|
| media.rs、media_ops.rs | stats/search/order/page、用户跳转、两类清理与回读；protocol 写/取消/非法值/错误，真实 upload/delete/metadata 404/bulk/purge |
| reports.rs | 列表/详情/3 状态/实体 Link/redact/ban/block/delete/返回/通知；protocol 三 moderation 的 exact payload 与取消，真实非空 report/redact/block/状态/delete |
| destinations.rs | list/detail/状态字段/reset；protocol 编码、回读、返回状态；真实 collection 查询 |
| registration_tokens.rs | search/filter/generate/uses/expiry/copy/delete；protocol end-of-day exact expiry 与生成/过滤/删除，真实 CRUD |
| appservices.rs | 全模板/类别/搜索/唯一 ID、结构化表单/flags/protocol、文件/粘贴 YAML/JSON、token/复制/下载、CRUD、namespace-conflict；protocol exact payload/CAS/失败/reveal，真实 registry/CRUD |
| auth_status.rs | version/flows/issuer/OIDC endpoints/scopes、well-known/delegation、UIA/403/5xx/consent；protocol 状态分类和 endpoint 来源 |
| server_status.rs | 配置、health/maintenance/process/lock/component 数据和 Help；protocol 请求/结果/锚点 |
| server_actions.rs、scheduled_commands.rs | catalog/lock/run/args、两类计划 CRUD/辅助字段；protocol typed args 与未编辑 string/空串 roundtrip、创建后 Save 为 PUT |
| server_notices.rs、组件 server_notices.rs | 单发、广播、并发/进度/部分失败、50 历史/清空；protocol 取消/部分失败/正文/历史重挂；真实 single/broadcast/history |
| server_notifications.rs | sidecar list/clear/error 与 root 本地通知集成；protocol list/clear/prefs |
| notification_preferences.rs | 5 类偏好、原存储键、root 即时生效；protocol toggles/persistence，应用隔离由主线 lifecycle suite 覆盖 |

原 e2e 的 table/select/modal DOM 选择器改为当前语义 UI 的可访问名称、Link、Check、记录结构，业务断言保留。旧源码无运行时入口或兼容层；最终 release 两套回归已包含上述追加项通过，本领域对应参考源码具备删除条件。

## 验收入口与实际环境边界

- `node scripts/browser-operations.cjs`：真实 Edge + 真实 PKCE 登录，网络协议 fixture 逐项断言 method/path/query/body/响应后 UI、确认取消、非法参数、服务端错误、Clipboard 拒绝、文件导入/下载、namespace fail-closed、CAS 409、广播部分失败与历史。结果 `D:/BuildCache/padmin-glory/operations-results.json`。
- `node scripts/browser-real-operations.cjs`：隔离真实 Pasion + Palpo，无 response 路由或伪造权限；PKCE/consent/admin verdict、token/Appservice CRUD、URL CAS/namespace 预检、实际媒体 upload/delete/metadata 404、bulk/purge、真实 Matrix room/message/report 与审核/撤回/阻止/删除、notice single/broadcast/history。结果 `D:/BuildCache/padmin-glory/real-operation-results.json`。测试只临时持有本次 HTTP Authorization，不输出/落盘；临时 token/Appservice/媒体/房间均清理。
- 纯测试覆盖数值上界、YAML wire roundtrip、模板 namespace 形状/唯一 ID、namespace 各类 exclusive 组合、原 args JSON 类型保持；统一 Cargo 构建/测试由主线执行，所有输出在 D 盘。

真实联邦集合没有远端目标，reset 由完整协议 fixture 验证；隔离环境没有 sidecar，命令/两类计划/服务端通知以完整协议 fixture 验证。真实 Palpo quarantine/protect 404/405 属明确服务器能力边界；未声称建立真实远端联邦、验证真实 bridge 连接、真实客户端收件或执行缺失 sidecar。不能通过删除参考源码抹除这些环境边界。

最终 release 复核：协议 suite 9 组、40 次 mutation，浏览器异常 0、未注册写请求 0；真实 suite 11 组、浏览器异常 0。协议 suite 在确认/路由提交屏障修正后独立连续两次全量通过，修正仅涉及测试等待实际 native dialog dismissal 与页面挂载，不更改生产源码。两份结果包含本节全部源语义追加回归。对应运营参考源码可删除；上文服务器能力与环境边界继续保留。
