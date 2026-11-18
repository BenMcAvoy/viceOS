# viceOS

viceOS is a hobby operating-system project written in Rust for x86_64. It boots a `no_std` kernel through a Multiboot 2 boot path and is a place to experiment with architecture setup, memory management, framebuffer output and process structures.

## Build and run

The Makefile targets a Linux development environment. Install the Rust toolchain selected by `rust-toolchain.toml`, NASM, `x86_64-elf-ld`, GRUB rescue tools and QEMU. Then:

```sh
make iso
make run-gui
```

`make kernel` builds the kernel only. The run target boots the generated image in QEMU.

## Status

This is an experimental kernel, not a complete operating system. Many driver, process and user-mode components are still under development.
