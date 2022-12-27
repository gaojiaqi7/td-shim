// Copyright (c) 2020-2022 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent
use core::arch::global_asm;

global_asm!(include_str!("cet_test.asm"));

extern "win64" {
    pub fn cet_test();
}