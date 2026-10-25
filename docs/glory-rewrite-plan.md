> Historical reference migration record from 2026-10-23. Current repository/branch integration and verification: [glory-branch-integration.md](glory-branch-integration.md).

# Padmin → Glory 改写任务与验收清单

更新：2026-10-23。用户已授权完整改写、并行代理、最终验收及旧参考源码清理。此文件为当前执行清单；早期暂停点和首版状态已由实际实现与测试取代。

源：`D:/Works/meldry/padmin`；目标：`D:/Works/glory-rs/padmin-glory`。应用身份保持 `padmin`，源工作树不修改。

## 执行范围

- [x] 固化源 HEAD、98 个 Rust 文件哈希、工作树状态与原 e2e inventory。
- [x] 对照源页面、组件、API 和原测试逐项复核；没有把“保留未编译参考代码”计作实现。
- [x] 使用纯手写 Glory semantic builders、Cage/Bond/Lotus 和 Owner，替换旧 UI、router、hooks、全局响应式模型。
- [x] 配置加载/失败重试、真实 PKCE/state、whoami、管理员权限、刷新单飞、撤权、登出与跨会话响应保护。
- [x] 应用根与页面所有权、native listener 输入排队、卸载清理、bounded cache/metrics/notifications/list。
- [x] 根/多层 prefix、history/back/深链、正常 Link 与新标签、404/未授权状态。
- [x] 双语、主题/系统/高对比、响应式导航、键盘、对话框焦点、受控编辑/UTF-16、通知和 toast。
- [x] 用户/房间完整专用读写、CSV、批量部分失败、列表状态与关联面板刷新。
- [x] 媒体/举报/联邦/令牌、12 个 Appservice 模板、YAML 文件服务和实际 URL CAS。
- [x] 认证诊断、sidecar 命令/两类计划、notice 单发/广播/历史、通知偏好。
- [x] Pasion 账户/权限/邮箱/风险/会话/provider/link/审计/健康/通道/模板；完整原表单与分页语义。
- [x] D 盘编译/缓存/浏览器输出、metadata 核对、精确 WASM 依赖版本、构建/部署/CI/双语 README。
- [x] 独立区分 fixture、真实服务、真实 nginx、Docker runtime 与 Docker image build 的证据边界。
- [x] Glory 发现与应用修复报告；没有确认框架 bug 或无根据扩充旧兼容 API。
- [x] 最终正式构建：8 套 fixture、6 套真实服务、4 套部署验收通过；原断言映射无待核对项；69 个旧参考源码已删除。见[完整性复核](glory-completion-audit.md)。

## 33 个具体路由及 fallback

所有路由都有专用活动 dispatch。明细能力、实际回归和外部后端限制见链接。

| 领域 | 路由 | 逐项证据 |
|---|---|---|
| 认证 | `/login`、`/oauth/callback` | auth/browser fixtures、实际 Pasion PKCE、同源 nginx |
| 首页 | `/` | dashboard 源断言、实际统计 |
| 用户 | `/users`、`/users/create`、`/users/known`、`/users/known/:user_id`、`/users/:user_id` | [用户/房间覆盖](coverage-users-rooms.md) |
| 房间 | `/rooms`、`/rooms/:room_id` | [用户/房间覆盖](coverage-users-rooms.md) |
| 媒体/审核/联邦 | `/media`、`/reports`、`/reports/:report_id`、`/destinations`、`/destinations/:destination_id` | [运营覆盖](coverage-operations.md) |
| 令牌/应用服务 | `/registration-tokens`、`/appservices` | [运营覆盖](coverage-operations.md) |
| 诊断/运维/通知 | `/auth-status`、`/server-status`、`/server-actions`、`/server-notices`、`/server-notifications`、`/settings/notifications` | [运营覆盖](coverage-operations.md) |
| Pasion | `/pasion/accounts`、`/pasion/accounts/:user_id`、`/pasion/audit-log`、`/pasion/oauth2-sessions`、`/pasion/personal-sessions`、`/pasion/connector-health`、`/pasion/upstream-providers`、`/pasion/upstream-links`、`/pasion/notification-channels`、`/pasion/notification-templates` | [Pasion 覆盖](coverage-pasion.md) |
| fallback | `/*` | 公开404、Dashboard恢复、编码/prefix边界测试 |

## 完成标准与复核资料

源业务等价允许重新设计 DOM 和交互布局，不要求保留旧 table/tab/select 选择器。不得以原源不存在的 `/pasion/policy-data` 或当前后端已变的 wire contract 虚构页面/服务响应；每项差异保留在 [原断言映射](e2e-behavior-map.json)。

高风险动作确认前零写入；错误/取消/部分失败不报全成功；原有字符串 JSON 参数不自动改类型；异步离页与旧会话回包不复活状态；普通输入更新不重建控件；token 不落持久存储。

[改写报告](glory-migration.md)记录架构、启动与实际能力边界；[完整性复核](glory-completion-audit.md)记录正式产物、测试结果和旧源码清理结论。编译与单测不代替浏览器/后端验收；无真实 sidecar/远端联邦或 Docker builder 存储条件时，明确记录已验证范围，不假称外部能力通过。
