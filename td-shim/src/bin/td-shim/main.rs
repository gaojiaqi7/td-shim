// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

#![allow(unused)]
#![feature(alloc_error_handler)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(unused_imports)]

extern crate alloc;
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;
use core::mem::size_of;
use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;
use td_shim::e820::MemoryDescriptor;

use r_efi::efi;
use scroll::{Pread, Pwrite};
use zerocopy::{AsBytes, ByteSlice, FromBytes};

use td_layout::build_time::{self, *};
use td_layout::memslice;
use td_layout::runtime::{self, *};
use td_layout::RuntimeMemoryLayout;
use td_shim::acpi::GenericSdtHeader;
use td_shim::event_log::{
    self, TdHandoffTable, TdHandoffTablePointers, UefiPlatformFirmwareBlob2,
    EV_EFI_HANDOFF_TABLES2, EV_EFI_PLATFORM_FIRMWARE_BLOB2, EV_PLATFORM_CONFIG_FLAGS,
    PLATFORM_CONFIG_HOB, PLATFORM_FIRMWARE_BLOB2_PAYLOAD, TD_LOG_EFI_HANDOFF_TABLE_GUID,
};
use td_shim::{
    speculation_barrier, PayloadInfo, TdKernelInfoHobType, TD_ACPI_TABLE_HOB_GUID,
    TD_KERNEL_INFO_HOB_GUID,
};
use td_uefi_pi::{fv, hob, pi};

use crate::ipl::ExecutablePayloadType;
use crate::tcg::TdEventLog;
use crate::td_hob::TdHobInfo;

use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::paging::{mapper::*, *};
use x86_64::{align_up, PhysAddr, VirtAddr};
use xmas_elf::{program, ElfFile};

mod acpi;
mod asm;
mod e820;
mod heap;
mod ipl;
mod linux;
mod memory;
mod mp;
mod payload_hob;
mod stack_guard;
mod tcg;
mod td;
mod td_hob;

#[cfg(feature = "cet-ss")]
mod cet_ss;

extern "win64" {
    fn switch_stack_call_win64(entry_point: usize, stack_top: usize, P1: usize, P2: usize);
    fn switch_stack_call_sysv(entry_point: usize, stack_top: usize, P1: usize, P2: usize);
}

#[cfg(not(test))]
#[panic_handler]
#[allow(clippy::empty_loop)]
fn panic(_info: &PanicInfo) -> ! {
    log::info!("panic ... {:?}\n", _info);
    panic!("deadloop");
}

#[cfg(not(test))]
#[alloc_error_handler]
#[allow(clippy::empty_loop)]
fn alloc_error(_info: core::alloc::Layout) -> ! {
    log::info!("alloc_error ... {:?}\n", _info);
    panic!("deadloop");
}

