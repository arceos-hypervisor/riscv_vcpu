<cite>
**本文档中引用的文件**
- [lib.rs](file://src/lib.rs)
- [vcpu.rs](file://src/vcpu.rs)
- [detect.rs](file://src/detect.rs)
- [regs.rs](file://src/regs.rs)
</cite>

# API参考

## 目录
1. [简介](#简介)
2. [核心结构体](#核心结构体)
3. [全局函数](#全局函数)
4. [寄存器操作](#寄存器操作)
5. [虚拟CPU运行机制](#虚拟cpu运行机制)

## 简介

`riscv_vcpu`库为RISC-V架构的虚拟化环境提供了核心虚拟CPU（vCPU）抽象层。该库专为嵌入式hypervisor和教育用途设计，支持在`no_std`环境中运行，并符合RISC-V Hypervisor扩展（RVH）规范。

本API文档详细介绍了库中的公共接口，包括`RISCVVCpu`虚拟CPU结构体、`RISCVVCpuCreateConfig`配置结构体以及硬件支持检测函数`has_hardware_support`。文档涵盖了所有导出项的方法签名、参数类型、返回值、错误条件及使用约束，便于开发者直接查阅调用方式。

**Section sources**
- [README.md](file://README.md#L0-L59)

## 核心结构体

### RISCVVCpuCreateConfig 配置结构体

`RISCVVCpuCreateConfig`结构体用于创建新的`RISCVVCpu`实例时的配置选项。它包含以下字段：

- **hart_id**: vCPU的ID，默认值为`0`。此值将作为启动参数传递给客户机操作系统。
- **dtb_addr**: 设备树二进制文件（Device Tree Blob）的物理地址，默认值为`0x9000_0000`。此地址将在客户机启动时通过a1寄存器传递。

该结构体实现了`Default` trait，允许通过`RISCVVCpuCreateConfig::default()`创建具有默认配置的实例。

```rust,ignore
let config = RISCVVCpuCreateConfig {
    hart_id: 1,
    dtb_addr: axaddrspace::GuestPhysAddr::from_usize(0x8000_0000),
};
```

**Section sources**
- [lib.rs](file://src/lib.rs#L37-L56)

### RISCVVCpu 虚拟CPU结构体

`RISCVVCpu`是库的核心结构体，代表一个RISC-V架构的虚拟CPU实例。它实现了`AxArchVCpu` trait，提供了完整的vCPU生命周期管理功能。

#### 创建与初始化

通过`new`方法创建一个新的vCPU实例：

```rust
fn new(
    _vm_id: usize, 
    _vcpu_id: usize, 
    config: Self::CreateConfig
) -> AxResult<Self>
```

该方法接收VM ID、vCPU ID和配置结构体作为参数，根据配置初始化通用寄存器（如将`hart_id`设置到a0寄存器，将`dtb_addr`设置到a1寄存器）。

#### 运行配置

`setup`方法用于设置vCPU的运行时环境：

```rust
fn setup(&mut self, _config: Self::SetupConfig) -> AxResult
```

该方法会配置`sstatus`和`hstatus`寄存器，确保vCPU以正确的权限级别运行。

#### 入口点设置

`set_entry`方法用于设置客户机代码的入口地址：

```rust
fn set_entry(&mut self, entry: GuestPhysAddr) -> AxResult
```

**Section sources**
- [vcpu.rs](file://src/vcpu.rs#L50-L100)

## 全局函数

### has_hardware_support 硬件支持检测

`has_hardware_support`是一个全局函数，用于检测当前硬件是否支持RISC-V虚拟化扩展（H Extension）。

```rust
pub use detect::detect_h_extension as has_hardware_support;
```

该函数通过尝试读取`hgatp`控制状态寄存器来检测虚拟化扩展的存在。如果读取操作成功，则返回`true`；如果触发非法指令异常，则返回`false`。

使用示例如下：

```rust
if has_hardware_support() {
    // 硬件虚拟化支持可用，可以创建vCPU
    let config = RISCVVCpuCreateConfig::default();
    let vcpu = RISCVVCpu::new(config)?;
    vcpu.run()?;
} else {
    // 硬件不支持虚拟化
    panic!("Hardware virtualization not supported");
}
```

**Section sources**
- [lib.rs](file://src/lib.rs#L24)
- [detect.rs](file://src/detect.rs#L14-L237)

## 寄存器操作

### 通用寄存器访问

`RISCVVCpu`提供了多种方法来访问和操作通用寄存器（GPR）。

#### get_gpr 方法

获取指定索引的通用寄存器值：

```rust
pub fn get_gpr(&self, index: GprIndex) -> usize
```

参数`index`为`GprIndex`枚举类型，表示寄存器索引。

#### set_gpr_from_gpr_index 方法

设置指定索引的通用寄存器值：

```rust
pub fn set_gpr_from_gpr_index(&mut self, index: GprIndex, val: usize)
```

此方法的操作限制：
- 不能修改`Zero`寄存器（x0），因为其值始终为0
- 索引必须是有效的`GprIndex`枚举值

#### set_gpr 方法

基于数组索引设置通用寄存器：

```rust
fn set_gpr(&mut self, index: usize, val: usize)
```

此方法仅支持索引0-7，对应于a0-a7寄存器。对于其他寄存器，会输出警告信息。

### GprIndex 枚举

`GprIndex`枚举定义了RISC-V架构32个通用寄存器的索引，包括：
- `Zero`: x0寄存器
- `RA`: x1寄存器（返回地址）
- `SP`: x2寄存器（栈指针）
- `GP`: x3寄存器（全局指针）
- `TP`: x4寄存器（线程指针）
- `A0-A7`: x10-x17寄存器（函数参数/返回值）

**Section sources**
- [vcpu.rs](file://src/vcpu.rs#L250-L270)
- [regs.rs](file://src/regs.rs#L12-L115)

## 虚拟CPU运行机制

### run 方法

`run`方法是vCPU执行的核心，负责启动并运行客户机代码：

```rust
fn run(&mut self) -> AxResult<AxVCpuExitReason>
```

行为语义：
1. 清除S级中断使能，设置外部、软件和定时器中断
2. 调用底层汇编函数`_run_guest`进入客户机模式执行
3. 客户机退出后恢复中断状态
4. 调用`vmexit_handler`处理退出原因

该方法返回`AxVCpuExitReason`枚举，指示vCPU退出的原因，可能包括：
- `Nothing`: 正常中断退出
- `SystemDown`: 系统关闭请求
- `CpuUp`: CPU启动请求
- `Hypercall`: 超级调用
- `ExternalInterrupt`: 外部中断
- `NestedPageFault`: 嵌套页错误

### regs 访问器

`regs`方法提供对vCPU完整寄存器状态的访问：

```rust
pub fn regs(&mut self) -> &mut VmCpuRegisters
```

返回的`VmCpuRegisters`结构体包含了vCPU的所有寄存器状态，包括：
- `hyp_regs`: 超级监视线程（hypervisor）的CPU状态
- `guest_regs`: 客户机的CPU状态
- `vs_csrs`: VS级控制状态寄存器
- `virtual_hs_csrs`: 虚拟化的HS级控制状态寄存器
- `trap_csrs`: 陷阱相关的控制状态寄存器

此访问器允许高级用户直接操作底层寄存器状态，但需谨慎使用。

### advance_pc 方法

`advance_pc`方法用于手动推进客户机程序计数器：

```rust
pub fn advance_pc(&mut self, instr_len: usize)
```

此方法通常在处理系统调用或异常后调用，将`sepc`（异常程序计数器）增加指定的指令长度，实现从异常中恢复执行。

**Section sources**
- [vcpu.rs](file://src/vcpu.rs#L150-L250)