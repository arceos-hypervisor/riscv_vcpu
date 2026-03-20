<h1 align="center">riscv_vcpu</h1>

<p align="center">RISC-V 虚拟 CPU (vCPU) 虚拟化实现库</p>

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/riscv_vcpu.svg)](https://crates.io/crates/riscv_vcpu)
[![Docs.rs](https://docs.rs/riscv_vcpu/badge.svg)](https://docs.rs/riscv_vcpu)
[![Rust](https://img.shields.io/badge/edition-2024-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/arceos-hypervisor/riscv_vcpu/blob/main/LICENSE)

</div>

[English](README.md) | 中文

# 简介

RISC-V 虚拟 CPU (vCPU) 虚拟化实现库，专为 RISC-V 架构的 Hypervisor 提供核心 vCPU 结构和虚拟化相关接口支持。兼容 RISC-V Hypervisor 扩展 (RVH)，适用于嵌入式虚拟化和教学用途。

本库导出以下核心类型：

- **`RISCVVCpu`** — RISC-V 虚拟 CPU 实现，提供完整的虚拟化支持
- **`RISCVPerCpu`** — RISC-V Hypervisor 每 CPU 数据结构和管理
- **`GprIndex`** — RISC-V 通用寄存器索引枚举 (x0-x31)
- **`EID_HVC`** — Hypercall 扩展 ID 常量 (0x485643 = ASCII 编码的 "HVC")

支持 `#![no_std]`，可用于裸机和操作系统内核开发。

## 快速上手

### 环境要求

- Rust nightly 工具链
- Rust 组件: rust-src, clippy, rustfmt
- RISC-V 目标: riscv64gc-unknown-none-elf

```bash
# 安装 rustup（如未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 nightly 工具链及组件
rustup install nightly
rustup component add rust-src clippy rustfmt --toolchain nightly

# 添加 RISC-V 目标
rustup target add riscv64gc-unknown-none-elf --toolchain nightly
```

### 运行检查和测试

```bash
# 1. 克隆仓库
git clone https://github.com/arceos-hypervisor/riscv_vcpu.git
cd riscv_vcpu

# 2. 代码检查（格式检查 + clippy + 构建 + 文档生成）
./scripts/check.sh

# 3. 运行测试
# 运行全部测试（单元测试 + 集成测试）
./scripts/test.sh

# 仅运行单元测试
./scripts/test.sh unit

# 仅运行集成测试
./scripts/test.sh integration

# 列出所有可用的测试套件
./scripts/test.sh list

# 使用特定目标运行单元测试（需要 QEMU 或交叉编译器）
cargo test --target riscv64gc-unknown-linux-gnu
```

## 集成使用

### 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
riscv_vcpu = "0.3"
```

### 使用示例

```rust,ignore
use riscv_vcpu::{RISCVVCpu, RISCVVCpuCreateConfig, has_hardware_support, GprIndex};

fn main() {
    // 检查硬件虚拟化支持
    if has_hardware_support() {
        println!("RISC-V H-extension 已支持");
        
        // 创建 vCPU 配置
        let config = RISCVVCpuCreateConfig::default();
        
        // 访问寄存器索引
        let a0 = GprIndex::A0;
        println!("A0 寄存器索引: {}", a0 as u32);  // 10
        
        // 从原始值转换
        let reg = GprIndex::from_raw(10).unwrap();
        assert_eq!(reg, GprIndex::A0);
    }
}
```

### 文档

生成并查看 API 文档：

```bash
cargo doc --no-deps --open
```

在线文档：[docs.rs/riscv_vcpu](https://docs.rs/riscv_vcpu)

## 架构

```text
┌─────────────────────────────────────────┐
│           RISCVVCpu                     │
│  ┌─────────────────────────────────┐    │
│  │  VmCpuRegisters                 │    │
│  │  ├─ hyp_regs (Hypervisor 状态)  │    │
│  │  ├─ guest_regs (Guest 状态)     │    │
│  │  ├─ vs_csrs (VS 级 CSRs)        │    │
│  │  ├─ virtual_hs_csrs             │    │
│  │  └─ trap_csrs (Trap 状态)       │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │  RISCVPerCpu                    │    │
│  │  (每 CPU 数据与管理)            │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

## 相关项目

- [ArceOS](https://github.com/arceos-org/arceos) - 实验性模块化操作系统（或 Unikernel）
- [AxVisor](https://github.com/arceos-hypervisor/axvisor) - 类型 1 Hypervisor 实现
- [aarch64_sysreg](https://github.com/arceos-org/aarch64_sysreg) - AArch64 系统寄存器定义

# 贡献

1. Fork 仓库并创建分支
2. 运行本地检查：`./scripts/check.sh`
3. 运行本地测试：`./scripts/test.sh`
4. 提交 PR 并通过 CI 检查

# 协议

本项目采用 Apache License, Version 2.0 许可证。详见 [LICENSE](LICENSE) 文件。