/// Main entry point of the td-shim, and the bootstrap code should jump here.
///
/// The bootstrap should prepare the context to satisfy `_start()`'s expectation:
/// - the memory is in 1:1 identity mapping mode with paging enabled
/// - the stack is ready for use
///
/// # Arguments
/// - `boot_fv`: pointer to the boot firmware volume
/// - `top_of_start`: top address of the stack
/// - `init_vp`: [31:0] TDINITVP - Untrusted Configuration
/// - `info`: [6:0] CPU supported GPA width, [7:7] 5 level page table support, [23:16] VCPUID,
///           [32:24] VCPU_Index
#[cfg(not(test))]
#[no_mangle]
#[export_name = "efi_main"]
pub extern "win64" fn _start(
    boot_fv: *const c_void,
    top_of_stack: *const c_void,
    init_vp: *const c_void,
    info: usize,
) -> ! {
    // The bootstrap code has setup the stack, but only the stack is available now...
    let _ = td_logger::init();
    log::info!("Starting RUST Based TdShim boot_fv - {:p}, Top of stack - {:p}, init_vp - {:p}, info - 0x{:x} \n",
               boot_fv, top_of_stack, init_vp, info);
    td_exception::setup_exception_handlers();
    log::info!("setup_exception_handlers done\n");

    // First initialize the heap allocator so that we have a normal rust world to live in...
    heap::init();

    // Get HOB list
    let hob_list = hob::check_hob_integrity(memslice::get_mem_slice(memslice::SliceType::TdHob))
        .expect("Integrity check failed: invalid HOB list");
    hob::dump_hob(hob_list);
    let mut td_hob_info =
        TdHobInfo::read_from_hob(hob_list).expect("Error occurs reading from VMM HOB");

    // Initialize memory subsystem.
    let mut mem = memory::Memory::new(&td_hob_info.memory)
        .expect("Unable to find a piece of suitable memory for runtime");
    mem.setup_paging();

    // Relocate the page table that map all the physical memory
    td::relocate_ap_page_table(TD_PAYLOAD_PAGE_TABLE_BASE);
    // Relocate Mailbox along side with the AP function
    td::relocate_mailbox(mem.layout.runtime_mailbox_base as u32);

    // Set up the TD event log buffer.
    // Safe because it's used to initialize the EventLog subsystem which ensures safety.
    let event_log_buf = unsafe {
        memslice::get_dynamic_mem_slice_mut(
            memslice::SliceType::EventLog,
            mem.layout.runtime_event_log_base as usize,
        )
    };
    let mut td_event_log =
        tcg::TdEventLog::new(event_log_buf).expect("Failed to create and initialize the event log");
    log_hob_list(hob_list, &mut td_event_log);

    let num_vcpus = td::get_num_vcpus();
    //Create MADT and TDEL
    let (madt, tdel) = prepare_acpi_tables(
        &mut td_hob_info.acpi_tables,
        &mem.layout,
        &mut td_event_log,
        num_vcpus,
    );
    td_hob_info.acpi_tables.push(madt.as_bytes());
    td_hob_info.acpi_tables.push(tdel.as_bytes());

    // If the Payload Information GUID HOB is present, try to boot the Linux kernel.
    if let Some(payload_info) = td_hob_info.payload_info {
        boot_linux_kernel(
            &payload_info,
            &td_hob_info.acpi_tables,
            &mem,
            &mut td_event_log,
            num_vcpus,
        );
    }

    boot_zcore(&mut mem, &mut td_event_log, &td_hob_info.acpi_tables);

    panic!("payload entry() should not return here, deadloop!!!");
}

fn boot_linux_kernel(
    kernel_info: &PayloadInfo,
    acpi_tables: &Vec<&[u8]>,
    mem: &memory::Memory,
    td_event_log: &mut TdEventLog,
    vcpus: u32,
) {
    // Create an EV_SEPARATOR event to mark the end of the td-shim events
    td_event_log.create_seperator();

    let image_type = TdKernelInfoHobType::from(kernel_info.image_type);
    match image_type {
        TdKernelInfoHobType::ExecutablePayload => return,
        TdKernelInfoHobType::BzImage | TdKernelInfoHobType::RawVmLinux => {}
        _ => panic!("Unknown kernel image type {}!!!", kernel_info.image_type),
    };

    let rsdp = install_acpi_tables(acpi_tables, &mem.layout);
    let e820_table = mem.create_e820();
    log::info!("e820 table: {:x?}\n", e820_table.as_slice());
    // Safe because we are handle off this buffer to linux kernel.
    let payload = unsafe { memslice::get_mem_slice_mut(memslice::SliceType::Payload) };

    linux::boot::boot_kernel(
        payload,
        rsdp,
        e820_table.as_slice(),
        kernel_info,
        #[cfg(feature = "tdx")]
        mem.build_unaccepted_memory_bitmap(),
    );
    panic!("Linux kernel should not return here!!!");
}

struct PageAllocator {
    allocator: LockedHeap,
}

