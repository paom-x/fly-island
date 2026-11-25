# ZCode 钩子契约与「岛上审批」可行性实测（2026-11-25）

反馈/目标：用户主用 ZCode + WorkBuddy + DSH，希望 ZCode 的会话状态上岛，并且能在岛上直接批准或拒绝 ZCode 请求确认的工具调用。本文记录实机探针的结论，作为后续两批改动的依据。

探针环境：ZCode `4.0.0`（Electron 44.7.0 / Chrome 152）、`D:\ZCODE`、配置 `~/.zcode/cli/config.json`。所有结论都来自实机回放，证据是钩子自己落盘的 `stdin-<event>-<ts>.json` 与 `probe.log`（探针目录一次性使用，未入库）。

## 1. 配置位置与生效条件

- 用户级配置在 `~/.zcode/cli/config.json` 的顶层 `hooks` 键；工作区级是 `<repo>/.zcode/config.json` 或 `zcode.json`。
- **配置方式注册的钩子默认不跑**，必须写 `"enabled": true`；只有插件贡献钩子时运行器才会自动打开。
- 事件恰好七个：`SessionStart`、`UserPromptSubmit`、`PreToolUse`、`PermissionRequest`、`PostToolUse`、`PostToolUseFailure`、`Stop`。`Notification`、`SubagentStop`、`PreCompact` 都不支持。
- 本次探针只在 `~/.zcode/cli/config.json` 加了 `hooks` 段，七个事件各挂一条 `type: "process"` 钩子（`command` 指向 `node.exe`，`args` 指向探针脚本）。

## 2. 进程启动的两个坑

| 现象 | 原因 | 处理 |
|---|---|---|
| `ZCode.exe` 立即退出，`exit 0`，没有窗口也没有日志 | 被继承的 `ELECTRON_RUN_AS_NODE=1` 让 Electron 退化成纯 Node | 启动前 `Remove-Item Env:ELECTRON_RUN_AS_NODE` |
| `--remote-debugging-port=9333` 报 `bad option`，退出码 9 | 同上，参数被 Node 解析 | 同上；清掉该环境变量后端口正常监听 |

打开 CDP 后可用 `Input.dispatchMouseEvent` / `Input.insertText` / `Input.dispatchKeyEvent` 驱动界面。**ZCode 的 UI 自动化树不可用**（Electron 未开 accessibility，只有空 Pane 节点），取证只能走 CDP，不能走 UIA。查找元素时用 `element.getClientRects().length > 0` 判断可见，不要用 `offsetParent !== null`（`position: fixed` 的菜单项会漏）。

## 3. 钩子收到的数据（stdin）

同一份载荷里**同时给出 snake_case 与 camelCase 两套键**，即既兼容 Claude Code 的字段名，又带 ZCode 自己的驼峰别名。

| 字段 | 出现事件 | 说明 |
|---|---|---|
| `cwd`、`permission_mode`、`timestamp`、`traceId`、`turnId`、`mode` | 全部 | `permission_mode` 实测取值 `build`（对应界面「变更前确认」）、`guarded`（对应「自主模式」） |
| `hook_event_name` / `hookEventName` | 全部 | 七个事件之一 |
| `session_id` / `sessionId` | 全部 | 会话 id |
| `transcript_path` / `transcriptPath` | 全部 | 形如 `%TEMP%\zcode-hook-XXXXXX\transcript.jsonl` |
| `source`、`model` | `SessionStart` | `source` 实测 `startup`；`model` 形如 `magpie/auto/deepseek-flash` |
| `prompt` | `UserPromptSubmit` | 用户输入原文 |
| `tool_name`、`tool_input`、`tool_use_id` / `toolCallId`、`riskLevel`、`sideEffectScope` | `PreToolUse` | `riskLevel` 实测 `high` / `medium`；`sideEffectScope` 实测 `system` / `workspace` |
| `requestId`、`reason`、`riskLevel`、`sideEffectScope` | `PermissionRequest` | `requestId` 形如 `perm_<uuid>`；`reason` 实测原文 `Tool has side effects and requires approval` |
| `tool_response`、`toolResultPreview` | `PostToolUse` | `tool_response` 含 `stdout`、`status`、`exitCode`、`timedOut`、`cancelled`、`stdoutBytes` |
| `responseText`、`responsePreview`、`last_assistant_message`、`stop_hook_active` / `stopHookActive`、`toolCallCount` | `Stop` | 停止时的一次性回报 |

