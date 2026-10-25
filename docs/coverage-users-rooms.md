# Users、Rooms 与 Dashboard 覆盖

更新：2026-10-23。本文记录当前实现、独立源语义复核和已执行行为证据。最终 release 构建、全项目验收与参考源码清理由主控报告统一记录。

## 当前功能

| 路由 | 已实现能力 |
| --- | --- |
| `/` | 服务器版本、用户/活跃用户/房间/举报计数；Matrix versions、启用的 unstable features、API 平均延迟；逐指标失败显示、刷新与领域导航 |
| `/users` | 服务端搜索、用户名/显示名/管理员/创建时间排序及方向；10/25/50/100 行分页；筛选/页码/排序/列显示持久化；当前页选择、停用/恢复/擦除/删除设备/授权管理员批量操作；确认、部分失败明细和写后重载；CSV 导入预览/验证及完整分页导出 |
| `/users/create` | 500ms 用户名可用性防抖与过期响应隔离、提交前复核；显示名、密码及强度提示、邮箱/国际电话、regular/bot/support、locked/admin；Pasion 管理员归属限制；成功详情导航、Cancel 返回列表 |
| `/users/known` | 搜索、服务器/本地/远程筛选；user_id/server_name/joined_rooms/total_rooms/last_membership_at 排序与方向；分页及状态恢复；远程账户只读 |
| `/users/known/:user_id` | 身份/主服务器/资料、成员关系房间、房间导航；仅本地且 has_account 时显示管理账户入口；关系集合按25行窗口展示 |
| `/users/:user_id` | 资料及 admin/shadow-ban/suspended/locked、Pasion 账户入口、归属感知的停用/恢复/擦除、密码重置及设备重载；3PID 添加/删除、设备和已加入房间分页、单设备/全部设备删除；whois 会话、只读账户数据、速率覆盖与恢复默认、实验功能、服务器公告 |
| `/rooms` | 服务端搜索、名称/成员/room_id/state_events 排序；分页与状态恢复；当前服务端页的 all/public/private/encrypted 筛选；选择/清除、批量封锁/删除、逐项失败明细与重载；public/private 创建，默认 public 与原实现一致 |
| `/rooms/:room_id` | 资料/state、名称/topic、目录可见性/join/history、alias/canonical alias；成员分页及 invite/kick/ban/unban/admin；最新100条消息、event读取/撤回、hierarchy/extremities；block/unblock/purge/delete；受影响资料和集合写后刷新 |

## 服务、数据与生命周期

页面使用 Glory builders、Cage/Bond/Lotus 和页面 Owner 任务。HTTP 保留在原业务服务与 `user_extras` 专用服务中。异步 future 不捕获 Scope；离页使任务随 Owner 退休。用户名检查使用代次阻止延迟响应覆盖新输入；提交有 busy 防重。账号所有权由服务端和既有 owner-aware API 决定，Pasion 查找失败不会降级为本地授权。

危险操作使用真实浏览器确认。批量操作保留成功数量和逐项失败；设备全删重复读取第一页，避免删除后位移遗漏。密码重置会重读已展示的设备 rows/total/page。user_registered 与房间删除通知接入应用 context，偏好与通知生命周期由共享应用层管理。

CSV 导入沿用粘贴入口，要求 `id`；可选 displayname/password/admin/deactivated，支持 BOM、CRLF、引号/逗号/多行字段，拒绝重复ID、畸形布尔值及超过5,000行。先验证全部输入，既有账户跳过，空密码安全生成。导出保留源 `users_export.csv`、user_id 表头和秒时间戳，并保留对 `= + - @ TAB CR` 开头数据的公式防护。

消息维持源最新100条语义。全量 state/hierarchy/known-user-room 集合由页面持有，显示时只克隆并挂载25行窗口，不保留已访问行缓存。JSON 仅用于账户数据、state/event 等业务数据查看。

## 原语义复核与界面差异

独立复核见 [领域完成复核](domain-completion-review.md) 和 [逐源API映射](domain-api-source-map.json)。领域源业务调用没有未映射项。复核发现的房间批量/类型筛选/state排序、用户admin/created列、known排序参数、CSV防护与格式、密码强度、公告清空、dashboard功能/延迟均已移植并验证。

用户详情采用带名称的面板，列表采用语义 records，替代旧 tab/table/placeholder DOM。User details 是按钮，用户ID当前是文本，因此不声称原行链接 href 断言直接通过；详情编码导航由领域套件验证，链接新标签行为由共享导航套件验证。创建成功进入详情，随后可返回 Users 搜索新账户；Cancel 直接返回 Users。原验收断言的等价映射和结构差异见 [全项目行为映射](e2e-behavior-map.json)。

源 `user_account_data` 组件仅 GET 与 JSON 查看，没有遗漏写操作。whois 的 devices→sessions→connections 层级、IP/time/agent 均保留。源 CSV 文案声称 displayname 可替代 id，但原 parser 实际仍必须 id；当前文案按真实能力修正。

## 已执行验收与真实后端边界

| 套件 | 已执行证据 |
| --- | --- |
| [browser-domains.cjs](../scripts/browser-domains.cjs) | 10组：dashboard功能；分页/筛选/列恢复、失败回滚重试；远程只读与有界关系；room资料刷新；Pasion cursor/account/provider/token/audit流程。[结果](D:/BuildCache/padmin-glory/browser-domains-results.json) |
| [browser-domains-writes.cjs](../scripts/browser-domains-writes.cjs) | 10组：用户 profile/password/device/3PID/rate/feature/notice 请求；whois和账户数据读取；实际CSV下载与公式防护、部分导入；可用性/验证/密码强度/创建/Cancel。[结果](D:/BuildCache/padmin-glory/browser-domains-writes-results.json) |
| [browser-rooms-writes.cjs](../scripts/browser-rooms-writes.cjs) | 5组：类型筛选/state排序、bulk部分失败/删除；create/topic/directory/join/history、alias、成员操作、复杂ID撤回、block/purge/delete及确认/验证。[结果](D:/BuildCache/padmin-glory/browser-rooms-writes-results.json) |
| [browser-real-matrix.cjs](../scripts/browser-real-matrix.cjs) | 真实PKCE/Pasion/Palpo：用户查找/创建/修改、数据/会话/设备/房间面板、速率/3PID、归属停用恢复；房间创建与state/alias/member/block操作、known详情及登出。[结果](D:/BuildCache/padmin-glory/browser-real-matrix-results.json) |

前三套使用实际 Glory/WASM 输入和状态协议 fixture，验证 method/path/body、失败、取消和实际下载；真实 Matrix 套件没有拦截响应。当前隔离 Palpo 对 experimental_features 真实返回404，页面呈现该不支持结果，不能将 fixture 中的成功写入称作该服务器已支持。异步删除/停用任务仅报告服务端接受或读回状态；receipt 不代表后台作业已最终完成。平台IME、键盘、主题、权限与资源释放由共享应用验收报告给出，本文不从领域编译或fixture推导这些结论。