unsafe impl FrameAllocator<Size4KiB> for PageAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let addr = unsafe {
            self.allocator
                .alloc(Layout::from_size_align_unchecked(0x1000, 0x1000)) as u64
        };
        Some(PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

/// This structure represents the information that the bootloader passes to the kernel.
#[repr(C)]
#[derive(Debug)]
pub struct BootInfo {
    pub memory_map: Vec<&'static MemoryDescriptor>,
    /// The offset into the virtual address space where the physical memory is mapped.
    pub physical_memory_offset: u64,
    /// The graphic output information
    pub graphic_info: GraphicInfo,
    /// Physical address of ACPI2 RSDP
    pub acpi2_rsdp_addr: u64,
    /// Physical address of SMBIOS
    pub smbios_addr: u64,
    /// The start physical address of initramfs
    pub initramfs_addr: u64,
    /// The size of initramfs
    pub initramfs_size: u64,
    /// Kernel command line
    pub cmdline: &'static str,
}

/// Graphic output information
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct GraphicInfo {
    /// Graphic mode
    pub mode: [u64; 5],
    /// Framebuffer base physical address
    pub fb_addr: u64,
    /// Framebuffer size
    pub fb_size: u64,
}

static mut EFI_MEMORY_MAP: [MemoryDescriptor; 128] = [MemoryDescriptor {
    r#type: 0,
    physical_start: 0,
    virtual_start: 0,
    number_of_pages: 0,
    attribute: 0,
}; 128];

fn boot_zcore(mem: &mut memory::Memory, td_event_log: &mut TdEventLog, acpi_tables: &Vec<&[u8]>) {
    // Create an EV_SEPARATOR event to mark the end of the td-shim events
    td_event_log.create_seperator();

    // Get and parse image file from the payload firmware volume.
    let fv_buffer = memslice::get_mem_slice(memslice::SliceType::ShimPayload);
    let mut payload_bin = fv::get_image_from_fv(
        fv_buffer,
        pi::fv::FV_FILETYPE_DXE_CORE,
        pi::fv::SECTION_PE32,
    )
    .expect("Failed to get image file from Firmware Volume");

    // Copy the zcore binary out from the BFV to the paylaod memory region
    // to make sure the alignment
    let payload_mem = unsafe { memslice::get_mem_slice_mut(memslice::SliceType::Payload) };
    payload_mem[..payload_bin.len()].copy_from_slice(payload_bin);

    let elf = { ElfFile::new(payload_mem).expect("failed to parse ELF") };
    let entry = unsafe { elf.header.pt2.entry_point() as usize };

    let kernel_start = PhysAddr::new(elf.input.as_ptr() as u64);

    // Create an allocator using the second half of the payload memory region.
    // The first half is used to put the raw bianry.
    let mut allocator = PageAllocator {
        allocator: unsafe {
            LockedHeap::new(
                TD_PAYLOAD_BASE as usize + TD_PAYLOAD_SIZE >> 1,
                TD_PAYLOAD_SIZE >> 1,
            )
        },
    };
    for segment in elf.program_iter() {
        map_segment(&segment, kernel_start, &mut mem.pt, &mut allocator).unwrap();
    }

    td_paging::cr3_write();

    let rsdp = install_acpi_tables(acpi_tables, &mem.layout);
    let e820_table = mem.create_e820();

    let mut efi_mem_map: Vec<&'static MemoryDescriptor> = Vec::new();
    for (idx, e820_entry) in e820_table.as_slice().iter().enumerate() {
        unsafe {
            EFI_MEMORY_MAP[idx] = (*e820_entry).into();
            efi_mem_map.push(&(EFI_MEMORY_MAP[idx]));
        }
    }

    let graphic_info = GraphicInfo {
        mode: [0; 5],
        fb_addr: 0,
        fb_size: 0,
    };

    // construct BootInfo
    let bootinfo = BootInfo {
        memory_map: efi_mem_map,
        physical_memory_offset: 0,
        graphic_info,
        acpi2_rsdp_addr: rsdp,
        smbios_addr: 0,
        initramfs_addr: 0,
        initramfs_size: 0,
        cmdline: "",
    };

    let stacktop = mem.layout.runtime_stack_top as usize;

    log::info!("Jump to zcore entry point...\n");
    unsafe { switch_stack_call_sysv(entry, stacktop, &bootinfo as *const BootInfo as usize, 0) };
    panic!("Kernel should not return here!!!");
}

fn map_segment(
    segment: &program::ProgramHeader,
    kernel_start: PhysAddr,
    page_table: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    if segment.get_type().unwrap() != program::Type::Load {
        return Ok(());
    }
    log::info!("mapping segment: {:#x?}\n", segment);
    let mem_size = segment.mem_size();
    let file_size = segment.file_size();
    let file_offset = segment.offset() & !0xfff;
    let phys_start_addr = kernel_start + file_offset;
    let virt_start_addr = VirtAddr::new(segment.virtual_addr());

    let start_page: Page = Page::containing_address(virt_start_addr);
    let start_frame = PhysFrame::containing_address(phys_start_addr);
    let end_frame = PhysFrame::containing_address(phys_start_addr + file_size - 1u64);

    log::info!(
        "phys_start_addr is {:x?}, phys_start_addr + file_size - 1u64 is {:x?}\n",
        phys_start_addr,
        phys_start_addr + file_size - 1u64
    );
    log::info!(
        "start_frame is {:x?}, end_frame is {:x?}\n",
        start_frame,
        end_frame
    );
    let flags = segment.flags();
    let mut page_table_flags = PageTableFlags::PRESENT;
    if !flags.is_execute() {
        // page_table_flags |= PageTableFlags::NO_EXECUTE
    };
    page_table_flags |= PageTableFlags::WRITABLE;
    if flags.is_write() {};

    if file_size != 0 {
        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            let offset = frame - start_frame;
            let page = start_page + offset;
            log::info!("page is {:x?}\n", page);
            unsafe {
                page_table
                    .map_to(page, frame, page_table_flags, frame_allocator)?
                    .flush();
            }
        }
    }

    if mem_size > file_size {
        // .bss section (or similar), which needs to be zeroed
        let zero_start = virt_start_addr + file_size;
        let zero_end = virt_start_addr + mem_size;
        if zero_start.as_u64() & 0xfff != 0 {
            // A part of the last mapped frame needs to be zeroed. This is
            // not possible since it could already contains parts of the next
            // segment. Thus, we need to copy it before zeroing.

            let new_frame = frame_allocator
                .allocate_frame()
                .ok_or(MapToError::FrameAllocationFailed)?;

            type PageArray = [u64; Size4KiB::SIZE as usize / 8];

            let last_page = Page::containing_address(virt_start_addr + file_size - 1u64);
            let last_page_ptr = end_frame.start_address().as_u64() as *mut PageArray;
            let temp_page_ptr = new_frame.start_address().as_u64() as *mut PageArray;

            unsafe {
                // copy contents
                temp_page_ptr.write(last_page_ptr.read());
            }

            // remap last page
            if let Err(e) = page_table.unmap(last_page) {
                return Err(match e {
                    UnmapError::ParentEntryHugePage => MapToError::ParentEntryHugePage,
                    UnmapError::PageNotMapped => unreachable!(),
                    UnmapError::InvalidFrameAddress(_) => unreachable!(),
                });
            }
            unsafe {
                page_table
                    .map_to(last_page, new_frame, page_table_flags, frame_allocator)?
                    .flush();
            }
        }
        // Map additional frames.
        let start_page: Page =
            Page::containing_address(VirtAddr::new(align_up(zero_start.as_u64(), Size4KiB::SIZE)));
        let end_page = Page::containing_address(zero_end);
        log::info!(
            "start_page is {:x?}, end_page is {:x?}\n",
            start_page,
            end_page
        );
        for page in Page::range_inclusive(start_page, end_page) {
            let frame = frame_allocator
                .allocate_frame()
                .ok_or(MapToError::FrameAllocationFailed)?;
            unsafe {
                page_table
                    .map_to(page, frame, page_table_flags, frame_allocator)?
                    .flush();
            }
        }

        // zero bss
        unsafe {
            core::ptr::write_bytes(
                zero_start.as_mut_ptr::<u8>(),
                0,
                (mem_size - file_size) as usize,
            );
        }
    }
    Ok(())
}

