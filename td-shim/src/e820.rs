// Copyright (c) 2021 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use zerocopy::{AsBytes, FromBytes};

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum E820Type {
    Memory = 1,
    Reserved = 2,
    Acpi = 3,
    Nvs = 4,
    Unusable = 5,
    Disabled = 6,
    Pmem = 7,
    Unaccepted = 8,
    Unknown = 0xff,
}

impl From<u32> for E820Type {
    fn from(i: u32) -> Self {
        match i {
            1 => E820Type::Memory,
            2 => E820Type::Reserved,
            3 => E820Type::Acpi,
            4 => E820Type::Nvs,
            5 => E820Type::Unusable,
            6 => E820Type::Disabled,
            7 => E820Type::Pmem,
            8 => E820Type::Unaccepted,
            _ => E820Type::Unknown,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryDescriptor {
    pub r#type: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub number_of_pages: u64,
    pub attribute: u64,
}

pub type EfiMemoryType = u32;

impl Into<EfiMemoryType> for E820Type {
    fn into(self) -> EfiMemoryType {
        match self {
            E820Type::Memory => 7,
            E820Type::Reserved => 0,
            E820Type::Acpi => 9,
            E820Type::Nvs => 10,
            E820Type::Unusable => 8,
            E820Type::Disabled => 8,
            E820Type::Pmem => 14,
            E820Type::Unaccepted => 8,
            E820Type::Unknown => 8,
        }
    }
}

// impl Into<MemoryType> for E820Type {
//     fn into(self) -> MemoryType {
//         match self {
//             E820Type::Memory => MemoryType::ConventionalMemory,
//             E820Type::Reserved => MemoryType::ReservedMemoryType,
//             E820Type::Acpi => MemoryType::AcpiReclaimMemory,
//             E820Type::Nvs => MemoryType::AcpiMemoryNvs,
//             E820Type::Unusable => MemoryType::UnusableMemory,
//             E820Type::Disabled => MemoryType::UnusableMemory,
//             E820Type::Pmem => MemoryType::PersistentMemory,
//             E820Type::Unaccepted => MemoryType::UnusableMemory,
//             E820Type::Unknown => MemoryType::UnusableMemory,
//         }
//     }
// }

#[derive(Clone, Copy, Debug, Default, FromBytes, AsBytes, PartialEq)]
#[repr(C, packed)]
pub struct E820Entry {
    pub addr: u64,
    pub size: u64,
    pub r#type: u32,
}

impl Into<MemoryDescriptor> for E820Entry {
    fn into(self) -> MemoryDescriptor {
        MemoryDescriptor {
            r#type: E820Type::from(self.r#type).into(),
            physical_start: self.addr,
            virtual_start: self.addr,
            number_of_pages: self.size / 0x1000,
            attribute: 0,
        }
    }
}

impl E820Entry {
    pub fn new(addr: u64, size: u64, r#type: E820Type) -> Self {
        E820Entry {
            addr,
            size,
            r#type: r#type as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;
    const MAX_E820_ENTRY: usize = 128;

    #[test]
    fn test_e820_entry_size() {
        assert_eq!(size_of::<E820Entry>(), 20);
        assert_eq!(
            size_of::<[E820Entry; MAX_E820_ENTRY]>(),
            20 * MAX_E820_ENTRY
        );
    }
}
