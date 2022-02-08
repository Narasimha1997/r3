# r3
A hobby x86_64 Operating System kernel written in Rust -- with minimal functionalities like preemptive and cooperative multi-tasking, usermode, framebuffer terminal, basic device drivers like -- ATA-PIO (for Disk I/O), PS/2 Keyboard, Serial UART, RTL8139 networking (in progress), file-system built with VFS that supports devfs and ustar, last but not the least - supports somewhat POSIXish basic system calls. 

The name `r3` stands for `Revision 3` because this is my third attempt to build a hobby Operating System kernel. The kernel barely works on QEMU as of now and some of the functionalities are not optimal or not perfectly implemented. This is an attempt to keep myself occupied, I am not competing with Linux or other rusty operating systems here on GitHub - in fact I got inspired from these operating systems and read their code-bases to understand various possible methods. (I have given credits to some of the rust OS repositories I looked at.)

### Functionalities supported as of now in the kernel:
1. Basic x86 Interrupts - both exceptions and hardware interrupts.
2. Programmable I/O
3. Descriptor tables like GDT, IDT and TSS
4. Legacy Programmable Interrupt Controller - (PIC) used for handling interrupts initially
5. Legacy Programmable Interval Timer - (PIT) used for initial CPU frequency detection
6. CPUID - CPU features identification
7. Timestamp Counter (TSC) which is used as the core system timer for events generation
8. Linear Physical Memory Manager - Right not it only allocates, never deallocates (LoL this is the biggest drawback as of now, anyways I am planning to write a slab allocator soon)
9. DMA Manager - Allocates DMA memory regions for drivers like RTL8139
10. Virtual Memory Manager and Paging - Allocate and manage both 4KB pages and 2MB huge-pages.
11. Heap Allocator built on top of VMA, this heap allocator is used by rust to manage it's dynamic structures -- uses a linked list allocator underneath.
12. Higher half kernel
13. ACPI and ACPI x2 support
14. Local APIC Interrupts support
15. Symmetric Multi-Processing (SMP) - still in very initial stages (in progress)
16. Peripheral Component Interconnect - legacy mode with Configuration Registers
17. PS/2 Keyboard Driver
18. UART serial interface driver
19. ATA Disk controller - using PIO mode, where data-transfers take CPU cycles. (DMA mode is in planning stage yet)
20. Framebuffer display
21. Random Numbers generator
22. TTY interface using PS/2 and Framebuffer together
23. VFS - Virtual File System implementation (Supports only crucial functionalities as of now)
24. Devfs - to manage devices as files (good old UNIX concept - everything is a file)
25. USTAR - A simple TAR file-system with read-only capabilities
26. System timer and ticks built on top of TSC and LAPIC interrupts
27. Multi-processing - ability to run multiple-processes concurrently (not parallely as of now)
28. Multi-threading - Still in early stages, as of now, a process can have only one thread
29. Software based context switching - ability to save and restore thread states
30. Mode switching - From kernel-mode to user-mode and vice-versa
31. Non-blocking I/O - for keyboard as of now (will implement the same for networking and disk I/O soon)
32. Sysv64 ABI for making System calls
33. System calls interface - uses legacy/portable `int 0x80` software based mechanism. 
34. Basic system calls like - `read`, `write`, `open`, `close`, `exit`, `fstat`, `lstat`, `lseek`, `getpid`, `getppid`, `fork`, `brk`, `sbrk`, `ioctl`, `yield`, `gettid`, `sleep`, `wait`, `shutdown`, `reboot`, `execvp`, `uname`, `getrandom`, `gettime` are implemented, these implementations barely work and are not perfectly POSIX. 
35.  Ability to load ELF files from the file-system and execute them as a process - by following the ELF process layout.
36. Basic networking - (Still in initial stages of development, as of now you can see a half-baked RTL8139 driver implementation)
36. Internal kernel logging via serial port used for debugging

### Userland:
The userland as of now is just a simple program that reads and writes whatever it read to TTY. 
More features are coming to userland once I finish pending kernel stuff. I have plans to port some C libraries like [newlib](https://sourceware.org/newlib/) and write a simple system-calls wrapper in Rust.

In fact, the documentation is yet to be written on how to port libraries on-top this kernel's system-calls (because I am not clear yet about what works and what might not work haha)

### Some third party libraries:
1. [Rust Bootloader](https://github.com/rust-osdev/bootloader) - Initial bootloader
2. [Rust logger](https://github.com/rust-lang/log) - for internal logging on top of UART
3. [spin](https://github.com/mvdnes/spin-rs) - For `Mutex` locks
4. [lazy_static](https://github.com/rust-lang-nursery/lazy-static.rs) - For runtime lazy initialization of static data
5. [bit_field](https://github.com/phil-opp/rust-bit-field) - For basic bit level operations
6. [bitflags](https://github.com/bitflags/bitflags) - For enum-like abstractions over primitive data-types.
7. [object](https://github.com/gimli-rs/object) - For parsing raw and ELF binaries
8. [pc-keyboard](https://github.com/rust-embedded-community/pc-keyboard) - For parsing and decoding raw key-events
9. [rand_xoshiro](https://github.com/rust-random/rngs/tree/master/rand_xoshiro) - Rust implementation of Non-cryptographic randmon number generation algorithm called [xoshiro128+](https://en.wikipedia.org/wiki/Xoroshiro128%2B)
10. [linked_list_allocator](https://github.com/phil-opp/linked-list-allocator) - Heap memory management using linked-list data-structure on-top of raw memory region

### Build and run:
1. Install all the presequites - QEMU, KVM, rust, cargo, xbuild and OVMF
```
./tools/setup_env.sh
```
2. Build the userspace tarfs (requires GCC)
```
./tools/build_tarfs.sh
```
3. Build and run the Kernel:
```
# with the tar-file mounted
./tools/run_qemu_disk.sh

# without tar-file mounted (will crash anyways lol)
./tools/run_qemu.sh
```

### Debugging
The emulator will generate a `serial.out` file to dump all the logs, also QEMU's debug panel will be launched just after starting the boot.

### Existing Projects:
1. [x86_64](https://github.com/rust-osdev/x86_64)
2. [moros](https://github.com/vinc/moros)
3. [kerla](https://github.com/nuta/kerla)
4. [rust_os](https://github.com/Dentosal/rust_os)
5. [rust_os](https://github.com/thepowersgang/rust_os)

Credis to the awesome [rust-osdev](https://os.phil-opp.com/) series by  [Philipp Oppermann](https://github.com/phil-opp) and [OSDev](https://wiki.osdev.org/Expanded_Main_Page) community.

### Demo:
ECHO client user-program running on QEMU (more to come)
![ECHO](https://i.ibb.co/mTy2cDf/sc.png)

### Contributing to R3:
Any contributions in the form of code, issues, discussions are welcome!