fn boot_builtin_payload(
    mem: &mut memory::Memory,
    td_event_log: &mut TdEventLog,
    acpi_tables: &Vec<&[u8]>,
) {
    // Get and parse image file from the payload firmware volume.
    let fv_buffer = memslice::get_mem_slice(memslice::SliceType::ShimPayload);
    let mut payload = fv::get_image_from_fv(
        fv_buffer,
        pi::fv::FV_FILETYPE_DXE_CORE,
        pi::fv::SECTION_PE32,
    )
    .expect("Failed to get image file from Firmware Volume");

    #[cfg(feature = "secure-boot")]
    {
        payload = secure_boot_verify_payload(payload, td_event_log);
    }

    // Record the payload binary information into event log.
    log_payload_binary(payload, td_event_log);

    // Create an EV_SEPARATOR event to mark the end of the td-shim events
    td_event_log.create_seperator();

    let relocation_info =
        ipl::find_and_report_entry_point(mem, payload).expect("Entry point not found!");

    // Set up NX (no-execute) protection for payload stack and hob
    mem.set_nx_bit(mem.layout.runtime_stack_base, TD_PAYLOAD_STACK_SIZE as u64);
    mem.set_nx_bit(
        mem.layout.runtime_shadow_stack_base,
        TD_PAYLOAD_SHADOW_STACK_SIZE as u64,
    );
    mem.set_nx_bit(mem.layout.runtime_hob_base, TD_PAYLOAD_HOB_SIZE as u64);

    // Initialize the stack to run the image
    stack_guard::stack_guard_enable(mem);
    #[cfg(feature = "cet-ss")]
    cet_ss::enable_cet_ss(
        mem.layout.runtime_shadow_stack_base,
        mem.layout.runtime_shadow_stack_top,
    );
    let stack_top = (mem.layout.runtime_stack_base + TD_PAYLOAD_STACK_SIZE as u64) as usize;

    // Prepare the HOB list to run the image
    payload_hob::build_payload_hob(acpi_tables, &mem).expect("Fail to create payload HOB");

    // Finally let's switch stack and jump to the image entry point...
    log::info!(
        " start launching payload {:p} and switch stack {:p}...\n",
        relocation_info.entry_point as *const usize,
        stack_top as *const usize
    );

    let switch_stack_call = match relocation_info.image_type {
        ExecutablePayloadType::Elf => switch_stack_call_sysv,
        ExecutablePayloadType::PeCoff => switch_stack_call_win64,
    };

    unsafe {
        switch_stack_call(
            relocation_info.entry_point as usize,
            stack_top,
            mem.layout.runtime_hob_base as usize,
            TD_PAYLOAD_BASE as usize,
        )
    };
}

