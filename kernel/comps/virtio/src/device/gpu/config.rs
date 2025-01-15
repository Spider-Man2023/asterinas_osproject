use core::mem::offset_of;

use aster_util::safe_ptr::SafePtr;
use ostd::Pod;

use crate::transport::{ConfigManager, VirtioTransport};

bitflags::bitflags! {
    pub struct GPUFeatures: u64{
        const VIRTIO_GPU_F_VIRGL = 1 << 0;
        const VIRTIO_GPU_F_EDID = 1 << 1;
        const VIRTIO_GPU_F_RESOURCE_UUID = 1 << 2;
        const VIRTIO_GPU_F_RESOURCE_BLOB = 1 << 3;
        const VIRTIO_GPU_F_CONTEXT_INIT = 1 << 4;
    }
}

// 5.7.4
const VIRTIO_GPU_EVENT_DISPLAY: u32 = 1 << 0;
#[derive(Debug, Pod, Clone, Copy)]
#[repr(C)]
pub struct VirtioGPUConfig {
    pub events_read: u32, //驱动不能写入
    pub events_clear: u32, //清空等待事件
    pub num_scanouts: u32, //输出数量[1, 16]
    pub num_capsets: u32, // >= 0
}

impl VirtioGPUConfig {
    pub(super) fn new_manager(transport: &dyn VirtioTransport) -> ConfigManager<Self> {
        let safe_ptr = transport
            .device_config_mem()
            .map(|mem| SafePtr::new(mem, 0));
        let bar_space = transport.device_config_bar();
        ConfigManager::new(safe_ptr, bar_space)
    }
}

impl ConfigManager<VirtioGPUConfig> {
    pub(super) fn read_config(&self) -> VirtioGPUConfig {
        let mut gpu_config = VirtioGPUConfig::new_uninit();
        gpu_config.events_read = self
            .read_once::<u32>(offset_of!(VirtioGPUConfig, events_read))
            .unwrap();
        gpu_config.events_clear = self
            .read_once::<u32>(offset_of!(VirtioGPUConfig, events_clear))
            .unwrap();
        gpu_config.num_scanouts = self
            .read_once::<u32>(offset_of!(VirtioGPUConfig, num_scanouts))
            .unwrap();
        gpu_config.num_capsets = self
            .read_once::<u32>(offset_of!(VirtioGPUConfig, num_capsets))
            .unwrap();
        
        gpu_config
    }
}