## 4. 钩子的输出能做什么（决定成败的四条）

1. **钩子是同步阻塞的。** 让脚本睡 8 秒，`UserPromptSubmit` 与 `PreToolUse` 都各自阻塞了 8.0 秒，ZCode 全程等待。⇒ 可以让钩子挂着等岛上决策。
2. **`PreToolUse` 返回 `allow` 能直接放行。** 输出 `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}` 之后，`PermissionRequest` **根本不再触发**，没有任何界面弹窗，工具直接执行（实测 Write 写出了文件）。
3. **`PreToolUse` 返回 `deny` 能拦死。** 附带 `permissionDecisionReason` 后工具不执行，模型收到该 reason，并在回复里复述「被 hook 拦截，返回信息 …」。
4. **`PermissionRequest` 上的决定被忽略。** 只在 `PermissionRequest` 返回 `allow`，界面**照样弹出** ZCode 自己的权限确认框，文件没有写入。⇒ 想在岛上批准，**必须在 `PreToolUse` 上决定**；`PermissionRequest` 只能当「需要审批」的信号源。

另外两条：

- **空输出 + 退出码 0 等于不干预**，ZCode 继续走自己的权限弹框。这正是理想的降级路径。
- **退出码 `2` 表示阻止**（对 `PreToolUse` / `PermissionRequest` 即拒绝），其余非零算错误；stdout 按严格 schema 解析，多一个键整份丢弃并记失败。

## 5. `riskLevel` 不能当成「ZCode 会不会问」

实测反例：`自主模式`（`guarded`）下 `Bash`（`riskLevel=high`、`sideEffectScope=system`）**没有**触发 `PermissionRequest`，直接执行；`变更前确认`（`build`）下 `Write`（`riskLevel=medium`）才触发 `PermissionRequest`。

⇒ 判断「这一步要不要送岛」不能只看 `riskLevel`。可行做法是「用户可配置的工具名集合」叠加当前 `permission_mode` 的启发式，并在设置页写明「可能与 ZCode 原生询问范围不完全一致」。

## 6. 由此确定的实现方案

- **审批走 `PreToolUse`**：钩子把 `tool_name`、`tool_input`、`toolCallId`、`session_id`、`cwd` 转发到 fly-island 的本地审批端点并阻塞等待；拿到允许就输出 `permissionDecision: "allow"`，拿到拒绝就输出 `deny` 加理由。
- **钩子必须自己超时**：等到上限仍未决定时输出空并退出 0，让 ZCode 退回自己的弹框，绝不把用户卡死。不要依赖 ZCode 的超时语义——本次把单个钩子的 `timeoutMs` 改小后未观察到生效（怀疑配置在会话启动时载入，需要重启才读），因此脚本自超时才是可控路径。
- 钩子会挂住整个 turn，等待上限要按「人要在岛上点按钮」来设。
- **只显示那一批不需要 `PreToolUse`**：`SessionStart` / `UserPromptSubmit` / `PostToolUse` / `Stop` 就够上报状态；`PermissionRequest` 的 `requestId` 与 `reason` 正好用来点亮岛上「在等你确认或回答」。
- 安装与卸载必须是幂等的，并且只增删自己的钩子条目，**不得改动用户已有的插件钩子**（本次探针结束后已把 `~/.zcode/cli/config.json` 还原为 503 字节的原始内容）；配置里标记 `enabled: true` 时也要考虑用户可能早就写了别的钩子。

## 7. 未验证项

- 钩子超时后 ZCode 的确切行为（是否等同空输出、是否标记失败并继续）。改用脚本自超时后这一项不影响实现。
- 「总是允许本项目」这类规则能否通过钩子表达。目前按「不支持」处理，岛上只做单次允许/拒绝/交回终端。
- 桌面端与 CLI 是否共用同一份 `~/.zcode/cli/config.json`：桌面端目录 `%APPDATA%\ZCode` 下没有 `config.json`，用户级配置只有 CLI 这一份，但未逐项验证桌面端会话确实读它。
