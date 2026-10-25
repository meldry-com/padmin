# 视觉迁移修复与验收（2026-10-25）

本轮以原 Dioxus 应用源码 `55eb50ec07f4fd867ad8255a7f6e279061eaea95` 为视觉参照，实际构建旧应用并通过同一组状态化 API/OAuth fixture 在真实 Edge 中对照。旧站为 `http://127.0.0.1:18789`，新站为 `http://127.0.0.1:8789`。仅在旧版截图隐藏 Dioxus 开发热更新提示；没有修改旧产品 CSS。

持久依赖已切至官方最新 main 的干净检出，不使用其它任务的脏工作目录：

- Glory：`D:/Works/glory-rs/main/glory`，`b9d06545c701510ba18d5cdb7cd363c102cad906`。
- Sapid：`D:/Works/glory-rs/main/sapid`，`c5c4e23aeb6d37b3f2537545116bac43dbb40a64`。
- Cargo 使用 `../../glory-rs/main/...`；CI 的检出目录和固定版本同步更新。Sapid 最后一次 main 更新仅涉及展示/部署文件，运行源码与本轮实现一致。

## 实际修复

删除迁移时新增的硬编码简化外观，保留原 Soft Orbit CSS、字体、颜色、玻璃面板和渐变。新增语义样式适配层，明确覆盖 Glory `BASE_CSS` 对字体的继承规则，防止标题、说明和数字被重置。原 53 个 SVG 图标的几何路径转为静态 CSS mask；未引入旧运行时。

恢复独立登录屏幕、64px 盾牌图标、448px 登录卡片、渐变按钮和页脚；管理页面恢复 256px 分组侧栏、56px 头部、原生语言下拉、主题切换、通知入口、身份头像和移动抽屉。Dashboard 使用六项统计的两列三行共边框布局。

用户、房间、媒体、举报、联邦、注册令牌和 Pasion 账号恢复有列头的真实 Sapid 表格、工具栏、筛选、空状态和分页。排序、列显示、选中项及原 API 请求保持可操作。用户创建恢复面包屑、独占整行的 Username、原字段提示、两列后续字段、原生 User type 和底部操作。Glory main 的 Input 当前没有 placeholder 属性，因此占位提示由纯 Rust 语义 Widget 根据受控空值渲染；不注入 DOM 属性或 JS UI。

CSV、房间创建、账号创建、Appservice 编辑、媒体清理和令牌创建使用实际 Sapid Dialog，保留表单状态和 Escape/关闭行为。Appservice 恢复已安装表和三列模板卡片。GET 列表/详情加载结果留在页面，错误仍保留提示，显式写操作继续显示 toast。

## 可复现验证

所有编译、Cargo 缓存、编译临时文件、浏览器 profile、截图及报告位于 D 盘。构建入口在执行前通过 `cargo metadata --no-deps --format-version 1` 核验 `D:/BuildCache/meldry-visual-20261025/padmin/target`。

```powershell
$env:WASM_BINDGEN = 'D:/BuildCache/glory-website/tools/wasm-bindgen-0.2.129-x86_64-pc-windows-msvc/wasm-bindgen.exe'
$env:PADMIN_PLAYWRIGHT = 'D:/BuildCache/glory-dependencies/browser/node_modules/playwright'
python scripts/glory.py build --dev # 迭代；正式构建去掉 --dev
python scripts/glory.py check
python scripts/glory.py test
$env:PORT = '8789'
node scripts/real-server.cjs # 另一个终端运行验收
node scripts/visual-reference.cjs # 需要旧站18789
node scripts/visual-audit.cjs
npm test
```

正式产物使用 release，源码输入 SHA256：`0bad39cb4c7869fe8a66fa1155eac2430ed3ec56fa94c89c7d97579bbdabfba7`；WASM SHA256：`882dcccf47534691b05e4633cbecd24c110cdb2b78efe69e800f9980e4073082`。

| 验证 | 覆盖与证据 |
| --- | --- |
| native all-targets、wasm32 check | `check-final.log`，通过 |
| Rust 单测 | `test-final.log`，25 通过 |
| Rust 格式、补丁空白 | package 级 rustfmt；`diff-check.log`，通过 |
| release 构建与绑定 | `build-release-final.log`、`dist/build-info.json`，构建前后指纹一致 |
| 路由外观 | `visual-audit.json`：25 个管理路由加登录，1440/375 两种视口，52 次真实页面测量/截图；检查标题字号、桌面壳几何、登录居中、全页横向溢出 |
| 原版视觉参照 | `../comparison/padmin-old-*.png`：登录、Dashboard、用户、创建、房间、媒体、举报、Appservice、Pasion 账号；各族通过实际源码和截图复核 |
| 有数据详情 | `detail-user-1440.png`、`detail-room-1440.png`、`detail-account-1440.png`、`detail-provider-1440.png` 随领域精确 payload fixture 生成 |
| 权限、协议、写操作 | `acceptance-fixtures.json`：最终单次统一驱动 8 套全部 exit 0，69 组通过（9+12+6+8+10+10+5+9）；绑定上述 release 的实际 HTTP WASM 与源码指纹，不拼接旧成功报告 |

上表未写绝对路径的报告均相对 `D:/BuildCache/meldry-visual-20261025/padmin/`。开发迭代失败日志不作为最终成功证据。

## 边界

这次验证是实际 WASM/Edge、状态化 OAuth/API fixture 和原页面对照，不替代真实 Pasion/Palpo/Matrix 后端、远端联邦、生产 sidecar、屏幕阅读器或 OS IME 验收。未构建、发布 Docker 镜像。并非每个参数化详情路由都有旧版配对截图，也不宣称字体渲染、每个动态数据状态或每个像素完全一致。

测试跟随恢复后的真实 UI：原来的文本框或按钮改为原生下拉，原来的平铺表单需先打开对话框；保留请求方法、路径、查询、body、取消、错误、部分失败和回读断言。跨标签退出测试先让两个页面分别完成真实 fixture PKCE，随后验证 storage 事件退会话；不依靠登录页面上错误出现的管理头部，也不把 token 写进浏览器持久存储。
