// SPDX-License-Identifier: MPL-2.0

pub mod config;
pub mod device;
pub mod header;

pub static DEVICE_NAME: &str = "Virtio-GPU";
pub const VIRTIO_GPU_CONTEXT_INIT_CAPSET_ID_MASK: u32 = 0x000000ff;
