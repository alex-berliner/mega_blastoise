# RP2350 errata against this design (VER-16)

Source: hardware/ref_design/datasheets/rp2350.pdf@611ebe3 (deleted in 574c94c with the RP2354B board), Appendix E (rp2350.txt:102278-103505, the
`pdftotext -layout` copy; its sha256 is in the VER-16 evidence). Design: hardware/ref_design at
71737ae (U4 is an RP2354B, QFN-80; design.py:332). Every erratum in the appendix is listed, 28 in all.

**The stepping is not known.** JLC's RP2354B (C39843328) may be A2, A3 or A4. The marking tells
(rp2350.txt:102136, section 14.4). Errata marked "A2 only" apply only if the parts are A2. The first
article inspection (VER-109) reads the marking on unit 1 and records it here.

"Applies" means it can affect this board or its firmware. A hardware erratum that applies needs a
fix on the board. A software erratum that applies needs a firmware rule. "Firmware rule" names
where the rule is written. "No" says why not.

| Erratum | Summary (the datasheet's words, shortened) | Steppings | Applies here? | Fix or rule |
|---|---|---|---|---|
| E1 | Interpolator OVERF bits broken by right-rotate | A2-A4 | No: the firmware does not use the interpolator's OVERF | none |
| E2 | SIO SPINLOCK writes mirrored at +0x80 | A2-A4 | Software only: the SDK's spinlocks | the SDK's workaround (use the SDK's spin_lock API) |
| E3 | QFN-60: GPIO_NSMASK controls wrong PADS registers | A2, QFN-60 only | No: U4 is QFN-80 | none |
| E4 | System bus access stalls when core 1 is in clock-gated sleep | A2-A4 | Debug only: a probe reading memory while core 1 sleeps | none for the board; a debugger note |
| E5 | DMA CHAIN_TO and ABORT interactions | A2-A4 | Software only: if the display or audio DMA aborts chained channels | firmware rule: abort a chain as the datasheet's workaround says |
| E6 | RISC-V PMPCFGx RWX fields transposed | A2-A4 | No: the firmware runs on the Arm cores | none |
| E7 | RISC-V U-mode does not ignore mstatus.mie | A2-A4 | No: Arm cores | none |
| E8 | DMA CHAIN_TO might not fire for zero-length transfers | A2-A4 | Software only | firmware rule: no zero-length chained transfers |
| E9 | Bank 0 GPIO leakage holds an input with IE set near 2.2 V; the pull-down cannot pull it low | A2 | **Yes, if A2.** Hardware: PWR_HOLD has R42 4.7k and CHG_OFF has R44 4.7k, both at or under 8.2 kohm (rp2350.txt:102915), so the latch and the charger's CE cannot be held up (paper_checks.py latch, VER-33 PASS). AMP_EN has R37 100k: E9 could hold it near 2.2 V only if firmware leaves it an input with IE set; it does not after a PoR or RUN reset (rp2350.txt:102952), and a watchdog reset resets the pads when PSM WDSEL includes RESETS, as the SDK's watchdog_enable() sets (paper_checks.py ampstate, VER-196 PASS). The SWD pads are pulled up (no ill effect, rp2350.txt:102958). The domes and buttons read with pull-ups, which E9 does not affect (rp2350.txt:102922) | fix in copper and BOM: R42, R44. Firmware rules (pinmap.md, Power control): AMP_EN driven low before sleep and never left an input; keep the SDK's watchdog_enable() |
| E10 | UF2 drag-and-drop fails with partition tables | A2 | Software only: if a partition table is used | picotool's workaround (rp2350.txt:102206) |
| E11 | XIP cache clean by set/way modifies tags of dirty lines | A2-A4 | Software only: the firmware runs from the in-package flash, no cache maintenance by set/way | none |
| E12 | USB status signals not synchronised when clk_sys <= clk_usb | A2-A4 (mitigated A3) | **Yes, software:** the game uses USB (logs, the bootloader) | firmware rule: clk_sys at least 10% faster than clk_usb (150 MHz against 48 MHz: met by the SDK default) |
| E13 | Invalid IMAGE_DEF before a valid one fails to boot | A2 | Software only: the build writes one image | none |
| E14 | connect_internal_flash() ignores FLASH_DEVINFO CS1 | A2 | No: the RP2354B's flash is on CS0 | none |
| E15 | otp_access() wrong permissions on pages 62, 63 | A2 | No: no OTP access from firmware | none |
| E16 | USB_OTP_VDD disruption can corrupt OTP row reads | A2 | **Yes, if A2,** and only through boot's OTP reads: USB_OTP_VDD is +3V3 (pin 68; C24 at 1.9 mm, paper_checks.py core). The firmware writes no OTP and enables no secure boot, so a corrupt read can only change boot flags that stay at their defaults | none on the board (no workaround exists, rp2350.txt:103155); firmware rule: write no OTP, keep secure boot off |
| E17 | Guarded read of a single ECC OTP row faults if the next row is not valid ECC | A2-A4 | No: no OTP reads from firmware | none |
| E18 | Boot fails forever if FLASH_PARTITION_SLOT_SIZE holds a bad ECC pattern | A2, A3 | No: that OTP row is never written | firmware rule: write no OTP |
| E19 | Reboot hangs if certain FRCE_OFF bits are set | A2 | Software only | none: the firmware does not use FRCE_OFF |
| E20 | Glitch attack on secure boot (reboot) | A2 | No: no secure boot | none |
| E21 | Glitch attack on OTP in BOOTSEL | A2 | No: no secrets in OTP | none |
| E22 | Malformed lollipop block loop hangs | A2 | Software only: the build writes well-formed blocks | none |
| E23 | PICOBOOT GET_INFO returns zero for PACKAGE_SEL | A2 | No: the tools do not rely on it | none |
| E24 | Glitch attack on secure boot | A2, A3 | No: no secure boot | none |
| E25 | LOAD_MAP with non-word sizes not rejected | A2, A3 | Software only | none |
| E26 | RCP random delays can create a side channel | A2-A4 | No: no secrets | none |
| E27 | Bus priority controls apply to the wrong managers on APB and FASTPERI | A2-A4 | Software only: only if the firmware sets bus priority | firmware rule: leave bus priority at reset, or set it per the datasheet's corrected table |
| E28 | OTP keys for pages 62/63 applied to all lock words | A2-A4 | No: no OTP keys | none |

## Result

Every one of the 28 errata is answered. Three can touch this design: E9 and E16 if the parts are
A2, and E12 on any stepping. E9 is fixed in the board (R42, R44) and by two firmware rules. E12
and E16 are firmware rules only. Every row that applies has a fix or a rule; none needs a waiver.

Three firmware rules come out of this table and belong in pinmap.md's Power control section
(DEF-21's doc sweep carries them): AMP_EN never left an input, the SDK's watchdog_enable() kept,
and no OTP writes. The stepping is read at VER-109 and written in the table's heading.

RESULT: PASS
