> Historical reference migration record from 2026-10-23. Current repository/branch integration and verification: [glory-branch-integration.md](glory-branch-integration.md).

# Padmin Glory 改写报告

本次改写使用 Glory 的手写 semantic builders、Cage/Bond/Lotus 和 Owner 任务生命周期，保留应用身份 `padmin`。33 个具体路由及 404 均有活动实现；界面为专用业务流程，没有用任意 JSON/API 操作台代替原页面。

源项目 `D:/Works/meldry/padmin` 未修改，基线 HEAD 为 `55eb50ec07f4fd867ad8255a7f6e279061eaea95`。原工作树仅有未跟踪的 `rustfmt.toml`、`typos.toml`。[基线清单](source-baseline.json)保留源文件哈希；[原测试映射](e2e-behavior-map.json)保留 77 个测试的 307 行原断言、当前等价行为与实际证据，不冒称旧 DOM 选择器原样通过。

## 功能和所有权

- 应用根拥有认证、语言、主题、高对比、通知和导航。页面 Owner 退休取消任务、依赖和处理器；缓存、指标、通知、批量及列表显示均有上限。原生监听器只排队，业务代码在 Glory Owner 内执行。
- OAuth 保留 S256 PKCE、一次性 state/verifier、真实 code exchange、whoami 和服务端管理员判定。刷新单飞并校验会话代数；登出、旧请求回包、定期撤权探测和跨标签登出均有回归。token 只在应用内存中，不写入持久存储。
- 路由支持正常链接、Ctrl 新标签、history/back、深链刷新以及根路径、`/admin`、`/console/manage`。配置失败阻止错误登录并提供重试。浏览器 WebApp 和事件监听器一起卸载。
- 双语切换不会替换正在编辑的控件，保留值与 UTF-16 selection。主题跟随系统，窄屏抽屉、桌面折叠、快捷键、帮助对话框焦点进入/Tab/Escape/恢复、通知偏好及 50 条容量均有浏览器验证。
- 用户/房间：专用资料与成员操作、设备/3PID/密码/速率限制、已知用户、CSV 导入导出、房间状态/别名/可见性、事件/历史/审核、批量操作与部分失败反馈。
- 运营：媒体、举报三种审核及关联 moderation、联邦目标、注册令牌、12 个 Appservice 模板和 YAML 文件导入/导出、真实 URL CAS 更新、诊断、sidecar 命令及两类计划、notice 广播与发送历史。
- Pasion：账户/权限/风险/邮箱、OAuth/browser/personal 会话、provider 的完整配置及 secret 保留/清除、上游链接、审计、连接器、通知通道和模板发布。cursor 与筛选恢复，不混用 Matrix 分页。

逐领域复核见 [用户/房间](coverage-users-rooms.md)、[运营/Appservice](coverage-operations.md)、[Pasion](coverage-pasion.md)及[源业务调用映射](domain-api-source-map.json)。

## 验收与可复现入口

最终记录集中在 [完整性复核](glory-completion-audit.md) 和 `docs/acceptance-summary.json`。验收器对源文件、locale 和实际本地/服务端 WASM 核对 SHA256，拒绝旧产物和旧结果。

```powershell
$env:WASM_BINDGEN='D:/BuildCache/padmin-glory/tools/bin/wasm-bindgen.exe'
python scripts/glory.py check
python scripts/glory.py test
python scripts/glory.py fmt
python scripts/glory.py build
npm test
npm run test:real
node scripts/native-nginx-acceptance.cjs
node scripts/native-nginx-acceptance.cjs --same-origin
```

Cargo home 为 `D:/BuildCache/cargo`；target、临时目录、浏览器运行数据与正式输出均在 `D:/BuildCache/padmin-glory`。构建器先用 cargo metadata 核对实际 target_directory。输出为该目录下 `dist/`；开发入口 `python scripts/glory.py serve --dev` 只提供静态资源，实际后端通过同源代理连接。

真实验收使用隔离 PostgreSQL、Pasion 和现有 Palpo 镜像，没有拦截真实响应或注入管理员 token。覆盖实际密码/consent/PKCE、注册与邮箱验证、双用户 Matrix 邀请/加入/发消息/sync/举报、用户/房间操作、非空媒体删除、Pasion 和 Appservice CRUD。临时测试实体由脚本清理。

生产 entrypoint 的 nginx 配置经过非 root 运行检查；完整 JS/WASM 通过 D 盘 native nginx 验证三个挂载位置及同源 `/auth` 登录。Pasion 的 public configuration 通过惰性 JSON 与外部脚本读取，保持 CSP 禁止内联执行；代理修正已确认的后端 JS 绝对 asset 前缀。没有构建或发布 Docker 镜像：现有 Docker builder 数据盘在 C，未满足 D/远端构建条件。复用镜像运行与镜像构建的证据不混淆。

## 源缺陷修正与外部能力边界

以下边界不通过虚构成功、删除页面或保留旧运行时掩盖：

- 当前 Palpo 没有 experimental-features、媒体 quarantine/protect 的实际支持，相应专用 UI 保留真实 HTTP 失败；bulk 清理不支持 keep_profiles，选择保留会阻止请求并解释。
- Palpo 不存在原源使用的完整 Appservice PUT。采用实际 `/url` CAS，仅改 URL；其余字段明确只读，通过显式删除/注册变更。重复 exclusive namespace 做读取失败即中止的精确同类 regex 预检，不声称任意 regex 重叠判断或跨客户端原子保证。
- 原个人 token 请求字段与真实 Pasion 不符，已修正为 actor_user_id/human_name/scope 与一次性 access_token。原媒体批量删除路径同样修正为真实端点。
- 源测试的 `/pasion/policy-data` 没有源路由或实现，已标记历史测试漂移。SSO-only flow 和不允许的 HTTP dynamic-registration callback 两项按当前后端实际契约验收，保留变更理由。
- 隔离服务没有 sidecar，联邦列表没有真实远端；其命令/计划/通知和 reset 完整请求、取消、失败及回读通过状态化协议 fixture 验证。未声称实际 OS 运维执行、调度触发、跨服务器联邦恢复或 bridge 连接。
- DOM/AX 快照、合成 composition 与 UTF-16 编辑检查不等于 OS IME、真实屏幕阅读器验收。

## Glory 观察和处理

没有确认的 Glory 框架缺陷，本次未修改 Glory 框架源代码。实际发现并修复的是应用集成问题：OAuth callback 因权限状态变动重挂造成 verifier 重放；同一 Cage revise 内再 get 引起借用冲突；跨会话异步回包；全局存储/缓存所有权；刷新并发；部署 asset/CSP 前缀。回归分别在认证、领域、共享壳及 nginx 套件中。

适配点：精确对齐 wasm-bindgen 0.2.129/web-sys 0.3.106；file picker、clipboard、download 是独立宿主服务，不能向 Glory 管理树注入任意外来节点。普通输入、标签与主题更新仍通过 semantic patches 完成。
