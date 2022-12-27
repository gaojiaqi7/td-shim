// Copyright (c) 2021 - 2023  Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

// Auto-generated by `td-layout-config`, do not edit manually.

/*
Image Layout
+----------------------------------------+ <- 0x0
|                 CONFIG                 |   (0x40000) 256 KB
+----------------------------------------+ <- 0x40000
|                MAILBOX                 |   (0x1000) 4 KB
+----------------------------------------+ <- 0x41000
|               TEMP_STACK               |   (0x20000) 128 KB
+----------------------------------------+ <- 0x61000
|               TEMP_HEAP                |   (0x20000) 128 KB
+----------------------------------------+ <- 0x81000
|                  FREE                  |   (0x1000) 4 KB
+----------------------------------------+ <- 0x82000
|            BUILTIN_PAYLOAD             |   (0xC2D000) 12.18 MB
+----------------------------------------+ <- 0xCAF000
|                METADATA                |   (0x1000) 4 KB
+----------------------------------------+ <- 0xCB0000
|               BOOTLOADER               |   (0x348000) 3.28 MB
+----------------------------------------+ <- 0xFF8000
|              RESET_VECTOR              |   (0x8000) 32 KB
+----------------------------------------+ <- 0x1000000
Image size: 0x1000000 (16 MB)
*/

// Image Layout Configuration

pub const TD_SHIM_CONFIG_OFFSET: u32 = 0x0;
pub const TD_SHIM_CONFIG_SIZE: u32 = 0x40000; // 256 KB

pub const TD_SHIM_MAILBOX_OFFSET: u32 = 0x40000;
pub const TD_SHIM_MAILBOX_SIZE: u32 = 0x1000; // 4 KB

pub const TD_SHIM_TEMP_STACK_OFFSET: u32 = 0x41000;
pub const TD_SHIM_TEMP_STACK_SIZE: u32 = 0x20000; // 128 KB

pub const TD_SHIM_TEMP_HEAP_OFFSET: u32 = 0x61000;
pub const TD_SHIM_TEMP_HEAP_SIZE: u32 = 0x20000; // 128 KB

pub const TD_SHIM_FREE_OFFSET: u32 = 0x81000;
pub const TD_SHIM_FREE_SIZE: u32 = 0x1000; // 4 KB

pub const TD_SHIM_BUILTIN_PAYLOAD_OFFSET: u32 = 0x82000;
pub const TD_SHIM_BUILTIN_PAYLOAD_SIZE: u32 = 0xC2D000; // 12.18 MB

pub const TD_SHIM_METADATA_OFFSET: u32 = 0xCAF000;
pub const TD_SHIM_METADATA_SIZE: u32 = 0x1000; // 4 KB

pub const TD_SHIM_BOOTLOADER_OFFSET: u32 = 0xCB0000;
pub const TD_SHIM_BOOTLOADER_SIZE: u32 = 0x348000; // 3.28 MB

pub const TD_SHIM_RESET_VECTOR_OFFSET: u32 = 0xFF8000;
pub const TD_SHIM_RESET_VECTOR_SIZE: u32 = 0x8000; // 32 KB

// Offset when Loading into Memory
pub const MEMORY_OFFSET: u32 = 0xFF000000;

// Base Address after Loaded into Memory
pub const TD_SHIM_CONFIG_BASE: u32 = 0xFF000000;
pub const TD_SHIM_MAILBOX_BASE: u32 = 0xFF040000;
pub const TD_SHIM_TEMP_STACK_BASE: u32 = 0xFF041000;
pub const TD_SHIM_TEMP_HEAP_BASE: u32 = 0xFF061000;
pub const TD_SHIM_FREE_BASE: u32 = 0xFF081000;
pub const TD_SHIM_BUILTIN_PAYLOAD_BASE: u32 = 0xFF082000;
pub const TD_SHIM_METADATA_BASE: u32 = 0xFFCAF000;
pub const TD_SHIM_BOOTLOADER_BASE: u32 = 0xFFCB0000;
pub const TD_SHIM_RESET_VECTOR_BASE: u32 = 0xFFFF8000;
