//! Copyright (c) VisualDevelopment 2021-2022.
//! This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives licence.

use alloc::vec::Vec;

use amd64::io::port::Port;
use log::debug;
use modular_bitfield::prelude::*;

use super::pci::{PciCmd, PciConfigOffset, PciDevice, PciIoAccessSize};

#[bitfield(bits = 16)]
#[derive(Default, Debug, Clone, Copy)]
#[repr(u16)]
pub struct MasterOutputVolume {
    pub right: B6,
    #[skip]
    __: B2,
    pub left: B6,
    #[skip]
    __: B1,
    pub mute: bool,
}

#[bitfield(bits = 16)]
#[derive(Default, Debug, Clone, Copy)]
#[repr(u16)]
pub struct PcmOutputVolume {
    pub right: B5,
    #[skip]
    __: B3,
    pub left: B5,
    #[skip]
    __: B2,
    pub mute: bool,
}

#[bitfield(bits = 8)]
#[derive(Default, Debug, Clone, Copy)]
#[repr(u8)]
pub struct RegBoxTransfer {
    pub transfer_data: bool,
    pub reset: bool,
    pub last_ent_fire_intr: bool,
    pub ioc_intr: bool,
    pub fifo_err_intr: bool,
    #[skip]
    __: B3,
}

#[bitfield(bits = 16)]
#[derive(Default, Debug, Clone, Copy)]
#[repr(u16)]
pub struct RegBoxStatus {
    pub transfer_data: bool,
    pub end_of_transfer: bool,
    pub last_ent_fire_intr: bool,
    pub ioc_intr: bool,
    pub fifo_err_intr: bool,
    #[skip]
    __: B11,
}

#[derive(Debug, BitfieldSpecifier, Default, Clone, Copy)]
#[bits = 2]
pub enum PcmChannels {
    #[default]
    Two = 0,
    Four,
    Six,
}

#[derive(Debug, BitfieldSpecifier, Default, Clone, Copy)]
#[bits = 2]
pub enum PcmOutMode {
    #[default]
    SixteenSamples = 0,
    TwentySamples,
}

#[bitfield(bits = 32)]
#[derive(Default, Debug, Clone, Copy)]
#[repr(u32)]
pub struct GlobalControl {
    pub interrupts: bool,
    pub cold_reset: bool,
    pub warm_reset: bool,
    pub shut_down: bool,
    #[skip]
    __: u16,
    pub channels: PcmChannels,
    pub pcm_out_mode: PcmOutMode,
    #[skip]
    __: u8,
}

#[bitfield(bits = 32)]
#[derive(Default, Debug, Clone, Copy)]
#[repr(u32)]
pub struct GlobalStatus {
    #[skip]
    __: B20,
    #[skip(setters)]
    pub channel_caps: PcmChannels,
    pub sample_caps: PcmOutMode,
    #[skip]
    __: u8,
}

#[bitfield(bits = 16)]
#[derive(Default, Debug, Clone, Copy)]
#[repr(u16)]
pub struct BufferDescCtl {
    #[skip]
    __: B14,
    pub last: bool,
    pub fire_interrupt: bool,
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(C, packed)]
pub struct BufferDescriptor {
    pub addr: u32,
    pub samples: u16,
    pub ctl: BufferDescCtl,
}

#[repr(u16)]
pub enum NamRegs {
    Reset = 0x0,
    MasterVolume = 0x2,
    PcmOutVolume = 0x18,
    SampleRate = 0x2C,
}

#[repr(u16)]
pub enum NabmRegs {
    PcmOutBdlAddr = 0x10,
    // PcmOutCurrentEnt = 0x14,
    PcmOutLastEnt = 0x15,
    PcmOutStatus = 0x16,
    // PcmOutTransferedSamples = 0x18,
    // PcmOutNextProcessedEnt = 0x1A,
    PcmOutTransferControl = 0x1B,
    GlobalControl = 0x2C,
    GlobalStatus = 0x30,
}

pub struct Ac97<'a> {
    pub dev: PciDevice<'a>,
    pub mixer_reset: Port<u16>,
    pub mixer_master_vol: Port<u16>,
    pub mixer_pcm_vol: Port<u16>,
    pub mixer_sample_rate: Port<u16>,
    pub global_ctl: Port<u32>,
    pub global_sts: Port<u32>,
    pub pcm_out_bdl_last_ent: Port<u8>,
    pub pcm_out_bdl_addr: Port<u32>,
    pub pcm_out_transf_ctl: Port<u8>,
    pub pcm_out_transf_sts: Port<u16>,
    pub buf: Vec<u8>,
    pub bdl: Vec<BufferDescriptor>,
}

