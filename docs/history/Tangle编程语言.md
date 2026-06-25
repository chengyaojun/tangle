## Tangle编程语言

文档即程序，文件即模块

#### 文件即模块 (File as Module)
1. 每个 .md 文件自动成为一个独立的命名空间/模块。
2. 文件名即模块名。例如 math_utils.md 在其他文件中通过 [Math Utils](./math_utils.md) 或类似语法导入。

#### 标题即作用域与函数 (Headings as Scopes/Functions)
1. 一级标题 (#)：定义程序名或模块主入口。
2. 二级标题 (##)：定义一个主功能块或结构体（Object），配合定义列表来表示一个纯粹的数据结构（Struct）。
3. 三级/四级标题 (###)：定义具体的函数（Function）或方法。标题文本就是函数名，标题下的内容是函数的上下文。


#### 代码块即执行体 (Code Blocks as Execution body)
１.　只有被特定标记的代码块（如 python ` 或自定义的 tangle `）才会被执行。
２.　代码块内部的变量、逻辑流遵循该代码块的逻辑。


#### 列表与引用即参数与配置 (Lists/Quotes as Configs)
1. 无序列表 (* 或 -)：定义函数的输入参数或配置项。
2. 引用块 (>)：用于定义断言（Assertions）、测试用例或返回值说明


#### 方法
1. 方法 (Method) $\rightarrow$ 用“接收者标题”绑定行为
> 语法模式：### 结构体名 -> 方法名
```
### User -> 发送通知 (send_notification)

为指定用户发送系统通知。

* `message`: 通知内容 (String)

```@tangle
// 这里的 `this` 自动绑定为 User 实例
if (this.is_active) {
    email_service.send(this.email, message);
}
```
2. 接口 (Interface) $\rightarrow$ 用“无实现标题”定义契约
```markdown
## Notifyable (接口)

任何可以接收通知的对象都必须实现此契约。

### 声明 -> 发送通知 (send_notification)
* `message`: 通知内容 (String)
> 注：此函数不需要执行体代码块，仅做参数与命名约束。
```



#### 指令集
1. 位置严格受限：指令只能出现在标题行的正下方，或者代码块的正上方。不允许夹杂在普通段落文本中。
2. 常用 directive 集
  - @export: 将当前标题（函数或结构体）标记为公开。只有被 @Export 修饰的函数，其他 .md 文件在引入该模块时才能调用
  - @eeprecated("原因"): 标记当前函数已弃用。当其他模块调用它时，编译器会在编译期或运行期抛出警告，并在最终生成的 HTML 文档中将该标题自动添加删除线。
  - @test(input=..., expect=...): 定义单条测试用例。编译器在运行 @tangle test 命令时，会自动提取这些指令并执行断言。
  - @hideCode: 在将 Markdown 渲染为面向读者的文档 HTML 时，隐藏下方的 ````@tangle ` 代码块，只保留标题和参数描述。适合面向产品或业务人员的文档。
  - @entry： 程序总入口（Main Entrance）。一个项目或一个模块目录中，只能有一个 @entry 指令。它通常挂载在一级标题（项目入口文件）或特定的入口函数上。被 @Entry 标记的函数会隐式获得 @Export 属性，并且由运行时系统自动调用。
  - @version：放在文件的**最顶部（Front Matter 区域）**，作为文件级元数据
    ```markdown
      ---
      title: 用户中心模块
      version: 1.2.4
      author: Alex
      ---
      
      # 用户中心
    ```
  - @rule.table：通过给表格加上指令修饰（如 @rule.table），让编译器将表格解析为业务规则/策略配置
  - @rule.tree:  修饰一个多级嵌套列表。默认行为：同级列表为“且（AND）”关系，缩进子列表为“或（OR）”关系（或通过关键字显式指定）。
  - @rule.toggle: 将复选框的状态直接映射为运行时的布尔配置
  - @rule.flow: Mermaid 图表即工作流引擎 (Graph as Workflow Engine)，

  #### 混合执行体方案

  通过统一IR（Intermediate Representation，中间表示）实现如下混合执行体方案，统一IR基于基于 JSON的控制流图，具体架构如：
  ```
  Markdown Layer
      ↓
  DSL Layer
      ↓
  Unified IR (Rule Graph)
      ↓
  Execution Engine
  ```
  Markdown Layer（表现层）:
   - 输入：原始的 .md 文本文件。
   - 职责：标准的 Markdown 解析器（如 Remark/Markdown-it）只负责把文本拆解成通用的 Markdown AST（如 Heading, Table, List, Link）。
   - 特点：这一层对“编程”一无所知，它只知道这里有一个表格，那里有一个三级标题。
  DSL Layer（领域特定语言映射层）
   - 输入：Markdown AST + 指令（@）。
   - 职责：这是你的核心语义解构层。 当解析器遇到特定的指令时，赋予底层 Markdown 元素以“生命”：遇到 @rule.table + Table $\rightarrow$ 激活决策表语义解析器。遇到 @rule.flow + Mermaid $\rightarrow$ 激活拓扑图语义解析器。遇到 ````tangle ` $\rightarrow$ 激活执行体代码块解析器。固定关键字this，并在参数列表中隐式地将 this: User 加载到当前作用域的符号表（Symbol Table）中。
   - 特点：将纯文本的表现形式，翻译成具有特定业务逻辑倾向的“高级领域模型”。
  Unified IR (Rule Graph)（统一中间表示层）
   - 输入：各种零散的 DSL 模型。
   - 职责：万流归宗。 无论上一层是表格、图还是列表，到了这一层，全部被揉碎并重新融合成一张统一的规则图（Rule Graph）。节点的本质就是 tangle 的原子代码块。 边的本质就是表格或列表转换来的布尔条件（Boolean Expressions）。文件中除了带有 @Export 或 @Entry 的二级/三级/四级标题，其余一律视为内部符号。在生成 Unified IR 时，它们会被打上 private: true 的 Tag，避免外部模块越界调用。
   - 特点：与前端表现形式解耦，与后端执行引擎解耦。 在这一层，你可以轻松实现代码优化、静态类型检查、循环依赖检测，以及最硬核的 Source Map（错误行号追踪标记）。
  Execution Engine（执行引擎/后端）
   - 输入：规范化的 Unified IR (Rule Graph)。
   - 职责：负责“让程序跑起来”。这里有两种实现策略，取决于你的应用场景：
    - 转译模式（JIT/AOT）：将 Rule Graph 直接生成为标准的 JavaScript、Python 或 Go 代码，然后调用现有的宿主引擎运行。
    - 自研 VM 模式（解释执行）：编写一个轻量的图执行引擎，直接在内存中像走迷宫一样根据条件触发节点。

  ###### 1. 核心逻辑/底层驱动：用 ````tangle ` 代码块
  ###### 2. 业务规则/策略配置：
    1. 表格指令修饰（如@rule.table）
    2. 多级嵌套列表指令修饰（如@rule.tree）
      ```markdown
        ### 信用卡审批流 (approve_credit_card)
        @rule.tree
        * `user`: 用户对象
        
        * 核心准入条件：
            * 收入门槛：`user.income >= 10000`
            * 信用良好：`user.credit_score > 700`
        * 风险对冲机制（满足其一即可）：
            * 资产证明：`user.has_house == true`
            * 担保人：`user.has_guarantor == true`
        * 结果：返回 `true`
      ```
    3. 任务列表/复选框即策略开关指令修饰(@rule.toggle)
      ```markdown
        ### 全局功能灰度配置 (get_features)
        @rule.toggle
        
        修改以下勾选状态可直接影响线上生产环境的功能表现：
        
        - [x] `enable_new_ui`: 启用 2026 全新 UI 界面
        - [ ] `enable_crypto_payment`: 开启加密货币支付通道
        - [x] `enable_ai_assistant`: 启用 AI 智能助手
      ```
    4. 图表即工作流引擎指令修饰(@rule.flow)
      ```markdown
      ### 订单状态机 (order_lifecycle)
      @rule.flow
      
      ```mermaid
      graph TD
          A[新订单: Created] -->|用户支付成功| B(待发货: Paid)
          A -->|24小时未支付| C(已取消: Cancelled)
          B -->|物流发货| D(已发货: Shipped)
          D -->|用户签收| E(已完成: Completed)
          D -->|拒收/退款| C
      ```
