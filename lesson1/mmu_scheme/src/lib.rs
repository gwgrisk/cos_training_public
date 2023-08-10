#![no_std]
#![feature(asm_const)]

use riscv::register::satp;

pub const KERNEL_BASE: usize = 0xffff_ffff_c000_0000;

const PHYS_VIRT_OFFSET: usize = 0xffff_ffc0_0000_0000;

#[link_section = ".data.boot_page_table"]
static mut BOOT_PT_SV39: [u64; 512] = [0; 512];
static mut BOOT_PT_SV48: [[u64; 512]; 4] = [[0; 512]; 4];

pub unsafe fn pre_mmu() {
    #[cfg(feature = "sv39")]
    {
        // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
        BOOT_PT_SV39[2] = (0x80000 << 10) | 0xef;
        // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
        BOOT_PT_SV39[0x102] = (0x80000 << 10) | 0xef;

        // 0xffff_ffff_c000_0000..highest, VRWX_GAD, 1G block
        BOOT_PT_SV39[0x1ff] = (0x80000 << 10) | 0xef;
    }
    #[cfg(feature = "sv48")]
    {
        let phys_addr = 0xffff_ffc0_0000_0000;
        let virt_addr = 0x80000;

        let l0_index = (virt_addr << 10) & 0xef;
        let l1_index = (virt_addr << 10) & 0xef;
        let l2_index = (virt_addr << 10) & 0xef;
        let l3_index = (virt_addr << 10) & 0xef;

        let l1_table = &mut BOOT_PT_SV48[1];
        BOOT_PT_SV48[0][l0_index] = l1_table.as_ptr() as u64 | 0xef;

        let l2_table = &mut BOOT_PT_SV48[2];
        l1_table[l1_index] = l2_table.as_ptr() as u64 | 0xef;

        let l3_table = &mut BOOT_PT_SV48[3];
        l2_table[l2_index] = l3_table.as_ptr() as u64 | 0xef;

        l3_table[l3_index] = (phys_addr << 10) | 0xef;
    }
}

pub unsafe fn enable_mmu() {
    #[cfg(feature = "sv39")]
    {
        let page_table_root = BOOT_PT_SV39.as_ptr() as usize;
        satp::set(satp::Mode::Sv39, 0, page_table_root >> 12);
        riscv::asm::sfence_vma_all();
    }
    #[cfg(feature = "sv48")]
    {
        let page_table_root = BOOT_PT_SV48[0].as_ptr() as usize;
        satp::set(satp::Mode::Sv48, 0, page_table_root >> 12);
        riscv::asm::sfence_vma_all();
    }
}

pub unsafe fn post_mmu() {
    core::arch::asm!("
        li      t0, {phys_virt_offset}  // fix up virtual high address
        add     sp, sp, t0
        add     ra, ra, t0
        ret     ",
        phys_virt_offset = const PHYS_VIRT_OFFSET,
    )
}