impl<'a> Ac97<'a> {
    pub fn new(dev: PciDevice<'a>) -> Self {
        unsafe {
            dev.cfg_write(
                PciConfigOffset::Command as _,
                u16::from(
                    PciCmd::from(
                        dev.cfg_read(PciConfigOffset::Command as _, PciIoAccessSize::Word) as u16,
                    )
                    .with_pio(true)
                    .with_bus_master(true)
                    .with_disable_intrs(true),
                ) as _,
                PciIoAccessSize::Word,
            );
        }
        let audio_bus = unsafe {
            (dev.cfg_read(PciConfigOffset::BaseAddr1 as _, PciIoAccessSize::DWord) as u16) & !1u16
        };
        let global_ctl = Port::<u32>::new(audio_bus + NabmRegs::GlobalControl as u16);
        let global_sts = Port::<u32>::new(audio_bus + NabmRegs::GlobalStatus as u16);
        let pcm_out_bdl_last_ent = Port::<u8>::new(audio_bus + NabmRegs::PcmOutLastEnt as u16);
        let pcm_out_bdl_addr = Port::<u32>::new(audio_bus + NabmRegs::PcmOutBdlAddr as u16);
        let pcm_out_transf_ctl =
            Port::<u8>::new(audio_bus + NabmRegs::PcmOutTransferControl as u16);
        let pcm_out_transf_sts = Port::<u16>::new(audio_bus + NabmRegs::PcmOutStatus as u16);
        let mixer = unsafe {
            (dev.cfg_read(PciConfigOffset::BaseAddr0 as _, PciIoAccessSize::DWord) as u16) & !1u16
        };
        let mixer_reset = Port::<u16>::new(mixer + NamRegs::Reset as u16);
        let mixer_master_vol = Port::<u16>::new(mixer + NamRegs::MasterVolume as u16);
        let mixer_pcm_vol = Port::<u16>::new(mixer + NamRegs::PcmOutVolume as u16);
        let mixer_sample_rate = Port::<u16>::new(mixer + NamRegs::SampleRate as u16);

        let off_calc = |ent: u32| 0xFFFE * 2 * ent as u32;

        let mut buf = Vec::new();
        buf.resize(0x1F * 0xFFFE * 2, 0);
        let mut bdl = Vec::new();
        for i in 0..0x1F {
            bdl.push(BufferDescriptor {
                addr: (buf.as_ptr() as usize - amd64::paging::PHYS_VIRT_OFFSET) as u32
                    + off_calc(i),
                samples: 0xFFFE,
                ..Default::default()
            })
        }
        bdl.last_mut().unwrap().ctl.set_last(true);
        unsafe {
            // Resume from cold reset
            global_ctl.write(u32::from(
                GlobalControl::from(global_ctl.read())
                    .with_cold_reset(true)
                    .with_interrupts(false),
            ));
            mixer_reset.write(!0);

            // Set volume and sample rate
            mixer_master_vol.write(u16::from(
                MasterOutputVolume::new()
                    .with_right(0x3F)
                    .with_left(0x3F)
                    .with_mute(false),
            ));
            mixer_pcm_vol.write(u16::from(
                PcmOutputVolume::new()
                    .with_right(0x1F)
                    .with_left(0x1F)
                    .with_mute(false),
            ));
            debug!("Sample rate: {:#?}", mixer_sample_rate.read());
            // NOTE: QEMU has a bug and 48KHz audio doesn't work
            mixer_sample_rate.write(44100);

            // Reset output channel
            pcm_out_transf_ctl.write(u8::from(
                RegBoxTransfer::from(pcm_out_transf_ctl.read()).with_reset(true),
            ));
            while RegBoxTransfer::from(pcm_out_transf_ctl.read()).reset() {
                core::arch::asm!("hlt");
            }

            // Set BDL address and last entry
            pcm_out_bdl_addr.write((bdl.as_ptr() as usize - amd64::paging::PHYS_VIRT_OFFSET) as _);
            pcm_out_bdl_last_ent.write((bdl.len() - 1) as _);
        }

        Self {
            dev,
            global_ctl,
            global_sts,
            mixer_reset,
            mixer_master_vol,
            mixer_pcm_vol,
            mixer_sample_rate,
            pcm_out_bdl_last_ent,
            pcm_out_bdl_addr,
            pcm_out_transf_ctl,
            pcm_out_transf_sts,
            buf,
            bdl,
        }
    }

    pub fn play_audio(&mut self, data: &[u8]) {
        let mut off = 0;

        while off < data.len() {
            unsafe {
                // Reset output channel
                self.pcm_out_transf_ctl.write(u8::from(
                    RegBoxTransfer::from(self.pcm_out_transf_ctl.read()).with_reset(true),
                ));
                while RegBoxTransfer::from(self.pcm_out_transf_ctl.read()).reset() {
                    core::arch::asm!("pause");
                }

                // Set BDL address and last entry
                self.pcm_out_bdl_addr
                    .write((self.bdl.as_ptr() as usize - amd64::paging::PHYS_VIRT_OFFSET) as _);
                self.pcm_out_bdl_last_ent.write((self.bdl.len() - 1) as _);

                // Copy audio data to BDL
                for (a, b) in self
                    .buf
                    .iter_mut()
                    .zip(data[off..].iter().chain(core::iter::repeat(&0)))
                {
                    *a = *b
                }

                // Begin transfer
                self.pcm_out_transf_ctl.write(u8::from(
                    RegBoxTransfer::from(self.pcm_out_transf_ctl.read()).with_transfer_data(true),
                ));

                while !RegBoxStatus::from(self.pcm_out_transf_sts.read()).end_of_transfer() {
                    core::arch::asm!("pause");
                }
            }
            off += 0x1F * 0xFFFE * 2;
        }
    }
}
