# steer — 设计思路 (IDEA)

> 本文件记录 steer 讨论中**已经想清楚的有用内容**。未定项明确标注，不作结论。

## 一句话

steer 是一个微型的**控制单元 / PC**：它解释一个由"代码 + 自然语言"写成的工作流程序，按程序计数器逐步推进。**控制语句**（赋值 / 分支 / 循环 / 函数）由 steer 自己执行；**行动就是函数调用**，唯一原语是 `task`（带 `instruction`/`return`/`check`/`produce`/`spawn` 参数），`collect`/`ask`/`command`/`print` 是基于它的糖——这些都是 agent op，**所有"做事"（读文件、跑命令、查内容、问人、验证）都归 agent**。steer 自己**只跑控制 + 算术/比较 + 状态 + 渲染**，不碰外部世界。**agent 是执行单元（ALU），steer 是控制单元（PC）。**

工作流由**人或 agent 编写**（新写一个简单的，或复用已有的），用来控制 agent 实现目标。

## 设计哲学（核心）

- 旧模型：需求文档 → 人写函数逻辑（代码）→ 编译器 / 解释器运行。
- 新模型：**需求文档就是 instruction** → agent 拿到指令执行 → **steer 只控制执行流程，像 PC（program counter）一样。**

| 计算机        | steer                          |
| ------------- | ------------------------------ |
| 内存中的程序  | 工作流脚本（代码 + 自然语言）  |
| 指令集        | 控制语句 + 函数调用            |
| 程序计数器 PC | steer 的步骤指针               |
| 执行单元 ALU  | 外部 coding agent（唯一执行者）|
| 控制单元      | steer（只跑控制，不碰外部世界）|
| 寄存器 / 内存 | 变量（执行上下文；标量/路径/答案/list）|
| trap / 中断   | `check`（读 agent 设的标志判决）|
| 子程序        | `func` / `return`（微型调用栈）|

## 关键原则：steer 不碰外部世界；靠"指令模板"改写 instruction

steer 只做四件事：

1. 维护 PC + 重试计数 + 调用栈 + 变量（执行上下文）；
2. `check` 时分派：控制节点自己执行；agent op 读 agent 经 `steer set` 设的值/标志；
3. **渲染 instruction**：把 task 参数套进**指令模板**，自动拼入回流动作（`steer set` / `steer set verified` / `steer error`）+ 变量插值 +（重试时）失败原因；
4. 写自己的 run-state / audit。

**steer 不跑 shell、不读写文件系统、不 spawn 进程、不碰终端——不碰任何外部世界。** 所有与世界的交互都渲染成指令交给 agent，结果经 `steer set` 回流。**作者不手写 `steer set`/`steer error`**——由模板按参数自动生成。

> 控制层 = 指令模板 + 上下文管理。这是与 Atomic 的本质区别：Atomic 绑自己的 UI + agent runtime；steer 把 runtime 留给 agent，连文件系统都不碰。

## 执行模型（agent 驱动的循环）

关键反转：**不是 steer 调用 agent 的循环，而是 agent（运行 steer skill）驱动循环，steer 只是一个有状态的 oracle。** skill 循环：`step`（取指令）→ 执行 → `check`（推进）→ 重复。

