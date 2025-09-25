# RISCVVCpuCreateConfig 配置结构体

<cite>
**Referenced Files in This Document**  
- [src/lib.rs](file://src/lib.rs)
- [src/vcpu.rs](file://src/vcpu.rs)
- [src/detect.rs](file://src/detect.rs)
</cite>

## Table of Contents
1. [RISCVVCpuCreateConfig 结构体字段详解](#riscvvcpucrateconfig-结构体字段详解)
2. [字段默认值设定逻辑](#字段默认值设定逻辑)
3. [vCPU 初始化过程中的作用机制](#vcpu-初始化过程中的作用机制)
4. [自定义配置代码示例](#自定义配置代码示例)
5. [与硬件检测的依赖关系](#与硬件检测的依赖关系)
6. [非法配置导致的初始化失败场景](#非法配置导致的初始化失败场景)

## RISCVVCpuCreateConfig 结构体字段详解

`RISCVVCpuCreateConfig` 是用于创建 RISC-V 虚拟 CPU 的核心配置结构体，定义于 `lib.rs` 文件中。该结构体包含两个关键配置项：

- **hart_id**: 表示虚拟 CPU 的硬件线程 ID（Hardware Thread ID），类型为 `usize`，在 vCPU 初始化时被写入 `a0` 寄存器。
- **dtb_addr**: 表示设备树二进制文件（Device Tree Blob）的物理地址，类型为 `axaddrspace::GuestPhysAddr`，在初始化时被写入 `a1` 寄存器。

这些字段直接映射到 RISC-V SBI（Supervisor Binary Interface）规范中规定的启动参数传递约定，确保客户机操作系统能够正确识别其运行环境。

**Section sources**
- [src/lib.rs](file://src/lib.rs#L38-L46)

## 字段默认值设定逻辑

`RISCVVCpuCreateConfig` 实现了 `Default` trait，提供了一套合理的默认配置：

- **hart_id 默认值**: 设定为 `0`，表示主虚拟 CPU 核心。这是多核系统中引导处理器（Bootstrap Processor）的标准标识符。
- **dtb_addr 默认值**: 设定为 `0x9000_0000`，这是一个在许多 RISC-V 系统中预留用于设备树的典型内存地址，避免与内核镜像和 RAM 区域冲突。

此默认配置旨在支持最常见的单核客户机启动场景，开发者可通过显式构造实例来覆盖这些值以适应特定需求。

**Section sources**
- [src/lib.rs](file://src/lib.rs#L47-L52)

## vCPU 初始化过程中的作用机制

在 `RISCVVCpu::new` 方法中，`RISCVVCpuCreateConfig` 的字段被用于初始化虚拟 CPU 的寄存器状态：

- **hart_id 的作用**: 该值被直接写入 `a0` 寄存器（GPR index 10），遵循 RISC-V Linux 内核启动协议，使客户机操作系统能获知当前正在执行的 hart ID，从而进行正确的 CPU 核心绑定和初始化流程。
- **dtb_addr 的作用**: 该值被转换为 `usize` 后写入 `a1` 寄存器（GPR index 11），指向客户机内存中设备树的位置。客户机操作系统通过此指针加载硬件描述信息，完成外设发现和驱动初始化。

这一机制严格遵循 RISC-V 架构的调用约定，确保虚拟化环境与客户机操作系统之间的无缝对接。

**Section sources**
- [src/vcpu.rs](file://src/vcpu.rs#L46-L58)

## 自定义配置代码示例

以下代码展示了如何创建自定义的 `RISCVVCpuCreateConfig` 实例以适配不同客户机环境：

```rust
use riscv_vcpu::{RISCVVCpuCreateConfig, GuestPhysAddr};

// 创建一个针对第二核心（hart 1）且使用非标准 DTB 地址的配置
let custom_config = RISCVVCpuCreateConfig {
    hart_id: 1,
    dtb_addr: GuestPhysAddr::from_usize(0xa000_0000),
};
```

此示例将 `hart_id` 设置为 `1`，适用于启动辅助核心；同时将 `dtb_addr` 移至 `0xa000_0000`，可用于避开特定内存区域或满足特殊固件要求。

**Section sources**
- [src/lib.rs](file://src/lib.rs#L38-L46)
- [src/vcpu.rs](file://src/vcpu.rs#L46-L58)

## 与硬件检测的依赖关系

虽然 `RISCVVCpuCreateConfig` 本身不直接依赖硬件检测结果，但整个 vCPU 子系统的启用受 `has_hardware_support` 函数控制。该函数位于 `detect.rs` 模块中，通过陷阱捕获机制探测 Hypervisor 扩展是否存在。

若 `has_hardware_support()` 返回 `false`，表明底层硬件不支持虚拟化，则即使提供了合法的 `RISCVVCpuCreateConfig`，vCPU 的 `hardware_enable` 过程也会失败。因此，有效的配置必须建立在硬件支持的基础之上。

**Section sources**
- [src/detect.rs](file://src/detect.rs#L1-L238)
- [src/percpu.rs](file://src/percpu.rs#L50-L52)

## 非法配置导致的初始化失败场景

尽管 `RISCVVCpuCreateConfig` 的字段本身不会直接导致初始化失败（因其均为基本类型且无内部不变量），但不当的配置组合可能引发后续问题：

- **hart_id 冲突**: 若多个 vCPU 实例被赋予相同的 `hart_id`，客户机操作系统可能无法区分核心，导致调度混乱或启动失败。
- **dtb_addr 无效地址**: 若 `dtb_addr` 指向未映射或受保护的内存区域，客户机在尝试读取设备树时将触发页错误，可能导致内核崩溃。

因此，确保 `hart_id` 的唯一性和 `dtb_addr` 的有效性是成功初始化的关键前提。

**Section sources**
- [src/vcpu.rs](file://src/vcpu.rs#L46-L58)
- [src/lib.rs](file://src/lib.rs#L38-L46)