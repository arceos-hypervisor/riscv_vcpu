// Copyright 2025 The Axvisor Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Tests for register-related public types.

use riscv_vcpu::GprIndex;

// ==================== GprIndex Tests ====================

#[test]
fn test_gpr_index_values() {
    // Test that enum variants have correct numeric values
    assert_eq!(GprIndex::Zero as u32, 0);
    assert_eq!(GprIndex::RA as u32, 1);
    assert_eq!(GprIndex::SP as u32, 2);
    assert_eq!(GprIndex::GP as u32, 3);
    assert_eq!(GprIndex::TP as u32, 4);
    assert_eq!(GprIndex::T0 as u32, 5);
    assert_eq!(GprIndex::T1 as u32, 6);
    assert_eq!(GprIndex::T2 as u32, 7);
    assert_eq!(GprIndex::S0 as u32, 8);
    assert_eq!(GprIndex::S1 as u32, 9);
    assert_eq!(GprIndex::A0 as u32, 10);
    assert_eq!(GprIndex::A1 as u32, 11);
    assert_eq!(GprIndex::A2 as u32, 12);
    assert_eq!(GprIndex::A3 as u32, 13);
    assert_eq!(GprIndex::A4 as u32, 14);
    assert_eq!(GprIndex::A5 as u32, 15);
    assert_eq!(GprIndex::A6 as u32, 16);
    assert_eq!(GprIndex::A7 as u32, 17);
    assert_eq!(GprIndex::S2 as u32, 18);
    assert_eq!(GprIndex::S3 as u32, 19);
    assert_eq!(GprIndex::S4 as u32, 20);
    assert_eq!(GprIndex::S5 as u32, 21);
    assert_eq!(GprIndex::S6 as u32, 22);
    assert_eq!(GprIndex::S7 as u32, 23);
    assert_eq!(GprIndex::S8 as u32, 24);
    assert_eq!(GprIndex::S9 as u32, 25);
    assert_eq!(GprIndex::S10 as u32, 26);
    assert_eq!(GprIndex::S11 as u32, 27);
    assert_eq!(GprIndex::T3 as u32, 28);
    assert_eq!(GprIndex::T4 as u32, 29);
    assert_eq!(GprIndex::T5 as u32, 30);
    assert_eq!(GprIndex::T6 as u32, 31);
}

#[test]
fn test_gpr_index_from_raw_valid() {
    // Test converting raw values to GprIndex
    assert_eq!(GprIndex::from_raw(0), Some(GprIndex::Zero));
    assert_eq!(GprIndex::from_raw(1), Some(GprIndex::RA));
    assert_eq!(GprIndex::from_raw(2), Some(GprIndex::SP));
    assert_eq!(GprIndex::from_raw(3), Some(GprIndex::GP));
    assert_eq!(GprIndex::from_raw(4), Some(GprIndex::TP));
    assert_eq!(GprIndex::from_raw(10), Some(GprIndex::A0));
    assert_eq!(GprIndex::from_raw(11), Some(GprIndex::A1));
    assert_eq!(GprIndex::from_raw(15), Some(GprIndex::A5));
    assert_eq!(GprIndex::from_raw(31), Some(GprIndex::T6));
}

#[test]
fn test_gpr_index_from_raw_invalid() {
    // Test that invalid raw values return None
    assert_eq!(GprIndex::from_raw(32), None);
    assert_eq!(GprIndex::from_raw(33), None);
    assert_eq!(GprIndex::from_raw(100), None);
    assert_eq!(GprIndex::from_raw(u32::MAX), None);
}

#[test]
fn test_gpr_index_roundtrip() {
    // Test that converting to raw and back preserves the value
    for i in 0..32u32 {
        let gpr = GprIndex::from_raw(i).expect("Valid register index");
        assert_eq!(gpr as u32, i, "Roundtrip failed for index {}", i);
    }
}

#[test]
fn test_gpr_index_clone_copy() {
    // Test that GprIndex implements Clone and Copy
    let a0 = GprIndex::A0;
    let a0_copy = a0;
    let a0_clone = a0.clone();
    
    assert_eq!(a0 as u32, a0_copy as u32);
    assert_eq!(a0 as u32, a0_clone as u32);
    
    // Verify Copy semantics (a0 is still usable after move)
    assert_eq!(a0 as u32, 10);
}

#[test]
fn test_gpr_index_eq() {
    // Test equality comparison
    assert_eq!(GprIndex::A0, GprIndex::A0);
    assert_eq!(GprIndex::SP, GprIndex::SP);
    assert_ne!(GprIndex::A0, GprIndex::A1);
}

#[test]
fn test_argument_registers() {
    // A0-A7 are consecutive (10-17)
    assert_eq!(GprIndex::A0 as u32 + 0, 10);
    assert_eq!(GprIndex::A1 as u32 + 0, 11);
    assert_eq!(GprIndex::A2 as u32 + 0, 12);
    assert_eq!(GprIndex::A3 as u32 + 0, 13);
    assert_eq!(GprIndex::A4 as u32 + 0, 14);
    assert_eq!(GprIndex::A5 as u32 + 0, 15);
    assert_eq!(GprIndex::A6 as u32 + 0, 16);
    assert_eq!(GprIndex::A7 as u32 + 0, 17);
}

#[test]
fn test_temporary_registers() {
    // T0-T6 are in two groups: T0-T2 (5-7) and T3-T6 (28-31)
    assert_eq!(GprIndex::T0 as u32, 5);
    assert_eq!(GprIndex::T1 as u32, 6);
    assert_eq!(GprIndex::T2 as u32, 7);
    assert_eq!(GprIndex::T3 as u32, 28);
    assert_eq!(GprIndex::T4 as u32, 29);
    assert_eq!(GprIndex::T5 as u32, 30);
    assert_eq!(GprIndex::T6 as u32, 31);
}

#[test]
fn test_saved_registers() {
    // S0-S11 are in two groups: S0-S1 (8-9) and S2-S11 (18-27)
    assert_eq!(GprIndex::S0 as u32, 8);
    assert_eq!(GprIndex::S1 as u32, 9);
    assert_eq!(GprIndex::S2 as u32, 18);
    assert_eq!(GprIndex::S11 as u32, 27);
}

#[test]
fn test_special_registers() {
    // Zero, RA, SP, GP, TP
    assert_eq!(GprIndex::Zero as u32, 0);
    assert_eq!(GprIndex::RA as u32, 1);
    assert_eq!(GprIndex::SP as u32, 2);
    assert_eq!(GprIndex::GP as u32, 3);
    assert_eq!(GprIndex::TP as u32, 4);
}
