

LEGACY_INTERRUPTS_BASE = 0x20
MAX_ARCH_INTERRUPTS = 256

entries = []

writer = open('entries.txt', 'w')
for idx in range(1, MAX_ARCH_INTERRUPTS - LEGACY_INTERRUPTS_BASE):
    entry = "IDT.lock().interrupts[LEGACY_HARDWARE_INTERRUPTS_BASE + {}] = prepare_no_irq_handler!(no_irq_fn, {});\n".format(
        hex(idx), hex(LEGACY_INTERRUPTS_BASE + idx)
    )
    writer.write(entry)