// Copyright (c) 2021 Intel Corporation
// Copyright (c) 2022 Alibaba Cloud
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

#[macro_use]
extern crate clap;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use serde::Deserialize;

const TD_LAYOUT_BUILD_TIME_RS_OUT: &str = "build_time.rs";
const TD_LAYOUT_RUNTIME_RS_OUT: &str = "runtime.rs";

// Equals to firmware size(16MiB) - metadata pointer offset(0x20) -
// OVMF GUID table size(0x28) - SEC Core information size(0xC).
const TD_SHIM_SEC_INFO_OFFSET: u32 = 0xFF_FFAC;

macro_rules! BUILD_TIME_TEMPLATE {
    () => {
"// Copyright (c) 2021 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

// Auto-generated by `td-layout-config`, do not edit manually.

/*
    Image Layout:
                  Binary                       Address
            {config_offset:#010X} -> +--------------+ <-  {config_base:#010X}
                          |     VAR      |
            {mailbox_offset:#010X} -> +--------------+ <-  {mailbox_base:#010X}
                          |  TD_MAILBOX  |
            {temp_stack_offset:#010X} -> +--------------+ <-  {temp_stack_base:#010X}
                          |    (Guard)   |
                          |  TEMP_STACK  |
            {temp_heap_offset:#010X} -> +--------------+ <-  {temp_heap_base:#010X}
                          |   TEMP_RAM   |
            {payload_offset:#010X} -> +--------------+ <-  {payload_base:#010X}
           ({payload_size:#010X})   | Rust Payload |
                          |     (pad)    |
            {metadata_offset:#010X} -> +--------------+
           ({metadata_size:#010X})   |   Metadata   |
            {ipl_offset:#010X} -> +--------------+ <-  {ipl_base:#010X}
           ({ipl_size:#010X})   |   Rust IPL   |
                          |     (pad)    |
            {rst_vec_offset:#010X} -> +--------------+ <-  {rst_vec_base:#010X}
           ({rst_vec_size:#010X})   | Reset Vector |
            {firmware_size:#010X} -> +--------------+ <- 0x100000000 (4G)
*/

// Image
pub const TD_SHIM_CONFIG_OFFSET: u32 = {config_offset:#X};
pub const TD_SHIM_CONFIG_SIZE: u32 = {config_size:#X};
pub const TD_SHIM_MAILBOX_OFFSET: u32 = {mailbox_offset:#X}; // TD_SHIM_CONFIG_OFFSET + TD_SHIM_CONFIG_SIZE
pub const TD_SHIM_MAILBOX_SIZE: u32 = {mailbox_size:#X};
pub const TD_SHIM_TEMP_STACK_GUARD_SIZE: u32 = {temp_stack_guard_size:#X};
pub const TD_SHIM_TEMP_STACK_OFFSET: u32 = {temp_stack_offset:#X}; // TD_SHIM_HOB_OFFSET + TD_SHIM_HOB_SIZE + TD_SHIM_TEMP_STACK_GUARD_SIZE
pub const TD_SHIM_TEMP_STACK_SIZE: u32 = {temp_stack_size:#X};
pub const TD_SHIM_TEMP_HEAP_OFFSET: u32 = {temp_heap_offset:#X}; // TD_SHIM_TEMP_STACK_OFFSET + TD_SHIM_TEMP_STACK_SIZE
pub const TD_SHIM_TEMP_HEAP_SIZE: u32 = {temp_heap_size:#X};

pub const TD_SHIM_PAYLOAD_OFFSET: u32 = {payload_offset:#X}; // TD_SHIM_TEMP_HEAP_OFFSET + TD_SHIM_TEMP_HEAP_SIZE
pub const TD_SHIM_PAYLOAD_SIZE: u32 = {payload_size:#X};
pub const TD_SHIM_METADATA_OFFSET: u32 = {metadata_offset:#X}; // TD_SHIM_PAYLOAD_OFFSET + TD_SHIM_PAYLOAD_SIZE
pub const TD_SHIM_METADATA_SIZE: u32 = {metadata_size:#X};
pub const TD_SHIM_IPL_OFFSET: u32 = {ipl_offset:#X}; // TD_SHIM_METADATA_OFFSET + TD_SHIM_METADATA_SIZE
pub const TD_SHIM_IPL_SIZE: u32 = {ipl_size:#X};
pub const TD_SHIM_RESET_VECTOR_OFFSET: u32 = {rst_vec_offset:#X}; // TD_SHIM_IPL_OFFSET + TD_SHIM_IPL_SIZE
pub const TD_SHIM_RESET_VECTOR_SIZE: u32 = {rst_vec_size:#X};
pub const TD_SHIM_FIRMWARE_SIZE: u32 = {firmware_size:#X}; // TD_SHIM_RESET_VECTOR_OFFSET + TD_SHIM_RESET_VECTOR_SIZE
pub const TD_SHIM_SEC_CORE_INFO_OFFSET: u32 = {sec_core_info_offset:#X}; // TD_SHIM_SEC_INFO_OFFSE

// Image loaded
pub const TD_SHIM_FIRMWARE_BASE: u32 = {firmware_base:#X}; // 0xFFFFFFFF - TD_SHIM_FIRMWARE_SIZE + 1
pub const TD_SHIM_CONFIG_BASE: u32 = {config_base:#X}; // TD_SHIM_FIRMWARE_BASE + TD_SHIM_CONFIG_OFFSET
pub const TD_SHIM_MAILBOX_BASE: u32 = {mailbox_base:#X}; // TD_SHIM_FIRMWARE_BASE + TD_SHIM_MAILBOX_OFFSET
pub const TD_SHIM_TEMP_STACK_BASE: u32 = {temp_stack_base:#X}; // TD_SHIM_FIRMWARE_BASE + TD_SHIM_TEMP_STACK_OFFSET
pub const TD_SHIM_TEMP_HEAP_BASE: u32 = {temp_heap_base:#X}; // TD_SHIM_FIRMWARE_BASE + TD_SHIM_TEMP_HEAP_OFFSET
pub const TD_SHIM_PAYLOAD_BASE: u32 = {payload_base:#X}; // TD_SHIM_FIRMWARE_BASE + TD_SHIM_PAYLOAD_OFFSET
pub const TD_SHIM_IPL_BASE: u32 = {ipl_base:#X}; // TD_SHIM_FIRMWARE_BASE + TD_SHIM_IPL_OFFSET
pub const TD_SHIM_RESET_VECTOR_BASE: u32 = {rst_vec_base:#X}; // TD_SHIM_FIRMWARE_BASE + TD_SHIM_RESET_VECTOR_OFFSET
pub const TD_SHIM_SEC_CORE_INFO_BASE: u32 = {sec_core_info_base:#X}; // 0xFFFFFFFF - TD_SHIM_SEC_INFO_OFFSET + 1
"
};
}

macro_rules! RUNTIME_TEMPLATE {
    () => {
        "// Copyright (c) 2021 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

// Auto-generated by `td-layout-config`, do not edit manually.

/*
    Mem Layout:
                                            Address
                    +--------------+ <-  0x0
                    |     Legacy   |
                    +--------------+ <-  0x00100000 (1M)
                    |   ........   |
                    +--------------+ <-  {pt_base:#010X}
                    |  Page Table  | <-  {pt_size:#010x}
                    +--------------+ <-  {td_hob_base:#010X}
                    |    TD HOB    |
                    +--------------+ <-  {payload_param_base:#010X}
                    | PAYLOAD PARAM|    ({payload_param_size:#010X})
                    +--------------+ <-  {payload_base:#010X}
                    |    PAYLOAD   |    ({payload_size:#010X})
                    +--------------+
                    |   ........   |
                    +--------------+ <-  {stack_base:#010X}
                    |     STACK    |    ({stack_size:#010X})
                    +--------------+ <-  {shadow_stack_base:#010X}
                    |      SS      |    ({shadow_stack_size:#010X})
                    +--------------+ <-  {payload_hob_base:#010X}
                    |  PAYLOAD_HOB |    ({payload_hob_size:#010X})
                    +--------------+ <-  {unaccepted_memory_bitmap_base:#010X}
                    |  UNACCEPTED  |    ({unaccepted_memory_bitmap_size:#010X})
                    +--------------+ <-  {acpi_base:#010X}
                    |     ACPI     |    ({acpi_size:#010X})
                    +--------------+ <-  {mailbox_base:#010X}
                    |    MAILBOX   |    ({mailbox_size:#010X})
                    +--------------+ <-  {event_log_base:#010X}
                    | TD_EVENT_LOG |    ({event_log_size:#010X})
                    +--------------+ <-  0x80000000 (2G) - for example
*/

pub const TD_PAYLOAD_EVENT_LOG_SIZE: u32 = {event_log_size:#X};
pub const TD_PAYLOAD_ACPI_SIZE: u32 = {acpi_size:#X};
pub const TD_PAYLOAD_MAILBOX_SIZE: u32 = {mailbox_size:#X};
pub const TD_PAYLOAD_UNACCEPTED_MEMORY_BITMAP_SIZE: u32 = {unaccepted_memory_bitmap_size:#X};
pub const TD_PAYLOAD_PARTIAL_ACCEPT_MEMORY_SIZE: u32 = {partial_accept_memory_size:#X};
pub const TD_PAYLOAD_HOB_SIZE: u32 = {payload_hob_size:#X};
pub const TD_PAYLOAD_SHADOW_STACK_SIZE: u32 = {shadow_stack_size:#X};
pub const TD_PAYLOAD_STACK_SIZE: u32 = {stack_size:#X};

pub const TD_PAYLOAD_PAGE_TABLE_BASE: u64 = {pt_base:#X};
pub const TD_PAYLOAD_PAGE_TABLE_SIZE: usize = {pt_size:#X};
pub const TD_HOB_BASE: u64 = {td_hob_base:#X};
pub const TD_HOB_SIZE: u64 = {td_hob_size:#X};
pub const TD_PAYLOAD_PARAM_BASE: u64 = {payload_param_base:#X};
pub const TD_PAYLOAD_PARAM_SIZE: u64 = {payload_param_size:#X};
pub const TD_PAYLOAD_BASE: u64 = {payload_base:#X};
pub const TD_PAYLOAD_SIZE: usize = {payload_size:#X};
"
    };
}

#[derive(Debug, PartialEq, Deserialize)]
struct TdLayoutConfig {
    image_layout: TdImageLayoutConfig,
    runtime_layout: TdRuntimeLayoutConfig,
}

#[derive(Debug, PartialEq, Deserialize)]
struct TdImageLayoutConfig {
    config_offset: u32,
    config_size: u32,
    mailbox_size: u32,
    temp_stack_guard_size: u32,
    temp_stack_size: u32,
    temp_heap_size: u32,
    payload_size: u32,
    metadata_size: u32,
    ipl_size: u32,
    reset_vector_size: u32,
}

#[derive(Debug, PartialEq, Deserialize)]
struct TdRuntimeLayoutConfig {
    event_log_size: u32,
    acpi_size: u32,
    mailbox_size: u32,
    unaccepted_memory_bitmap_size: u32,
    partial_accept_memory_size: u32,
    payload_hob_size: u32,
    shadow_stack_size: u32,
    stack_size: u32,
    payload_size: u32,
    payload_param_size: u32,
    payload_param_base: u32,
    td_hob_size: u32,
    page_table_size: u32,
    page_table_base: u32,
}

#[derive(Debug, Default, PartialEq)]
struct TdLayout {
    img: TdLayoutImage,
    img_loaded: TdLayoutImageLoaded,
    runtime: TdLayoutRuntime,
}

impl TdLayout {
    fn new_from_config(config: &TdLayoutConfig) -> Self {
        let img = TdLayoutImage::new_from_config(config);
        let img_loaded = TdLayoutImageLoaded::new_from_image(&img);

        TdLayout {
            img,
            img_loaded,
            runtime: TdLayoutRuntime::new_from_config(config),
        }
    }

    fn generate_build_time_rs(&self, output: &str) {
        let mut to_generate = Vec::new();
        write!(
            &mut to_generate,
            BUILD_TIME_TEMPLATE!(),
            // Image
            config_offset = self.img.config_offset,
            config_size = self.img.config_size,
            mailbox_offset = self.img.mailbox_offset,
            mailbox_size = self.img.mailbox_size,
            temp_stack_guard_size = self.img.temp_stack_guard_size,
            temp_stack_offset = self.img.temp_stack_offset,
            temp_stack_size = self.img.temp_stack_size,
            temp_heap_offset = self.img.temp_heap_offset,
            temp_heap_size = self.img.temp_heap_size,
            payload_offset = self.img.payload_offset,
            payload_size = self.img.payload_size,
            ipl_offset = self.img.ipl_offset,
            metadata_offset = self.img.metadata_offset,
            metadata_size = self.img.metadata_size,
            ipl_size = self.img.ipl_size,
            rst_vec_offset = self.img.rst_vec_offset,
            rst_vec_size = self.img.rst_vec_size,
            sec_core_info_offset = self.img.sec_core_info_offset,
            firmware_size = self.img.firmware_size,
            // Image loaded
            firmware_base = self.img_loaded.firmware_base,
            config_base = self.img_loaded.config_base,
            mailbox_base = self.img_loaded.mailbox_base,
            temp_stack_base = self.img_loaded.temp_stack_base,
            temp_heap_base = self.img_loaded.temp_heap_base,
            payload_base = self.img_loaded.payload_base,
            ipl_base = self.img_loaded.ipl_base,
            rst_vec_base = self.img_loaded.rst_vec_base,
            sec_core_info_base = self.img_loaded.sec_core_info_base,
        )
        .expect("Failed to generate configuration code from the template and JSON config");

        let dest_path = Path::new(output).join(TD_LAYOUT_BUILD_TIME_RS_OUT);
        fs::write(&dest_path, to_generate).expect(&format!(
            "Failed to write generated content to {}: {}",
            dest_path.display(),
            io::Error::last_os_error()
        ));
    }

    fn generate_runtime_rs(&self, output: &str) {
        let mut to_generate = Vec::new();
        write!(
            &mut to_generate,
            RUNTIME_TEMPLATE!(),
            pt_base = self.runtime.pt_base,
            pt_size = self.runtime.pt_size,
            td_hob_base = self.runtime.td_hob_base,
            td_hob_size = self.runtime.td_hob_size,
            payload_base = self.runtime.payload_base,
            payload_size = self.runtime.payload_size,
            stack_base = self.runtime.stack_base,
            stack_size = self.runtime.stack_size,
            shadow_stack_base = self.runtime.shadow_stack_base,
            shadow_stack_size = self.runtime.shadow_stack_size,
            payload_hob_base = self.runtime.payload_hob_base,
            payload_hob_size = self.runtime.payload_hob_size,
            unaccepted_memory_bitmap_base = self.runtime.unaccepted_memory_bitmap_base,
            unaccepted_memory_bitmap_size = self.runtime.unaccepted_memory_bitmap_size,
            partial_accept_memory_size = self.runtime.partial_accept_memory_size,
            mailbox_base = self.runtime.mailbox_base,
            mailbox_size = self.runtime.mailbox_size,
            event_log_base = self.runtime.event_log_base,
            event_log_size = self.runtime.event_log_size,
            acpi_base = self.runtime.acpi_base,
            acpi_size = self.runtime.acpi_size,
            payload_param_base = self.runtime.payload_param_base,
            payload_param_size = self.runtime.payload_param_size,
        )
        .expect("Failed to generate configuration code from the template and JSON config");

        let dest_path = Path::new(output).join(TD_LAYOUT_RUNTIME_RS_OUT);
        fs::write(&dest_path, to_generate).expect(&format!(
            "Failed to write generated content to {}: {}",
            dest_path.display(),
            io::Error::last_os_error()
        ));
    }
}

#[derive(Debug, Default, PartialEq)]
struct TdLayoutImage {
    config_offset: u32,
    config_size: u32,
    mailbox_offset: u32,
    mailbox_size: u32,
    temp_stack_guard_size: u32,
    temp_stack_offset: u32,
    temp_stack_size: u32,
    temp_heap_offset: u32,
    temp_heap_size: u32,
    payload_offset: u32,
    payload_size: u32,
    ipl_offset: u32,
    metadata_size: u32,
    metadata_offset: u32,
    ipl_size: u32,
    rst_vec_offset: u32,
    rst_vec_size: u32,
    sec_core_info_offset: u32,
    firmware_size: u32,
}

impl TdLayoutImage {
    fn new_from_config(config: &TdLayoutConfig) -> Self {
        let mailbox_offset = config.image_layout.config_offset + config.image_layout.config_size;
        let temp_stack_offset = mailbox_offset
            + config.image_layout.mailbox_size
            + config.image_layout.temp_stack_guard_size;
        let temp_heap_offset = temp_stack_offset + config.image_layout.temp_stack_size;
        let payload_offset = temp_heap_offset + config.image_layout.temp_heap_size;
        let metadata_offset = payload_offset + config.image_layout.payload_size;
        let ipl_offset = metadata_offset + config.image_layout.metadata_size;
        let rst_vec_offset = ipl_offset + config.image_layout.ipl_size;
        let sec_core_info_offset = TD_SHIM_SEC_INFO_OFFSET;
        let firmware_size = rst_vec_offset + config.image_layout.reset_vector_size;

        TdLayoutImage {
            config_offset: config.image_layout.config_offset,
            config_size: config.image_layout.config_size,
            mailbox_offset,
            mailbox_size: config.image_layout.mailbox_size,
            temp_stack_guard_size: config.image_layout.temp_stack_guard_size,
            temp_stack_offset,
            temp_stack_size: config.image_layout.temp_stack_size,
            temp_heap_offset,
            temp_heap_size: config.image_layout.temp_heap_size,
            payload_offset,
            payload_size: config.image_layout.payload_size,
            ipl_offset,
            metadata_offset,
            metadata_size: config.image_layout.metadata_size,
            ipl_size: config.image_layout.ipl_size,
            rst_vec_offset,
            rst_vec_size: config.image_layout.reset_vector_size,
            sec_core_info_offset,
            firmware_size,
        }
    }
}

#[derive(Debug, Default, PartialEq)]
struct TdLayoutImageLoaded {
    firmware_base: u32,
    config_base: u32,
    mailbox_base: u32,
    temp_stack_base: u32,
    temp_heap_base: u32,
    payload_base: u32,
    ipl_base: u32,
    rst_vec_base: u32,
    sec_core_info_base: u32,
}

impl TdLayoutImageLoaded {
    fn new_from_image(img: &TdLayoutImage) -> Self {
        let firmware_base = u32::MAX - img.firmware_size + 1;
        let config_base = firmware_base + img.config_offset;
        let mailbox_base = firmware_base + img.mailbox_offset;
        let temp_stack_base = firmware_base + img.temp_stack_offset;
        let temp_heap_base = firmware_base + img.temp_heap_offset;
        let payload_base = firmware_base + img.payload_offset;
        let ipl_base = firmware_base + img.ipl_offset;
        let rst_vec_base = firmware_base + img.rst_vec_offset;
        let sec_core_info_base = firmware_base + TD_SHIM_SEC_INFO_OFFSET;

        TdLayoutImageLoaded {
            firmware_base,
            config_base,
            mailbox_base,
            temp_stack_base,
            temp_heap_base,
            payload_base,
            ipl_base,
            rst_vec_base,
            sec_core_info_base,
        }
    }
}

#[derive(Debug, Default, PartialEq)]
struct TdLayoutRuntime {
    pt_base: u32,
    pt_size: u32,
    td_hob_base: u32,
    td_hob_size: u32,
    payload_base: u32,
    payload_size: u32,
    payload_param_base: u32,
    payload_param_size: u32,
    stack_base: u32,
    stack_size: u32,
    shadow_stack_base: u32,
    shadow_stack_size: u32,
    payload_hob_base: u32,
    payload_hob_size: u32,
    unaccepted_memory_bitmap_base: u32,
    unaccepted_memory_bitmap_size: u32,
    partial_accept_memory_size: u32,
    event_log_base: u32,
    event_log_size: u32,
    acpi_base: u32,
    acpi_size: u32,
    mailbox_base: u32,
    mailbox_size: u32,
}

impl TdLayoutRuntime {
    fn new_from_config(config: &TdLayoutConfig) -> Self {
        let event_log_base = 0x80000000 - config.runtime_layout.event_log_size; // TODO: 0x80000000 is hardcoded LOW_MEM_TOP, to remove
        let mailbox_base = event_log_base - config.runtime_layout.mailbox_size;
        let acpi_base = mailbox_base - config.runtime_layout.acpi_size;
        let unaccepted_memory_bitmap_base =
            acpi_base - config.runtime_layout.unaccepted_memory_bitmap_size;
        let payload_hob_base = acpi_base - config.runtime_layout.payload_hob_size;
        let shadow_stack_base = payload_hob_base - config.runtime_layout.shadow_stack_size;
        let stack_base = shadow_stack_base - config.runtime_layout.stack_size;
        let td_hob_base =
            config.runtime_layout.page_table_base + config.runtime_layout.page_table_size;
        let payload_base =
            config.runtime_layout.payload_param_base + config.runtime_layout.payload_param_size;

        TdLayoutRuntime {
            pt_base: config.runtime_layout.page_table_base,
            pt_size: config.runtime_layout.page_table_size,
            td_hob_base,
            td_hob_size: config.runtime_layout.td_hob_size,
            payload_param_base: config.runtime_layout.payload_param_base,
            payload_param_size: config.runtime_layout.payload_param_size,
            payload_base,
            payload_size: config.runtime_layout.payload_size,
            stack_base,
            stack_size: config.runtime_layout.stack_size,
            shadow_stack_base,
            shadow_stack_size: config.runtime_layout.shadow_stack_size,
            payload_hob_base,
            payload_hob_size: config.runtime_layout.payload_hob_size,
            unaccepted_memory_bitmap_base,
            unaccepted_memory_bitmap_size: config.runtime_layout.unaccepted_memory_bitmap_size,
            partial_accept_memory_size: config.runtime_layout.partial_accept_memory_size,
            event_log_base,
            event_log_size: config.runtime_layout.event_log_size,
            acpi_base,
            acpi_size: config.runtime_layout.acpi_size,
            mailbox_base,
            mailbox_size: config.runtime_layout.mailbox_size,
        }
    }
}

fn main() {
    let matches = command!()
        .arg(
            arg!([output] "Directory to store the generated layout files")
                .required(true)
                .allow_invalid_utf8(false),
        )
        .arg(
            arg!(
                -c --config <FILE> "Custom configuration file to generate the layout files from"
            )
            .required(true)
            .allow_invalid_utf8(false),
        )
        .get_matches();

    // Safe to unwrap because they are mandatory arguments.
    let config = matches.value_of("config").unwrap();
    let output = matches.value_of("output").unwrap();
    let data = fs::read_to_string(config).expect(&format!(
        "Failed to read configuration file {}, {}",
        config,
        io::Error::last_os_error()
    ));
    let td_layout_config: TdLayoutConfig = json5::from_str(&data).expect(&format!(
        "Content is configuration file {} is invalid",
        config
    ));
    let layout = TdLayout::new_from_config(&td_layout_config);

    // TODO: sanity checks on the layouts.
    // TODO: assert!(size_of::<TdxMetadata>() <= metadata_size);

    // Generate config .rs file from the template and JSON inputs, then write to fs.
    layout.generate_build_time_rs(output);
    layout.generate_runtime_rs(output);
}
