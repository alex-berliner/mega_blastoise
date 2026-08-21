MEMORY {
    /*
     * Pico 2 flash. No BOOT2 window here: the RP2350 boot ROM reads flash
     * itself and looks for an IMAGE_DEF block in the first 4K instead of
     * running a second-stage loader.
     */
    FLASH : ORIGIN = 0x10000000, LENGTH = 4096K
    /* SRAM0-7, striped mapping. */
    RAM : ORIGIN = 0x20000000, LENGTH = 512K
    /* Direct-mapped banks, for anything that wants predictable access. */
    SRAM8 : ORIGIN = 0x20080000, LENGTH = 4K
    SRAM9 : ORIGIN = 0x20081000, LENGTH = 4K
}

SECTIONS {
    /* IMAGE_DEF (lib.rs) — must sit in the first 4K where the ROM scans. */
    .start_block : ALIGN(4)
    {
        __start_block_addr = .;
        KEEP(*(.start_block));
        KEEP(*(.boot_info));
    } > FLASH
} INSERT AFTER .vector_table;

/* Move .text to start after the boot info. */
_stext = ADDR(.start_block) + SIZEOF(.start_block);

SECTIONS {
    /* picotool 'Binary Info' entries, if anything ever emits them. */
    .bi_entries : ALIGN(4)
    {
        __bi_entries_start = .;
        KEEP(*(.bi_entries));
        . = ALIGN(4);
        __bi_entries_end = .;
    } > FLASH
} INSERT AFTER .text;

SECTIONS {
    /* Trailer the ROM can chase from the start block (signature space). */
    .end_block : ALIGN(4)
    {
        __end_block_addr = .;
        KEEP(*(.end_block));
    } > FLASH
} INSERT AFTER .uninit;

PROVIDE(start_to_end = __end_block_addr - __start_block_addr);
PROVIDE(end_to_start = __start_block_addr - __end_block_addr);
