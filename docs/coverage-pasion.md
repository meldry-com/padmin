# Pasion 页面覆盖

更新：2026-10-23。本文记录当前实现、源语义复核、已执行fixture与真实后端边界。最终 release 验收和参考源码清理由主控报告统一记录。

## 当前路由与行为

| 路由 | 已实现能力 |
| --- | --- |
| `/pasion/accounts` | 服务端 search/status/admin 筛选、25条cursor分页；成功接受的filter/cursor/history跨页面保存；失败回滚及重试，未Apply输入不参与翻页；创建账户、可选初始密码/跳过策略/管理员授权；后续设置部分失败准确显示并保留详情入口 |
| `/pasion/accounts/:user_id` | profile/avatar/locale；管理员授予撤销、锁定解锁、停用恢复及可选不可逆homeserver擦除；阻止自身撤权/锁定/停用；密码及可选结束会话；risk lock/force_password_reset/terminate_sessions与原因；邮箱添加删除；browser/OAuth会话读取和单项结束；状态变更后重载账户与关联集合 |
| `/pasion/audit-log` | 1–1000条限制、对已加载记录按operation过滤；时间/管理员/资源/详情；刷新及加载/空/失败状态 |
| `/pasion/oauth2-sessions` | user筛选、client/status/scope/时间/IP、单项详情、结束确认和重取列表 |
| `/pasion/personal-sessions` | 列表/详情；必填actor user ID、human name、scope创建；重新生成/撤销确认；一次性access token显示、复制、主动清除；操作后重读列表 |
| `/pasion/connector-health` | provider/homeserver/status/error与刷新 |
| `/pasion/upstream-providers` | 列表含brand、详情及原25项配置；创建、只发送变化字段的PATCH、启停/删除；config来源只读；secret留空保留/显式清除/新值；必填、枚举、i32、key=value和claims JSON验证；保留未修改的未知既有枚举 |
| `/pasion/upstream-links` | user/provider筛选；user/provider/subject/human-name创建；确认删除及本地列表更新 |
| `/pasion/notification-channels` | channel/configured列表与刷新；仅展示实际服务端能力 |
| `/pasion/notification-templates` | key/description列表与选择；locale/channel/subject/body编辑、验证、确认发布与真实反馈 |

## 协议、所有权与边界

动态ID、username和筛选字符串分别URL编码；JSON:API resource ID覆盖同名attributes字段，畸形resource或缺失data数组报错。Pasion账户分页保留page[first]/page[after]语义。全量列表（包括provider、email、browser/OAuth sessions）只挂载25行窗口。

真实后端验收发现原个人令牌请求错误地使用可选name/user_id，且未读取真实access_token字段。当前服务改为typed `CreatePersonalSessionRequest`，必填 `actor_user_id` / `human_name` / `scope`；准确解析 `actor_user_id` / `human_name` / `access_token`，并有协议回归测试。真实创建、再生和撤销均已通过。

任务随页面Owner退休，异步future不捕获Scope或包含Scope的刷新回调；busy防重复提交、回包使用安全写入。密码/新secret成功后清除，token仅在页面Cage中持有。确认与clipboard使用真实浏览器能力，拒绝不会伪装成功。中文/英文文案绑定应用语言，通知与偏好使用应用context。

独立复核见 [领域完成复核](domain-completion-review.md) 和 [逐源API映射](domain-api-source-map.json)：

- Provider原form的25项字段和全部token-auth/discovery/PKCE/response/backchannel枚举均保留。输入采用有名称且验证的Input/Check；未沿用旧select DOM。claims_imports编辑时留空按原实现保留既有值。
- 通知channels原页面仅channel/configured与Refresh/Retry，没有detail或管理写接口。Connector health是独立读取能力。
- 模板列表原API不返回已发布正文，因此当前不声称可以读取/回填旧正文。邮箱沿用原添加/删除能力，不凭存在的协议入口虚构confirmed/is_primary编辑流程。
- accounts以外的服务维持原首批读取能力；界面的25行窗口不表示已遍历全部后台cursor，也不把审计limit当作后台总量。
- 创建完整成功进入详情；后续密码/授权部分失败保留准确错误和新账户入口，防止误重复创建。原来源中的自动跳转DOM流程不作为原封不动通过的断言。
- 原测试中的`/pasion/policy-data`在捕获的source router与模块中没有实现，应在 [行为映射](e2e-behavior-map.json) 标记旧测试漂移，不虚构路由/API。

## 已执行行为证据

| 套件 | 覆盖与结果 |
| --- | --- |
| [browser-domains.cjs](../scripts/browser-domains.cjs) | Pasion cursor/filter恢复与失败重试、账户profile/lock、确认取消零写入、关联重载、provider changed-only PATCH、个人token显示/清除/撤销、audit operation过滤。[结果](D:/BuildCache/padmin-glory/browser-domains-results.json) |
| [browser-domains-writes.cjs](../scripts/browser-domains-writes.cjs) | 邮箱增删、browser/OAuth结束会话、密码/risk-action、upstream link创建删除、模板发布的method/path/body。[结果](D:/BuildCache/padmin-glory/browser-domains-writes-results.json) |
| [browser-real-domains.cjs](../scripts/browser-real-domains.cjs) | 真实PKCE/密码/consent及9组业务：账户create/profile/admin/lock/email/password/risk/deactivate/reactivate；provider create/PATCH/secret写入清除/启停/delete；identity link增删；个人token create/regenerate/revoke/dismiss；audit/OAuth/connector/channel/template读取与隔离key模板发布。[结果](D:/BuildCache/padmin-glory/real-pasion-domain-results.json) |

领域最终扩充fixture总计25组（10个domains、10个domains-writes、5个rooms-writes），已在同一最新版debug bundle通过；这里不累积历史首轮计数。fixture套件使用实际Glory/WASM事件与协议响应，真实Pasion套件不拦截响应；204按成功空响应处理。真实后端细粒度OAuth筛选、connector刷新、共享权限/键盘/IME/clipboard等额外原断言由全项目映射及主控验收记录，不能从GET 200或编译推断其全部通过。
