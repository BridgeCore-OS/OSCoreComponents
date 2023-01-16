// Copyright (c) ChefKiss Inc 2021-2023.
// This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives license.

#![no_std]
#![deny(warnings, clippy::cargo, clippy::nursery, unused_extern_crates)]

pub mod boot_attrs;
pub mod fb;
pub mod kern_sym;
pub mod mmap;

pub const CURRENT_REVISION: u64 = 0x17;

pub type EntryPoint = extern "sysv64" fn(&'static BootInfo) -> !;

#[repr(C)]
#[derive(Debug)]
pub struct BootInfo {
    pub revision: u64,
    pub kern_symbols: &'static [kern_sym::KernSymbol],
    pub settings: boot_attrs::BootSettings,
    pub memory_map: &'static [mmap::MemoryEntry],
    pub frame_buffer: Option<&'static fb::FrameBufferInfo>,
    pub acpi_rsdp: &'static acpi::tables::rsdp::RSDP,
    pub dc_cache: &'static [u8],
}

impl BootInfo {
    #[must_use]
    pub fn new(
        kern_symbols: &'static [kern_sym::KernSymbol],
        settings: boot_attrs::BootSettings,
        frame_buffer: Option<&'static fb::FrameBufferInfo>,
        acpi_rsdp: &'static acpi::tables::rsdp::RSDP,
        dc_cache: &'static [u8],
    ) -> Self {
        Self {
            revision: CURRENT_REVISION,
            kern_symbols,
            settings,
            memory_map: Default::default(),
            frame_buffer,
            acpi_rsdp,
            dc_cache,
        }
    }
}
