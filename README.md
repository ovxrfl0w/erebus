# Erebus

Erebus is a kernel-mode driver written using the official [windows-drivers-rs] crate. It exposes two IOCTL
actions for user-mode programs. One to read process memory and the other to write to process memory. It uses
`MmCopyVirtualMemory` to read/write process memory.

## Why?

This project was mainly created just to somewhat familiarize myself with Windows kernel driver development in Rust using
the [windows-drivers-rs] crate.

## Special Thanks

- 0xflux (for the [blog posts] related to building a Rust driver using [windows-drivers-rs])

## LICENSE

See [LICENSE.md](LICENSE.md)

[windows-drivers-rs]: https://github.com/microsoft/windows-drivers-rs
[blog posts]: https://fluxsec.red/rust-windows-driver