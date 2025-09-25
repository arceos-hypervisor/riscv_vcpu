# consts模块

<cite>
**本文档中引用的文件**
- [consts.rs](file://src/consts.rs)
</cite>

## 目录
1. [引言](#引言)
2. [陷阱与中断常量分类](#陷阱与中断常量分类)
3. [SBI协议一致性保障机制](#sbi协议一致性保障机制)
4. [架构特定常量分析](#架构特定常量分析)
5. [具名常量对可维护性的提升](#具名常量对可维护性的提升)
6. [常量汇总表](#常量汇总表)
7. [扩展新SBI功能的命名规范](#扩展新sbi功能的命名规范)

## 引言
本模块系统定义了RISC-V虚拟CPU运行过程中涉及的关键常量，集中管理各类陷阱（trap）、中断（interrupt）和异常（exception）的数值编码。通过将底层硬件行为抽象为具名常量，提升了代码的可读性、可维护性和协议一致性。

**Section sources**
- [consts.rs](file://src/consts.rs#L1-L91)

## 陷阱与中断常量分类
`traps` 模块下按功能划分为中断、异常和IRQ三类常量：

### 中断常量
定义了RISC-V各特权级下的软件、定时器和外部中断位标志：
- 用户态软件中断：`USER_SOFT`
- 监督态软件中断：`SUPERVISOR_SOFT`
- 虚拟监督态软件中断：`VIRTUAL_SUPERVISOR_SOFT`
- 机器态软件中断：`MACHINE_SOFT`
- 用户态定时器中断：`USER_TIMER`
- 监督态定时器中断：`SUPERVISOR_TIMER`
- 外部中断系列：`*_EXTERNAL`

这些常量采用左移位操作生成唯一的位掩码，确保在`sip`或`mie`等CSR寄存器中进行精确的位操作。

### 异常常量
涵盖指令、加载、存储访问错误及环境调用等同步异常类型：
- 指令地址未对齐：`INST_ADDR_MISALIGN`
- 非法指令：`ILLEGAL_INST`
- 断点异常：`BREAKPOINT`
- 各类页错误：`*_PAGE_FAULT`
- 环境调用（ECALL）来源区分：`ENV_CALL_FROM_*`

所有异常码均为单一位标志，便于在异常处理流程中快速识别异常类型。

### IRQ常量
用于解析`scause`寄存器中的中断编码：
- `INTC_IRQ_BASE` 表示中断标识位（最高位）
- `S_SOFT`, `S_TIMER`, `S_EXT` 分别对应监督态软、时钟、外部中断编号
- `TIMER_IRQ_NUM` 明确指定计时器中断号

**Section sources**
- [consts.rs](file://src/consts.rs#L4-L80)

## SBI协议一致性保障机制
虽然当前文件未直接定义SBI扩展ID（如EID_HVC）或功能ID（FID_CONSOLE_WRITE），但其设计模式为SBI接口提供了基础支持。通过统一使用具名常量而非硬编码数值，确保vCPU在触发环境调用（如`ENV_CALL_FROM_HS`）时能正确传递标准定义的异常码，从而与SBI实现保持语义一致。

此外，中断和异常的标准化定义使得SBI服务能够准确判断陷入原因，并据此执行相应的处理逻辑（如IPI分发、时间片调度等），保障了跨平台兼容性。

**Section sources**
- [consts.rs](file://src/consts.rs#L4-L80)

## 架构特定常量分析
本模块中的常量严格遵循RISC-V ISA规范：
- `INTC_IRQ_BASE = 1 << (usize::BITS - 1)` 对应`scause`寄存器的第63位（RV64）或第31位（RV32），即“中断”标志位
- 各中断源偏移值（+1, +5, +9）符合RISC-V特权架构文档中规定的编码
- 异常码布局与《RISC-V特权架构手册》完全一致

尽管未显式依赖`riscv-h`库，但其常量定义实质上复现了该库的核心语义，体现了对标准规范的遵循。

**Section sources**
- [consts.rs](file://src/consts.rs#L70-L80)

## 具名常量对可维护性的提升
使用具名常量替代魔法数字带来显著优势：
- **可读性增强**：`if cause == traps::irq::S_TIMER` 比 `if cause == 0x80000005` 更直观
- **可维护性提高**：一旦规范变更，只需修改常量定义一处即可全局生效
- **减少错误**：避免手动计算位掩码或记忆具体数值导致的拼写错误
- **编译期检查**：类型安全和作用域限制防止非法引用

这种抽象方式使代码更易于理解与调试，尤其在复杂中断处理路径中体现明显价值。

**Section sources**
- [consts.rs](file://src/consts.rs#L1-L91)

## 常量汇总表
| 类别 | 常量名称 | 数值表达式 | 用途说明 |
|------|--------|-----------|--------|
| 中断 | USER_SOFT | 1 << 0 | 用户态软件中断 |
| 中断 | SUPERVISOR_SOFT | 1 << 1 | 监督态软件中断 |
| 中断 | VIRTUAL_SUPERVISOR_SOFT | 1 << 2 | 虚拟监督态软件中断 |
| 中断 | MACHINE_SOFT | 1 << 3 | 机器态软件中断 |
| 中断 | USER_TIMER | 1 << 4 | 用户态定时器中断 |
| 中断 | SUPERVISOR_TIMER | 1 << 5 | 监督态定时器中断 |
| 中断 | VIRTUAL_SUPERVISOR_TIMER | 1 << 6 | 虚拟监督态定时器中断 |
| 中断 | MACHINE_TIMER | 1 << 7 | 机器态定时器中断 |
| 中断 | USER_EXTERNAL | 1 << 8 | 用户态外部中断 |
| 中断 | SUPERVISOR_EXTERNAL | 1 << 9 | 监督态外部中断 |
| 中断 | VIRTUAL_SUPERVISOR_EXTERNAL | 1 << 10 | 虚拟监督态外部中断 |
| 中断 | MACHINEL_EXTERNAL | 1 << 11 | 机器态外部中断 |
| 中断 | SUPERVISOR_GUEST_EXTERNEL | 1 << 12 | 监督态客户机外部中断 |
| 异常 | INST_ADDR_MISALIGN | 1 << 0 | 指令地址未对齐 |
| 异常 | INST_ACCESSS_FAULT | 1 << 1 | 指令访问错误 |
| 异常 | ILLEGAL_INST | 1 << 2 | 非法指令 |
| 异常 | BREAKPOINT | 1 << 3 | 断点异常 |
| 异常 | LOAD_ADDR_MISALIGNED | 1 << 4 | 加载地址未对齐 |
| 异常 | LOAD_ACCESS_FAULT | 1 << 5 | 加载访问错误 |
| 异常 | STORE_ADDR_MISALIGNED | 1 << 6 | 存储地址未对齐 |
| 异常 | STORE_ACCESS_FAULT | 1 << 7 | 存储访问错误 |
| 异常 | ENV_CALL_FROM_U_OR_VU | 1 << 8 | U/VU模式环境调用 |
| 异常 | ENV_CALL_FROM_HS | 1 << 9 | HS模式环境调用 |
| 异常 | ENV_CALL_FROM_VS | 1 << 10 | VS模式环境调用 |
| 异常 | ENV_CALL_FROM_M | 1 << 11 | M模式环境调用 |
| 异常 | INST_PAGE_FAULT | 1 << 12 | 指令页错误 |
| 异常 | LOAD_PAGE_FAULT | 1 << 13 | 加载页错误 |
| 异常 | STORE_PAGE_FAULT | 1 << 15 | 存储页错误 |
| 异常 | INST_GUEST_PAGE_FAULT | 1 << 20 | 指令客户页错误 |
| 异常 | LOAD_GUEST_PAGE_FAULT | 1 << 21 | 加载客户页错误 |
| 异常 | VIRTUAL_INST | 1 << 22 | 虚拟指令异常 |
| 异常 | STORE_GUEST_PAGE_FAULT | 1 << 23 | 存储客户页错误 |
| IRQ | INTC_IRQ_BASE | 1 << (usize::BITS - 1) | scause中断标志位 |
| IRQ | S_SOFT | INTC_IRQ_BASE + 1 | 监督软中断编号 |
| IRQ | S_TIMER | INTC_IRQ_BASE + 5 | 监督定时器中断编号 |
| IRQ | S_EXT | INTC_IRQ_BASE + 9 | 监督外部中断编号 |
| IRQ | MAX_IRQ_COUNT | 1024 | 最大IRQ数量 |
| IRQ | TIMER_IRQ_NUM | S_TIMER | 计时器中断号 |

**Section sources**
- [consts.rs](file://src/consts.rs#L4-L80)

## 扩展新SBI功能的命名规范
当新增SBI功能时，应遵循以下组织原则：
- **命名清晰**：使用全大写字母和下划线分隔，如`EID_MY_EXTENSION`
- **模块化组织**：在独立模块中定义相关常量，避免污染全局命名空间
- **数值唯一性**：确保SBI扩展ID和功能ID不与其他标准冲突
- **文档注释**：每个常量必须附带中文注释说明其用途和来源依据
- **一致性**：延续现有位运算风格（如`1 << n`）定义标志位

建议未来将SBI相关常量单独设立`sbi_consts`模块，以更好地区分底层陷阱与高层接口定义。

**Section sources**
- [consts.rs](file://src/consts.rs#L1-L91)