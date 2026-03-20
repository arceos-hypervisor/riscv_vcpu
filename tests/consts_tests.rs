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

//! Tests for public constants exported from the crate.

use riscv_vcpu::EID_HVC;

#[test]
fn test_eid_hvc_constant() {
    // EID_HVC should be 0x485643 ("HVC" in ASCII)
    assert_eq!(EID_HVC, 0x485643);
}

#[test]
fn test_eid_hvc_ascii_encoding() {
    // Verify ASCII encoding: 'H' = 0x48, 'V' = 0x56, 'C' = 0x43
    let h = b'H' as usize;
    let v = b'V' as usize;
    let c = b'C' as usize;
    let expected = (h << 16) | (v << 8) | c;
    assert_eq!(EID_HVC, expected);
}