// Install ACPI tables into ACPI reclaimable memory for the virtual machine
// and panics if error happens.
fn install_acpi_tables(acpi_tables: &Vec<&[u8]>, layout: &RuntimeMemoryLayout) -> u64 {
    // Safe because BSP is the only active vCPU so it's single-threaded context.
    let acpi_slice = unsafe {
        memslice::get_dynamic_mem_slice_mut(
            memslice::SliceType::Acpi,
            layout.runtime_acpi_base as usize,
        )
    };
    let mut acpi = acpi::AcpiTables::new(acpi_slice, acpi_slice.as_ptr() as *const _ as u64);

    for &table in acpi_tables {
        acpi.install(table);
    }

    acpi.finish()
}

// Prepare ACPI tables for payload and panic if error happens
fn prepare_acpi_tables(
    acpi_tables: &mut Vec<&[u8]>,
    layout: &RuntimeMemoryLayout,
    td_event_log: &mut TdEventLog,
    vcpus: u32,
) -> (mp::Madt, event_log::Ccel) {
    let mut vmm_madt = None;
    let mut idx = 0;
    while idx < acpi_tables.len() {
        let table = acpi_tables[idx];
        if table.len() < size_of::<GenericSdtHeader>() {
            panic!("Invalid ACPI table HOB\n");
        }
        speculation_barrier();

        let header = GenericSdtHeader::read_from(&table[..size_of::<GenericSdtHeader>()])
            .expect("Faile to read table header from ACPI GUID HOB");
        if table.len() < header.length as usize {
            panic!("Invalid ACPI table length\n");
        }
        speculation_barrier();

        if &header.signature == b"APIC" {
            vmm_madt = Some(table);
            acpi_tables.remove(idx);
        }
        idx += 1;
    }

    let madt = if let Some(vmm_madt) = vmm_madt {
        mp::create_madt(vmm_madt, layout.runtime_mailbox_base as u64)
            .expect("Failed to create ACPI MADT table")
    } else {
        mp::create_madt_default(vcpus, layout.runtime_mailbox_base as u64)
            .expect("Failed to create ACPI MADT table")
    };

    let tdel = td_event_log.create_ccel();

    (madt, tdel)
}

