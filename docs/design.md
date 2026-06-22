# steer 设计文档

> steer 的设计原理。语法与 CLI 用法见 [`README.md`](../README.md);已实现行为见 [`specs/`](specs/index.md)。

## 概述

steer 是一个**面向 Agent 的动态工作流控制单元**。它解释一个由代码与自然语言写成的工作流程序,逐步推进,把每一步要做的事渲染成指令交给外部 agent 执行,再把 agent 回报的结果写回执行上下文。

steer 自己只做三件事:

1. 跑控制流——赋值、分支、循环、函数调用;
2. 渲染指令——把行动节点的参数套进模板,生成发给 agent 的完整指令;
3. 管理状态——程序计数、变量、调用栈、每步的验证结果,全部序列化到磁盘。

所有实际工作(读文件、跑命令、查内容、问人、验证)都由 agent 完成。steer 不跑 shell、不读写文件系统、不 spawn 进程、不碰终端。与外部世界的一切交互都渲染成指令交给 agent,结果经 `steer instance set` 回流。

工作流由人或 agent 编写,用来控制 agent 实现目标。

## 执行模型

agent(运行 steer skill)驱动执行循环,steer 是一个有状态的执行体:

```
step    →  取当前指令(不改状态)
execute →  agent 执行指令
set     →  agent 回报值或验证结果
check   →  steer 推进到下一步
```

```
steer workflow validate <wf>              # 静态校验
steer workflow simulate <wf>              # dry-run,打印每个节点渲染后的指令
steer instance start <wf> <name>          # 建实例
steer instance step <name>                # 取当前指令
steer instance check <name>               # 推进
steer instance set <name> <var> <value>   # 回报值/验证结果
steer instance error <name> "<reason>"    # 报致命失败
```

两个数据方向:steer → agent(`step` 给指令,`check` 给判决);agent → steer(`set` 回流值或标志,`error` 报致命错)。

执行状态(程序计数、调用栈、变量、每步状态)全部序列化到 `context.json`,因此实例可跨 CLI 调用恢复——未完成的实例继续 `step` / `check` 即续上。

## 语言模型

语法分两类:

1. **控制语句**(steer 自己执行):`=`(赋值)、`if` / `elseif` / `else` / `end`、`loop ... until cond`、`for x in list ... end`、`func` / `return` / `end`。
2. **函数调用**(交给 agent):内置节点或用户 `func`。

`for` 只迭代 list 元素。steer 不执行领域逻辑,只声明任务与步骤;无 `len` / `range` 等通用 stdlib,字符串拼接靠 `{var}` 插值。

### 行动节点与指令模板

`task` 是基本行动原语。参数:`instruction`(位置,必填)+ 命名 `return` / `check` / `produce`。`step` 用指令模板把参数渲染成发给 agent 的完整指令,自动拼入回流动作,作者不手写 `steer instance set`。

一个行为节点对应一个模板文件(`.steer/templates/<set>/<node>.j2.md`,Jinja2 + Markdown)。模板由 front-matter(参数定义,供 `validate` 校验)和 Jinja2 正文组成。用户加 `<foo>.j2.md` 即定义新行为节点 `foo(...)`。

| 参数 | 类型 | 必填条件 | 渲染内容 |
| --- | --- | --- | --- |
| `instruction` | string | 必填 | 指令主体 |
| `return` | string | 结果被赋值时必填 | 让 agent 按 `{return}` 格式回报值 |
| `check` | string | 可选 | 让 agent 验证后设 `checked` 标志 |
| `produce` | list | 可选 | 本步应产出的文件 |

`collect` / `ask` / `command` 是带 `return` 的取值型 task,区别在值的来源:`ask` 来自人,`command` 来自 shell,`collect` 来自 agent 推理。`print` 只有指令、无 `return`、无 `check`。`judge` 固有返回 bool、无 `return` 参数。

```
task("修复 bug")
task("分析根因,写到 artifacts/root.md", return="文件路径",
     produce=["artifacts/root.md"],
     check="确认含'根因'")
files = command("git diff --name-only", return="字符串数组,每行一个文件")
ok = ask("构建系统是?", return="单个字符串")
```

### check 验证

`check` 是 task 的一个参数(指令字符串)。agent 执行验证、用 `steer instance set checked` 设当前 step 的 `checked` 标志(由模板自动注入),`steer instance check` 读它判决:通过则前进;失败则记录原因、留原位重试。值型节点(`ask` / `command` / `collect`)不需要 `check`——`check` 查值是否已 set 即推进。

steer 自己只求值算术与比较运算符(`+ - * / > < == !=` 等),供 `if` / `until` 判断。不做 `file_exists` / `read_file` / `contains` / `split` 等内置——一律用 task + `return` + `check` 或 `command` 表达。

## 控制流

### 条件是 steer 侧谓词

`if cond` / `until cond` 里的 `cond` 是 steer 侧的表达式(比较 + `and` / `or` / `not`),对上下文里已有的变量求值,不触发 agent、不碰外部世界。当退出条件依赖外部世界(如"构建通过了吗"),把取值的行动节点放在循环体里,agent 把结果设回变量,`until` 再去比那个变量。条件里不嵌行动调用——行动调用必须是独立语句。

```
loop
    ok = judge("构建是否通过?")
until ok
```

`and` / `or` / `not` 在表达式层对布尔求值。行动调用不进条件——保证条件求值不触发 agent 交互。

`judge` 节点让 agent 做布尔判断、结果进变量,固有返回 bool、无 `return` 参数:`passed = judge("构建是否通过?")`。有界重试(最多 N 次、成功提前停)用 `judge` + 手动计数器 + `or` 表达:

```
i = 0
loop
    i = i + 1
    passed = judge("构建是否通过?")
until passed or i >= 3
```

`judge` 与 `check` 正交:`judge` 判断一次、布尔进变量、无重试;`task(check=...)` 是断言加 step 级无上限重试。要"判断结果进条件"用 `judge`;要"做不好就重试到好"用 `check`。

### 循环

| 形式 | 语义 |
| --- | --- |
| `loop … until cond` | 后测:先跑 body 再测 `cond`,假则回边再跑,真则退出,至少 1 次。定次用计数器 + `until i >= N`。 |
| `for x in list … end` | 遍历:`list` 在入口求值一次,每轮弹出首元素绑给 `x`,空则退出,原变量不被消耗。 |

不实现 `while` 预测试关键字。严格"先判断、可能 0 次"的预测试用 `if` 守卫加 `loop` 表达。

## 存储布局

- **工作流脚本**:`.steer/workflows/<name>.steer`。
- **行为节点模板**:`.steer/templates/<set>/<node>.j2.md`(`default/` 为内置默认,`@template` 切换其他集合)。
- **实例**:`steer instance start <wf> <name>` 建 `.steer/instances/<name>/`,若已存在则清空重建(v1 不保留历史):
  - `context.json` — 执行上下文:程序计数、调用栈、`vars`、`steps`(每步含 `checked` / `value` / `attempts` 等)、`meta`(`@template` / `@context`);
  - `audit.jsonl` — 每步审计;
  - `workflow.steer` — 启动时拷贝的工作流源码。
