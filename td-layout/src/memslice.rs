// Copyright (c) 2021 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use core::fmt::Display;

use crate::image::*;

/// Type of build time and runtime memory regions.
#[derive(Clone, Copy, Debug)]
pub enum SliceType {
    /// The `VAR` regions in image file
    Config,
    /// The `TD_HOB` region in image file
    TdHob,
    /// The `Payload and Metadata` region in image file
    BuiltinPayload,
    /// The `Metadata` region in image file
    Metadata,
    /// The `TD_MAILBOX` region in image file
    MailBox,
    /// The `payload` region in runtime memory layout
    Payload,
    /// The `Payload Parameter` region in runtime memory layout
    PayloadParam,
    /// The `TD_HOB` region in runtime memory layout
    PayloadHob,
    /// The 'Payload Mailbox' region in runtime memory layout
    PayloadMailbox,
    /// The 'Payload Page Table' region in runtime memory layout
    PayloadPageTable,
    /// The `TD_EVENT_LOG` region in runtime memory layout
    EventLog,
    /// The `ACPI` region in runtime memory layout
    Acpi,
    /// The 'UNACCEPTED_BITMAP' region in runtime memory layout
    LazyAcceptBitmap,
}

impl SliceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SliceType::Config => "Config",
            SliceType::TdHob => "TdHob",
            SliceType::BuiltinPayload => "BuiltinPayload",
            SliceType::Metadata => "Metadata",
            SliceType::MailBox => "MailBox",
            SliceType::Payload => "Payload",
            SliceType::PayloadParam => "PayloadParam",
            SliceType::PayloadHob => "PayloadHob",
            SliceType::PayloadMailbox => "PayloadMailbox",
            SliceType::EventLog => "EventLog",
            SliceType::Acpi => "Acpi",
            SliceType::LazyAcceptBitmap => "LazyAcceptBitmap",
            SliceType::PayloadPageTable => "PayloadPageTable",
        }
    }
}

impl Display for SliceType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Get an immutable reference to a fixed memory region.
///
/// These regions are read-only, so it's safe to return multiple immutable references.
pub fn get_mem_slice<'a>(t: SliceType) -> &'a [u8] {
    unsafe {
        match t {
            SliceType::Config => core::slice::from_raw_parts(
                TD_SHIM_CONFIG_BASE as *const u8,
                TD_SHIM_CONFIG_SIZE as usize,
            ),
            SliceType::BuiltinPayload => core::slice::from_raw_parts(
                TD_SHIM_BUILTIN_PAYLOAD_BASE as *const u8,
                TD_SHIM_BUILTIN_PAYLOAD_SIZE as usize,
            ),
            SliceType::Metadata => core::slice::from_raw_parts(
                TD_SHIM_METADATA_BASE as *const u8,
                TD_SHIM_METADATA_SIZE as usize,
            ),
            SliceType::MailBox => core::slice::from_raw_parts(
                TD_SHIM_MAILBOX_BASE as *const u8,
                TD_SHIM_MAILBOX_SIZE as usize,
            ),
            _ => panic!("get_mem_slice: not support"),
        }
    }
}

/// Get mutable reference to a fixed memory region.
///
/// # Safety
///
/// This function may break rust ownership model potentially. So caller must take the responsibility
/// to ensure ownership and concurrent access to the underlying data.
pub unsafe fn get_mem_slice_mut<'a>(t: SliceType) -> &'a mut [u8] {
    match t {
        SliceType::MailBox => core::slice::from_raw_parts_mut(
            TD_SHIM_MAILBOX_BASE as *const u8 as *mut u8,
            TD_SHIM_MAILBOX_SIZE as usize,
        ),
        SliceType::Config | SliceType::BuiltinPayload | SliceType::Metadata => {
            panic!("get_mem_slice_mut: read only")
        }
        _ => panic!("get_mem_slice_mut: not support"),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_mem_slice_with_type_config() {
        let config = get_mem_slice(SliceType::Config);
        assert_eq!(config.len(), TD_SHIM_CONFIG_SIZE as usize);
    }

    #[test]
    fn test_get_mem_slice_with_type_builtin_payload() {
        let payload = get_mem_slice(SliceType::BuiltinPayload);
        assert_eq!(payload.len(), TD_SHIM_BUILTIN_PAYLOAD_SIZE as usize);
    }

    #[test]
    #[should_panic(expected = "get_mem_slice: not support")]
    fn test_get_mem_slice_with_type_payload() {
        get_mem_slice(SliceType::Payload);
    }

    #[test]
    #[should_panic(expected = "get_mem_slice: not support")]
    fn test_get_mem_slice_with_type_payloadhob() {
        get_mem_slice(SliceType::PayloadHob);
    }

    #[test]
    fn test_get_mem_slice_with_type_mailbox() {
        let mailbox = get_mem_slice(SliceType::MailBox);
        assert_eq!(mailbox.len(), TD_SHIM_MAILBOX_SIZE as usize);
    }

    #[test]
    #[should_panic(expected = "get_mem_slice: not support")]
    fn test_get_mem_slice_with_type_eventlog() {
        get_mem_slice(SliceType::EventLog);
    }

    #[test]
    #[should_panic(expected = "get_mem_slice: not support")]
    fn test_get_mem_slice_with_type_acpi() {
        get_mem_slice(SliceType::Acpi);
    }

    #[test]
    fn test_get_mem_slice_mut_with_type_mailbox() {
        let mailbox = unsafe { get_mem_slice_mut(SliceType::MailBox) };
        assert_eq!(mailbox.len(), TD_SHIM_MAILBOX_SIZE as usize);
    }

    #[test]
    #[should_panic(expected = "get_mem_slice_mut: read only")]
    fn test_get_mem_slice_mut_with_type_builtin_payload() {
        unsafe {
            get_mem_slice_mut(SliceType::BuiltinPayload);
        }
    }

    #[test]
    #[should_panic(expected = "get_mem_slice_mut: read only")]
    fn test_get_mem_slice_mut_with_type_config() {
        unsafe {
            get_mem_slice_mut(SliceType::Config);
        }
    }

    #[test]
    #[should_panic(expected = "get_mem_slice_mut: not support")]
    fn test_get_mem_slice_mut_with_type_payloadhob() {
        unsafe {
            get_mem_slice_mut(SliceType::PayloadHob);
        }
    }

    #[test]
    #[should_panic(expected = "get_mem_slice_mut: not support")]
    fn test_get_mem_slice_mut_with_type_eventlog() {
        unsafe {
            get_mem_slice_mut(SliceType::EventLog);
        }
    }

    #[test]
    #[should_panic(expected = "get_mem_slice_mut: not support")]
    fn test_get_mem_slice_mut_with_type_acpi() {
        unsafe {
            get_mem_slice_mut(SliceType::Acpi);
        }
    }

    #[test]
    #[cfg(not(feature = "linux-payload"))]
    #[should_panic(expected = "get_mem_slice_mut: not support")]
    fn test_get_mem_slice_mut_with_type_payload() {
        unsafe {
            get_mem_slice_mut(SliceType::Payload);
        }
    }
}