```
# 命令统一为 `steer <resource> <action> <args>`，两个资源：workflow（定义）/ instance（run）。

# workflow —— 作者/调试（无 instance）
steer workflow validate <wf>        静态校验：语法 + 语义（取值型 task 必须有 return、produce 是 list、end 配对等）
steer workflow simulate <wf>        dry-run：mock agent（check 自动过、ask/command 返回 canned 值）走一遍，
                                      打印每个 task 渲染后的 instruction，供人 debug 工作流与模板

# instance —— run 生命周期 + 执行循环
steer instance start <wf> <name>    PC = 0，建/清空 .steer/instances/<name>/（v1：若已存在则清空重建，不保留历史；run-id 多版本留待后续）
steer instance status <name>        run 状态（complete / failed / halted）
steer instance step <name>          返回当前 PC 节点渲染后的 instruction（不改状态）
                                      · task/ask/command/collect → 渲染指令（模板拼入 steer set 等）
                                      · 上次 FAILED → 附失败原因，仍是该节点
steer instance check <name>         推进 PC，按节点类型分派：
                                      · 控制节点（=/if/loop/for/func）→ steer 自己执行，PC 前进
                                      · 值 op（ask/command/collect）   → 查上下文里值是否已 set；已 set → 前进
                                      · task+check                     → 读当前 step 的 checked 标志 → 前进 / 记录 failure 留原位
steer instance set <name> <var> <value>  agent 把值/标志写进上下文（类型化 JSON 字面量：[1,2,3]=数组、"[1,2,3]"=字符串、42=数、true=布尔）
steer instance error <name> "<reason>"   agent 报致命失败 → run 立即停止（区别于 check 失败的重试）
# 未来 TODO：steer instance runs <name>（列历史）/ clean <name> [<run-id>] / resume <name>
```

**两个数据方向**：steer → agent（`step` 给指令 / `check` 给判决）；agent → steer（`steer set` 回流值或标志、`steer error` 报致命错）。

**红利**：PC、调用栈、变量全部可序列化（`context.json`）→ **resume 天然免费**（未完成的 current run 直接继续 `step`/`check` 即续上）。

## 语言模型：行动 = 函数调用

语法只有两类：

1. **控制语句**（steer 自己执行，不阻塞、不需 agent）：`=`（赋值）、`if/elseif/else/end`、`loop...until cond`、`for x in list...end`、`func/return/end`。
2. **函数调用**（其余一切）：内置函数或用户 `func`。

> `for` **只迭代 list 元素**（不拆字符串；要按行就得让来源返回 list）。
> **声明式 = steer 的本质**：steer 不执行领域逻辑，只**声明任务与步骤**（交给 agent），自己只跑控制。需要手动控制重试与验证时用 `task(...)` + `check` + `loop`。**v1 无 `len`/`range` 等通用 stdlib**；字符串拼接靠 `{var}` 插值。

### task 参数与指令模板（核心）