#[cfg(feature = "secure-boot")]
fn secure_boot_verify_payload<'a>(payload: &'a [u8], td_event_log: &mut TdEventLog) -> &'a [u8] {
    use td_shim::event_log::{
        UefiPlatformFirmwareBlob2, EV_EFI_PLATFORM_FIRMWARE_BLOB2,
        PLATFORM_CONFIG_SECURE_AUTHORITY, PLATFORM_CONFIG_SECURE_POLICY_DB, PLATFORM_CONFIG_SVN,
        PLATFORM_FIRMWARE_BLOB2_PAYLOAD,
    };
    use td_shim::secure_boot::PayloadVerifier;

    let cfv = memslice::get_mem_slice(memslice::SliceType::Config);
    let verifier = PayloadVerifier::new(payload, cfv)
        .expect("Secure Boot: Cannot read verify header from payload binary");
    let trust_anchor =
        PayloadVerifier::get_trust_anchor(cfv).expect("Fail to get trust anchor from CFV");

    // Record the provisioned trust anchor into event log.
    td_event_log
        .create_event_log_platform_config(1, PLATFORM_CONFIG_SECURE_POLICY_DB, trust_anchor)
        .expect("Fail to measure and log the provisioned trust anchor");

    verifier.verify().expect("Verification fails");

    // Record the matched trust anchor which is same as the provisioned
    // trust anchor if it passes the verification.
    td_event_log
        .create_event_log_platform_config(1, PLATFORM_CONFIG_SECURE_AUTHORITY, trust_anchor)
        .expect("Fail to measure and log the provisioned trust anchor");

    // Record the payload SVN into event log.
    td_event_log
        .create_event_log_platform_config(
            2,
            PLATFORM_CONFIG_SVN,
            &u64::to_le_bytes(verifier.get_payload_svn()),
        )
        .expect("Fail to measure and log the payload SVN");

    // Parse out the image from signed payload
    return PayloadVerifier::get_payload_image(payload)
        .expect("Unable to get payload image from signed binary");
}

fn log_hob_list(hob_list: &[u8], td_event_log: &mut tcg::TdEventLog) {
    td_event_log
        .create_event_log_platform_config(1, PLATFORM_CONFIG_HOB, hob_list)
        .expect("Failed to log HOB list to the td event log");
}

fn log_payload_binary(payload: &[u8], td_event_log: &mut tcg::TdEventLog) {
    let blob2 = UefiPlatformFirmwareBlob2::new(
        PLATFORM_FIRMWARE_BLOB2_PAYLOAD,
        payload.as_ptr() as u64,
        payload.len() as u64,
    )
    .expect("Invalid payload binary information or descriptor");

    td_event_log
        .create_event_log(2, EV_EFI_PLATFORM_FIRMWARE_BLOB2, blob2.as_bytes(), payload)
        .expect("Failed to log HOB list to the td event log");
}
