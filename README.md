<h1 align="center">riscv_vcpu</h1>

<p align="center">RISC-V Virtual CPU (vCPU) Implementation for Hypervisors</p>

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/riscv_vcpu.svg)](https://crates.io/crates/riscv_vcpu)
[![Docs.rs](https://docs.rs/riscv_vcpu/badge.svg)](https://docs.rs/riscv_vcpu)
[![Rust](https://img.shields.io/badge/edition-2024-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/arceos-hypervisor/riscv_vcpu/blob/main/LICENSE)

</div>

English | [中文](README_CN.md)

# Introduction

A library providing RISC-V Virtual CPU (vCPU) implementation for hypervisors. This crate provides the core vCPU structure and virtualization-related interface support specifically designed for the RISC-V architecture. Compliant with the RISC-V Hypervisor Extension (RVH), designed for embedded hypervisors and educational use.

This library exports the following core types:

- **`RISCVVCpu`** — RISC-V virtual CPU implementation with full virtualization support
- **`RISCVPerCpu`** — Per-CPU data structure and management for RISC-V hypervisors
- **`GprIndex`** — RISC-V general-purpose register index enumeration (x0-x31)
- **`EID_HVC`** — Hypercall extension ID constant (0x485643 = "HVC" in ASCII)

Supports `#![no_std]` for bare-metal and OS kernel development.

## Quick Start

### Requirements

- Rust nightly toolchain
- Rust components: rust-src, clippy, rustfmt
- RISC-V target: riscv64gc-unknown-none-elf

```bash
# Install rustup (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain and components
rustup install nightly
rustup component add rust-src clippy rustfmt --toolchain nightly

# Add RISC-V target
rustup target add riscv64gc-unknown-none-elf --toolchain nightly
```

### Run Check and Test

```bash
# 1. Clone the repository
git clone https://github.com/arceos-hypervisor/riscv_vcpu.git
cd riscv_vcpu

# 2. Code check (format + clippy + build + doc generation)
./scripts/check.sh

# 3. Run tests
# Run all tests (unit tests + integration tests)
./scripts/test.sh

# Run unit tests only
./scripts/test.sh unit

# Run integration tests only
./scripts/test.sh integration

# List all available test suites
./scripts/test.sh list

# Run unit tests with specific target (requires QEMU or cross-compiler)
cargo test --target riscv64gc-unknown-linux-gnu --tests  

## Integration

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
riscv_vcpu = "0.3"
```

### Example

```rust
use riscv_vcpu::{RISCVVCpuCreateConfig, has_hardware_support, GprIndex};

// Check if hardware virtualization is supported
if has_hardware_support() {
    println!("RISC-V H-extension is supported");
    
    // Create vCPU configuration
    let config = RISCVVCpuCreateConfig::default();
    
    // Access register indices
    let a0 = GprIndex::A0;
    println!("A0 register index: {}", a0 as u32);  // 10
    
    // Convert from raw value
    let reg = GprIndex::from_raw(10).unwrap();
    assert_eq!(reg, GprIndex::A0);
}
```

### Documentation

Generate and view API documentation:

```bash
cargo doc --no-deps --open
```

Online documentation: [docs.rs/riscv_vcpu](https://docs.rs/riscv_vcpu)

## Architecture

```text
┌─────────────────────────────────────────┐
│           RISCVVCpu                     │
│  ┌─────────────────────────────────┐    │
│  │  VmCpuRegisters                 │    │
│  │  ├─ hyp_regs (Hypervisor state) │    │
│  │  ├─ guest_regs (Guest state)    │    │
│  │  ├─ vs_csrs (VS-level CSRs)     │    │
│  │  ├─ virtual_hs_csrs             │    │
│  │  └─ trap_csrs (Trap state)      │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │  RISCVPerCpu                    │    │
│  │  (Per-CPU data & management)    │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

## Related Projects

- [ArceOS](https://github.com/arceos-org/arceos) - An experimental modular OS (or Unikernel)
- [AxVisor](https://github.com/arceos-hypervisor/axvisor) - Type 1 hypervisor implementation
- [aarch64_sysreg](https://github.com/arceos-org/aarch64_sysreg) - AArch64 system register definitions

# Contributing

1. Fork the repository and create a branch
2. Run local check: `./scripts/check.sh`
3. Run local tests: `./scripts/test.sh`
4. Submit PR and pass CI checks

# License

Licensed under the Apache License, Version 2.0. See \[LICENSE\](LICENSE) for details.