`task` 是唯一 agent 原语。参数：**`instruction`（位置，必填）+ 命名 `return`/`check`/`produce`/`spawn`**。这些都是**声明式输入**；`step` 用一个**指令模板**把它们渲染成发给 agent 的完整指令，**自动拼入回流动作**，作者不手写 `steer set`。**一个行为节点 = 一个模板文件**（`.steer/lib/templates/<node>.j2.md`，Jinja2 + Markdown）：`task`/`ask`/`command`/`collect`/`print` 各一个，用户加 `<foo>.j2.md` 即定义新行为节点 `foo(...)`。模板分 `formatter` + `body`：`formatter` = 该节点的**参数定义**（名字/类型/必填/**默认值**，纯声明，供 `validate` 校验）；`body` = Jinja2 渲染正文。模板间用 `extends`/`block` 组合（`ask.j2.md` `{% extends "task.j2.md" %}`、覆写来源 block 复用 task 正文）。steer 只把参数传进 Jinja2 渲染，**文案/拼装由模板作者决定**；`formatter` 不放执行/赋值逻辑（declarative `default` 除外）。

| 参数 | 类型 | 必填条件 | 模板注入的回流动作 |
| --- | --- | --- | --- |
| `instruction` | string | 必填 | 指令主体 |
| `return` | string | **结果被赋值 / `return` 时必填** | "把结果按 `{return}` 格式用 `steer set <var>` 设值" |
| `check` | string | 可选；给了即开验证 | "验证：`{check}`；完成后 `steer set checked true|false`（当前 step 子属性）" |
| `produce` | list | 可选 | "本步应产出文件：`{produce}`" |
| `spawn` | bool | 可选 | "用一个全新 sub-agent 执行本步" |

> `collect`/`ask`/`command` 是**"带 `return` 的取值型 task"的糖**，区别只在模板里"值的来源"提示：`ask`=自人、`command`=自 shell、`collect`=自 agent 推理。`print` = task 仅 instruction、无 `return`、无 `check`。
>
> `return=` 命名参数与 `return <expr>` 语句靠上下文 + `=` 区分，无歧义。

示例：
```
task("修复 bug")                                                  // 无值、无 check
task("分析根因，写到 artifacts/root.md", return="文件路径",
     produce=["artifacts/root.md"],
     check="确认含'根因'", spawn=false)
files = command("git diff --name-only", return="字符串数组，每行一个文件")  // 取值→return 必填
ok = ask("构建系统是？", return="单个字符串")
```

### check 验证：单一形式（v1）

`check` 是 task 的一个参数（一段指令字符串）。agent 执行验证、用 `steer set checked true|false` 设当前 step 的 `checked` 子属性（**由模板自动注入这句话**），`steer check` 读它判决：pass → PC 前进；fail → 记录原因、留原位重试。**所有验证走同一条路**。值 op（`ask`/`command`/`collect`）不需要 `check`——`steer check` 查值是否已 set 即推进。

### steer 自己求值的（仅这些）

算术与比较运算符（`+ - * / > < == !=` 等），供 `if`/`until` 在上下文值上判断。

> **不做 steer 内置**：`file_exists`/`read_file`/`contains`/`split` 等——一律用 task+`return`+`check` 或 `command` 表达。例：检查文件存在 → `task("检查 X 是否存在", return="布尔")` → agent 设布尔 → `if` 比对。

## 最小语法 v1（面向编译器实现，显式 `end` 收尾）

语句每行一条；`//` 注释；字符串支持 `{var}` 插值。

```
// 赋值 / 输出 / 提问 / 注释
x = 5
name = "hello"
print("hi")
toolchain = ask("你用的是哪个构建系统？", return="单个字符串")
// 这是注释

// 两种循环（定次 = 计数器 + until，无专门形式）
i = 0
loop
    print("hi")
    i = i + 1
until i >= 3

loop                                  // 后测：先做再判
    print(x)
    x = x + 1
until x > 5

for item in list                 // list 取元素
    print(item)
end

// 条件（if / elseif / else）
if x > 3
    print("big")
elseif x > 0
    print("small")
else
    print("zero")
end

// 函数（真子例程，微型调用栈，可 return）
func add(a, b)
    return a + b
end
```

### 行动节点示例（根因优先 bugfix）

```
func analyze(bug)
    existing = command("test -f artifacts/root-{bug}.md && echo yes || echo no",
                       return="yes 或 no")
    if existing == "yes"
        return "artifacts/root-{bug}.md"                        // return 语句
    end
    task("只分析、不要改代码。复现 bug，把根因写到 artifacts/root-{bug}.md，含复现/证据/根因。",
         return="文件路径", produce=["artifacts/root-{bug}.md"],
         check="确认 artifacts/root-{bug}.md 已生成且含'根因'")
    return "artifacts/root-{bug}.md"
end

rc = analyze("login-500")
print("根因已定位：{rc}")

gate = ask("根因在 {rc}。继续实现修复吗？", return="yes 或 no")
if gate != "yes"
    return                                                       // 顶层 return = 终止工作流
end

files = command("git diff --name-only", return="字符串数组，每行一个文件")
for f in files
    task("依据 {rc} 修复 {f}，跑 go build。", return="布尔：build 是否通过",
         check="确认 {f} 修复正确")
end

print("bugfix 完成。")
```

### 数据模型

变量 = 标量 / 路径 / 用户答案 / 列表，存在执行上下文（`context.json`）里。值的来源：`func` 的 `return` 语句、取值型 task（`ask`/`command`/`collect`）经模板注入的 `steer set` 回流、算术/比较派生。**steer 不解析 agent 自由文本、不读文件**——agent 主动 `steer set`（类型化值）或写文件后把路径作为值设回。

## 循环语义：条件求值与 `loop` 关键字

> 关键字已由 `repeat` 改名为 `loop`（**已落地到代码 / README / 示例**）；**定次形式 `repeat N … end` 已移除**——用计数器 + `until` 表达（见下），少一种形式、且消灭 count 类型 footgun。`for` 保持独立关键字。`loop` 为后测专用（必带 `until`）。下文语义与改名无关。

### 条件（`if` / `until`）= steer 自己做的单比较，不是 agent op

这是循环语义的**核心钉死项**，也是 steer 设计 DNA 的直接体现（见对照表："trap / 中断 → check（读 agent 设的标志判决）"）：

- `if cond` / `until cond` 里的 `cond` 永远是 **steer 侧的单比较**（`==`/`!=`/`</>`/`<=`/`>=` 之一），对**上下文里已有的变量**求值，steer 自己算，不碰 agent、不碰外部世界。
- 当循环退出条件**依赖外部世界**（如"构建通过了吗"），agent 的"做事"放在**循环体**里：用一个 agent op（`task`/`command`/...）取值，经模板注入的 `steer set` 写进上下文变量；`until` 再去比那个变量。
- 这就是 **PC / ALU 分工**：agent（ALU）置标志，steer（PC）按标志跳转。条件里绝不嵌 agent 动作——既守住"steer 不碰世界"，又避免与"body 里放 agent op 设变量"重复。

示例（循环到构建通过为止）：
```
loop
    ok = judge("构建是否通过？")
until ok
```
`judge` 在 body 里让 agent 做布尔判断、把结果设回 `ok`（见下）；`until ok` 是 steer 对上下文变量的单比较。

### 条件谓词：`and` / `or` / `not` + `judge` 节点

条件要能组合（"成功 **或** 已达上限"），故锁定两项增量——**都保持条件纯**：

- **`and` / `or` / `not`**：加进表达式层（`BinOp` + `UnaryOp` 变体），对布尔求值；`Value::Bool` 与 `truthy()` 已具备（`value.rs:31`），纯增量、不碰 IR/VM/resume。**call 仍不进条件**——`Expr::Call => EvalError::UnexpectedCall`（`value.rs:195`）是承重墙，"call 永远不是子表达式"。
- **`judge` 节点**：新增**默认行为节点**，agent 做布尔判断、结果进变量。实现 = 一个默认模板 `judge.j2.md`（`{% extends "task.j2.md" %}`），**无语法/AST/parser/IR 改动**（"一个模板文件 = 一个行为节点"）。**固有返回 bool、无 `return` 参数**：`formatter` 声明布尔返回，`body` 自动渲染"判断：{instruction}。只回答 `true`/`false`，再 `steer set <var>`"。作者侧一句 `passed = judge("构建是否通过？")`。

由此**有界重试**（最多 N 次、成功提前停）无需专门循环形式，被 `or` + 手动计数器取代：
```
i = 0
loop
    i = i + 1
    passed = judge("构建是否通过？")
until passed or i >= 3
```

**`judge` 与 `check` 正交，必须分清**：`judge` = 判断一次→布尔进变量（可作条件、**无重试**）；`task(check=...)` = 断言+**step 级无上限重试**（失败留原 pc 重跑）。要"判断结果进条件"用 `judge`；要"做不好就重试到好"用 `check`。

> 否决项：节点名 `condition`（名词，破坏 task/ask/command/collect/print 全动词惯例）、`expect`（自带"断言/预期"色彩，与 `check` 混、在重试场景词义打架）。`collect(..., return="bool")` 技术可行但每次要写 `return="bool"` + "只回答 true/false" 样板，被 `judge` 取代。

### 两种循环形式的精确语义（改名后）

| 形式 | 语义 | 说明 |
| --- | --- | --- |
| `loop … until cond` | 后测 | 先跑 body 再测 `cond`；`cond` 假→回边再跑，真→退出。**≥1 次**（Lua `repeat/until`）。定次 = 计数器 + `until i >= N`，无专门形式。 |
| `for x in list … end` | 遍历 | `list` 在入口求值一次进隐藏迭代器槽；每轮弹出首元素绑给 `x`，空则退出。原变量不被消耗。 |

> 关于 `until` 与 `do…while`：`loop … until c` 是**后测**（≥1 次），`c` 为**真**时退出——等价于 C 的 `do … while (!c)` / Lua 的 `repeat … until c`（同一后测结构，`until` 极性与 `while` 相反）。保留 `until` 而非 `do…while`，是因为"重试到成功"`loop … until passed` 读起来最直接，`do…while` 需套 `not`。

循环状态（`for` 的隐藏迭代器槽 `__for_n`；`loop … until` 无隐藏槽、纯回边）与 PC 一起存在 `context.json`，**resume 天然免费**：PC 指回循环头，迭代器槽留着剩余列表。

## 存储与目录布局

- **工作流脚本**：`.steer/workflows/<name>.steer`（用户编写的工作流）。
- **自定义函数库**：`.steer/lib/*.steer`（跨工作流复用的用户 `func`；v1 简单全量加载、全局可用）。
- **行为节点模板**：`.steer/lib/templates/<node>.j2.md`（一个文件 = 一个行为节点定义，含 task/ask/command/collect/print 及用户自定义；Jinja2 + Markdown，分 `formatter`+`body`，可编辑/覆盖/扩展）。
- **实例 / run（v1）**：`steer instance start <wf> <name>` 建 `.steer/instances/<name>/`，**若已存在则清空重建**（v1 不保留历史、不选 run）：
  - `context.json` — 执行上下文：`pc`、`call_stack`、`vars`、以及 **`steps`（每个 step 是一个 JSON object，含 `checked`/`value`/`attempts` 等子属性）**；
  - `audit.jsonl` — 每步审计（step / check / 结果）。
  - （后续版本再加 run-id 多版本：`runs/<run-id>/` + `current`，保留历史。）
- instance 相关命令统一为 **`steer instance <action> <name> ...`**（`start` 额外带 `<workflow>`；未来 `list`/`clean`/`resume`）。

## 与 Atomic / OpenSpec / Make 的边界

- **vs Atomic**：Atomic 绑自己的 UI + agent runtime；steer 不拥有 runtime/UI，agent 自己是 runtime，steer 连文件系统都不碰。
- **vs OpenSpec**：OpenSpec 是固定 artifact pipeline，无循环 / 无 check-fail 修复；steer 是用户可编程的通用流程，有循环 / 分支 / check / 重试 / 函数。
- **vs Make / CI**：Make/CI 的 task body 是 shell（自己跑）；steer 的 task body 是自然语言 instruction，执行者是 agent，steer 只管控制与判决。

## 范围护栏（保持轻量 + 纯粹）

- 语言极小：只保留必要语句与内置；不演进成通用编程语言。
- **steer 不碰外部世界**：不跑 shell、不读写 FS、不 spawn、不碰终端；只在自己上下文里跑控制 + 算术/比较 + 模板渲染。所有与世界的交互归 agent，经 `steer set`/`error` 回流（由模板自动注入，作者不手写）。
- 不自带 agent runtime、不 spawn agent（`spawn` 参数是人指令改写；人机交互让 agent 用自己的工具）。
- check 单一形式（agent + 模板注入 `steer set verified`）；确定性直求不做。
- 不做 `file_exists`/`read_file`/`contains`/`split` 等 steer 内置；用 task+return+check / command 表达。
- 不做通用 stdlib（`len`/`range` 等）；`for` 只迭代 list；字符串拼接靠 `{var}` 插值。
- 不单独设 `human` 内置；人工 gate 用 `ask`。
- 不做多 agent fan-out、不做 GUI/TUI、不做跨工作流调度。

## 开放问题

1. ~~控制构造执行归属~~ → **已定**：控制语句由 steer 执行；只有 agent op 渲染成 instruction。循环内 agent op 每次迭代独立 check。
2. ~~agent op 输出绑定~~ → **已定**：取值型 task 经模板注入的 `steer set` 回流；`func return` 带回路径；steer 不解析 agent 输出。
3. ~~宏 vs 子例程~~ → **已定**：真子例程（`func`/`return` + 微型调用栈）。
4. ~~`print`/`input`~~ → **已定**：`print`=task 无 return 无 check；`input`→`ask`。
5. ~~verifier 形式~~ → **已定**：`check` 参数（指令字符串）+ 模板注入 `steer set verified`，`steer check` 读标志。单一形式。
6. ~~格式~~ → **已定**：小型 DSL，显式 `end`。
7. ~~行动统一~~ → **已定**：函数调用；`task` 是唯一原语。
8. ~~括号 / spawn~~ → **已定**：统一带括号；`spawn` 命名参数。
9. ~~纯工具函数~~ → **已定**：无通用 stdlib；`file_exists` 等用 task+return+check 表达。
10. ~~取值原语层级~~ → **已定**：`task`→`collect`→`ask`/`command`（+ `print`）。
11. ~~`collect` 命名~~ → **已定**：保留。
12. ~~"声明式"含义~~ → **已定**：steer 本质，只声明任务/步骤。
13. ~~CLI 结构~~ → **已定**：subcommand 分组 `steer <resource> <action> <args>`；资源 = `workflow`（`validate`/`simulate`）/ `instance`（`start`/`status`/`step`/`check`/`set`/`error`，未来 `list`/`clean`/`resume`）。推进在 `check`；按节点分派。曾考虑 flat，选 subcommand（命名空间清晰、可扩展；agent 热路径多一个 `instance` 词，成本可忽略）。
14. ~~值结构 / 上下文 / command 返回 / 目录 / 重跑~~ → **已定**：上下文 JSON；`steer set` 类型化字面量；`command` 值由 `return` 决定；目录 `.steer/workflows`、`.steer/lib`（含 `templates/*.j2.md`）、`.steer/instances/<name>/{context.json,audit.jsonl}`（**v1：start 清空重建、不保留历史**；run-id 多版本留待后续）。每个 step 是 JSON object（含 `checked` 子属性）。
15. ~~task 参数~~ → **已定**：`instruction`/`return`/`check`/`produce`/`spawn`；`return` 在赋值/返回时必填；指令模板自动注入回流动作（作者不手写 `steer set`）；`verify`→`check`、`result`→`return`、`produce`=list、`for`=list。
16. ~~模板文案 / run-id / ask 映射 / lib 导入 / 模板组合~~ → **已定**：
    - (a) 模板文案与拼装**由人写模板决定**，steer 只把参数传进 Jinja2 渲染。run-id = start **时间戳**。
    - (b) `ask` 经 AskUserQuestion 由 ask 模板处理（指令 agent 先 AskUserQuestion 提问、再按 `return` 格式 `steer set` 答案），无特殊 steer 映射。
    - (d) lib 走**显式 `import`**；builtin（task/ask/command/collect/print）**自动可用**，无需 import。
    - **模板组合**：用 Jinja2 `extends`/`block`（ask/command/collect/print extends task）；**`formatter` 保持纯参数定义（可带 declarative `default`），不放执行/赋值逻辑**；糖级参数变换放 steer 渲染器、不污染 schema。
17. ~~`verified` 命名~~ → **已定**：改名 `checked`，作为**当前 step 的子属性**（每个 step 是 JSON object）。**行为节点 = `.j2.md` 模板文件**（一文件一节点，含自定义 `foo.j2.md`）。
18. ~~import / 跨工作流 func~~ → **v1 不做**：工作流自包含（`func` 内联定义）；builtin（task/ask/command/collect/print）与自定义节点（`.j2.md` 模板）自动可用；`import` 与跨工作流共享留待后续版本。

19. **循环语义收尾**（全部已定，已实现）：
    - (a) **已定**：`repeat`→`loop` 纯改名；`for` 保持独立关键字；`if`/`until` 条件 = steer 侧单比较/布尔表达式，agent 交互放 body（PC/ALU 分工）。
    - (b) **已定**：加 `and`/`or`/`not`（纯表达式层）；call 不进条件（`UnexpectedCall` 承重墙）。
    - (c) **已定**：新增 `judge` 节点——默认模板 `judge.j2.md`，固有返回 bool、无 `return` 参数、无语法改动；agent 布尔判断进变量、可作条件。
    - (d) **已定**：有界重试不设专门形式，由 `judge` + 手动计数器 + `until <var> or i>=N` 表达。
    - (e) **已定**：移除定次形式 `loop N … end`（≈ 计数器 + `until i>=N`，原形式无下标、更弱）；`loop` 收为后测专用（必带 `until`）。副作用：count 类型 footgun（非整数静默 0 次）随之消灭。
    - (f) **已定**：`check` 只管 step 级无上限重试，与 `judge`/循环正交。节点名定 `judge`（否决 `condition`=名词、`expect`=断言色彩；`collect(return=bool)` 样板重被取代）。
    - (g) **已定**：不实现 `while` 关键字——`loop … until <谓词>`（+ `and/or/not`）是唯一条件循环，覆盖典型 while 用例（重试/轮询均 ≥1 次）。严格"先判断、可能 0 次"的预测试为 v1 已知限制（用 `if` 守卫 + `loop`，或接受一次良性迭代）。

20. ~~`if` 多分支（elseif）~~ → **已定，已实现**：支持 `if`/`elseif`/`else` 多分支。AST 用显式 `IfBranch { cond, body }` + `If { branches: Vec<IfBranch>, else_block: Option<Block> >`（`branches[0]` 是 `if`，其余是 `elseif`）——**显式建模链条**，不脱糖成嵌套 `If`。文法 `if := "if" expr sep block ("elseif" expr sep block)* ("else" sep block)? "end" sep`。关键字选 `elseif`（一词、独占一行，避开 `else if` 在换行语法里"同列=链 / 换行=嵌套"的歧义）。改了 parser（循环吃 `elseif`）+ ir（对 `branches` 循环降级，一串 `JumpIfFalse`→下一分支 + `Jump`→end）+ validate/simulate（遍历分支）；**vm 不动**（跑 IR，多分支只是一串跳转）。
21. ~~`loop … until` 语义复核~~ → **已定**：保留 `loop … until c`（后测、`c` 真退出，= Lua `repeat/until` = C `do…while(!c)`）；不改为 `do…while`。"重试到成功"用 `until passed` 最直接。

## 下一步

设计 + 存储布局已完整闭合（#1–#15 已定，#16 是写代码时顺手定的小事）。建议**冻结、开工**：定稿 smoke（上面的 bugfix 示例）存到 `.steer/workflows/smoke-bugfix.steer`，然后用 TaskCreate 切实现任务——lexer（行 / `//` / 字符串+`{var}` / `end`/`else`/`until`）→ parser（控制结构 + 函数调用 + 命名参数）→ AST → 树遍历解释器（PC + 调用栈 + `context.json` 读写 + `step`/`check` 分派 + **指令模板渲染**）→ `steer` CLI（subcommand：`workflow validate/simulate`、`instance start/status/step/check/set/error`）→ Claude Code skill（驱动 step→执行→check 循环）。
