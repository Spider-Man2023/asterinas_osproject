use alloc::{
    boxed::Box,
    vec,
};
use log::debug;
use core::{
    hint::spin_loop,
    ops::Range,
};
use super::{
    config::{VirtioGPUConfig, GPUFeatures}, 
    header::{
        VirtioGpuCtrlHdr,
        VirtioGpuCtrlType,
        HDR_LEN,
        VirtioGpuRect,
        VirtioGpuRespDisplayInfo,
        GET_DISPLAY_INFO_LEN,
        VirtioGpuGetEdid,
        GET_EDID_LEN,
        VirtioGpuRespEDID,
        RESP_EDID_LEN,
        VirtioGpuFormats,
        VirtioGpuResourceCreate2d,
        CREATE_RESOURCE_LEN,
        VirtioGpuResourceAttachBacking,
        VirtioGpuMemEntry,
        ATTACH_RESOURCE_LEN,
        ATTACH_RESOURCE_ENTRY_LEN,
        VirtioGpuSetScanout,
        SET_SCANOUT_LEN,
        VirtioGpuTransferToHost2d,
        TRANSFER_TO_HOST_LEN,
        VirtioGpuResourceFlush,
        FLUSH_RESOURCE_LEN,
    }, 
    DEVICE_NAME
};
use ostd::{
    early_println, mm::{
        DmaDirection, DmaStream, DmaStreamSlice, FrameAllocOptions, VmIo
    }, sync::SpinLock
};

use crate::{
    device::VirtioDeviceError,
    queue::VirtQueue,
    transport::{ConfigManager, VirtioTransport},
};

pub struct GPUDevice {
    config_manager: ConfigManager<VirtioGPUConfig>,
    transport: SpinLock<Box<dyn VirtioTransport>>,
    control_queue: SpinLock<VirtQueue>, //控制命令
    cursor_queue: SpinLock<VirtQueue>, //光标更新
    control_buffer: DmaStream,
    cursor_buffer: DmaStream,
}


impl GPUDevice {
    pub fn negotiate_features(features: u64) -> u64 {
        features
    }

    pub fn init(mut transport: Box<dyn VirtioTransport>) -> Result<(), VirtioDeviceError> {
        let config_manager = VirtioGPUConfig::new_manager(transport.as_ref());
        let features = GPUFeatures::from_bits_truncate(Self::negotiate_features(
            transport.read_device_features(),
        ));
        early_println!("virtio_gpu_config = {:?}", config_manager.read_config());
        early_println!("gpu_feature = {:?}", features);

        const CONTROL_QUEUE_INDEX: u16 = 0;
        const CURSOR_QUEUE_INDEX: u16 = 1;
        let control_queue = SpinLock::new(VirtQueue::new(CONTROL_QUEUE_INDEX, 128, transport.as_mut()).unwrap());
        let cursor_queue = SpinLock::new(VirtQueue::new(CURSOR_QUEUE_INDEX, 128, transport.as_mut()).unwrap());

        // 初始化 DMA 缓冲区
        let control_buffer = {
            let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
            DmaStream::map(segment.into(), DmaDirection::Bidirectional, false).unwrap()
        };
        let cursor_buffer = {
            let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
            DmaStream::map(segment.into(), DmaDirection::Bidirectional, false).unwrap()
        };

        let mut device = GPUDevice {
            config_manager,
            transport: SpinLock::new(transport),
            control_queue,
            cursor_queue,
            control_buffer,
            cursor_buffer,
        };

        // 5.7.5
        early_println!("————————5.7.5————————");
        early_println!("    ————Get Display Info————");
        let result = device.get_display_info();
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        let mut width: u32 = 1024;
        let mut height: u32 = 768;
        match result {
            Ok(rect) => {
                x = rect.x;
                y = rect.y;
                width = rect.width;
                height = rect.height;
            },
            Err(e) => {
                early_println!("    Failed to get display info: {:?}", e);
            },
        }
        early_println!("    Display x: {}", x);
        early_println!("    Display y: {}", y);
        early_println!("    Display width: {}", width);
        early_println!("    Display height: {}", height);

        early_println!("    ————Get EDID————");
        let result = device.get_edid();
        match result {
            Ok(edid_msg) => {
                early_println!("    Display size: {}", edid_msg.size);
                early_println!("    Display padding: {}", edid_msg.padding);
                // early_println!("    Display edid: {:?}", edid_msg.edid);
            },
            Err(e) => {
                early_println!("    Failed to get display info: {:?}", e);
            },
        }

        // 5.7.6.1
        early_println!("————————5.7.6.1————————");
        // 为显示创建缓冲
        early_println!("    ————Create Resource————");
        let resp = device.create_resource(width, height);
        match resp {
            Ok(type_) => {
                early_println!("    Response type: {:x}", type_);
            },
            Err(e) => {
                early_println!("    ailed to get response: {:?}", e);
            },
        }
        // 缓冲绑定内存
        early_println!("    ————Attach Resource————");
        let resp = device.attach_resource(width, height); 
        match resp {
            Ok(type_) => {
                early_println!("    Response type: {:x}", type_);
            },
            Err(e) => {
                early_println!("    Failed to get response: {:?}", e);
            },
        }
        // 绑定显示输出
        early_println!("    ————Set Scanout————");
        let resp = device.set_scanout(x, y, width, height); 
        match resp {
            Ok(type_) => {
                early_println!("    Response type: {:x}", type_);
            },
            Err(e) => {
                early_println!("    Failed to get response: {:?}", e);
            },
        }
        // 传输显示数据
        early_println!("    ————Transfer to Host————");
        let resp = device.transfer_to_host(x, y, width, height); 
        match resp {
            Ok(type_) => {
                early_println!("    Response type: {:x}", type_);
            },
            Err(e) => {
                early_println!("    Failed to get response: {:?}", e);
            },
        }
        // 刷新显示
        early_println!("    ————Flush Resource————");
        let resp = device.flush_resource(x, y, width, height); 
        match resp {
            Ok(type_) => {
                early_println!("    Response type: {:x}", type_);
            },
            Err(e) => {
                early_println!("    Failed to get response: {:?}", e);
            },
        }
        early_println!("————————GPU End————————");
        Ok(())
    }

    // 5.7.5
    fn get_display_info(&mut self) -> Result<VirtioGpuRect, VirtioDeviceError> {
        //询问信息
        let req_slice = {
            let req_slice = DmaStreamSlice::new(&self.control_buffer, 0, HDR_LEN);
            // 只发报头，无需数据
            let req = VirtioGpuCtrlHdr {
                type_: VirtioGpuCtrlType::VIRTIO_GPU_CMD_GET_DISPLAY_INFO as u32,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                ring_idx: 0,
                padding: [0; 3],
            };
            req_slice.write_val(0, &req).unwrap();
            req_slice.sync().unwrap();
            req_slice
        };
        
        // 创建临时接收buffer
        let resp_buffer = {
            let segment = FrameAllocOptions::new()
                .zeroed(false)
                .alloc_segment(1)
                .unwrap();
            DmaStream::map(segment.into(), DmaDirection::FromDevice, false).unwrap()
        };
        let resp_slice = DmaStreamSlice::new(&resp_buffer, 0, GET_DISPLAY_INFO_LEN);
        
        // 绑定buffer并发送
        self.control_queue.disable_irq().lock()
            .add_dma_buf(&[&req_slice], &[&resp_slice])
            .expect("GPU query failed");
        if self.control_queue.lock().should_notify() {
            self.control_queue.lock().notify();
        }
        while !self.control_queue.lock().can_pop() {
            spin_loop();
        }
        let _ = self.control_queue.lock().pop_used();
        resp_buffer.sync(0..GET_DISPLAY_INFO_LEN).unwrap();

        // 接收消息
        resp_slice.sync().unwrap();
        let resp_msg: VirtioGpuRespDisplayInfo = resp_slice.read_val(0).unwrap();
        Ok(resp_msg.pmodes[0].r)
    }

    fn get_edid(&mut self) -> Result<VirtioGpuRespEDID, VirtioDeviceError> {
        let req_slice = {
            let req_slice = DmaStreamSlice::new(&self.control_buffer, 0, GET_EDID_LEN);
            // 构造数据包
            let req = VirtioGpuGetEdid {
                hdr: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VIRTIO_GPU_CMD_GET_EDID as u32,
                    flags: 0,
                    fence_id: 0,
                    ctx_id: 0,
                    ring_idx: 0,
                    padding: [0; 3],
                },
                scanout: 0,
                padding: 0,
            };
            req_slice.write_val(0, &req).unwrap();
            req_slice.sync().unwrap();
            req_slice
        };
        
        // 创建临时接收buffer
        let resp_buffer = {
            let segment = FrameAllocOptions::new()
                .zeroed(false)
                .alloc_segment(1)
                .unwrap();
            DmaStream::map(segment.into(), DmaDirection::FromDevice, false).unwrap()
        };
        
        // 创建临时接收buffer
        let resp_buffer = {
            let segment = FrameAllocOptions::new()
                .zeroed(false)
                .alloc_segment(1)
                .unwrap();
            DmaStream::map(segment.into(), DmaDirection::FromDevice, false).unwrap()
        };
        let resp_slice = DmaStreamSlice::new(&resp_buffer, 0, RESP_EDID_LEN);
        
        // 绑定buffer并发送
        self.control_queue.disable_irq().lock()
            .add_dma_buf(&[&req_slice], &[&resp_slice])
            .expect("GPU query failed");
        if self.control_queue.lock().should_notify() {
            self.control_queue.lock().notify();
        }
        while !self.control_queue.lock().can_pop() {
            spin_loop();
        }
        let _ = self.control_queue.lock().pop_used();
        resp_buffer.sync(0..RESP_EDID_LEN).unwrap();

        // 接收消息
        resp_slice.sync().unwrap();
        let resp_msg: VirtioGpuRespEDID = resp_slice.read_val(0).unwrap();
        Ok(resp_msg)
    }

    // 5.7.6.1
    fn create_resource(&mut self, width: u32, height: u32) -> Result<u32, VirtioDeviceError> {
        let req_slice = {
            let req_slice = DmaStreamSlice::new(&self.control_buffer, 0, CREATE_RESOURCE_LEN);
            // 构造数据包
            let req = VirtioGpuResourceCreate2d {
                hdr: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VIRTIO_GPU_CMD_RESOURCE_CREATE_2D as u32,
                    flags: 0,
                    fence_id: 0,
                    ctx_id: 0,
                    ring_idx: 0,
                    padding: [0; 3],
                }, 
                resource_id: 1, 
                format: VirtioGpuFormats::VIRTIO_GPU_FORMAT_A8R8G8B8_UNORM as u32,
                width, 
                height,
            };
            req_slice.write_val(0, &req).unwrap();
            req_slice.sync().unwrap();
            req_slice
        };
        
        // 创建临时接收buffer
        let resp_buffer = {
            let segment = FrameAllocOptions::new()
                .zeroed(false)
                .alloc_segment(1)
                .unwrap();
            DmaStream::map(segment.into(), DmaDirection::FromDevice, false).unwrap()
        };
        let resp_slice = DmaStreamSlice::new(&resp_buffer, 0, HDR_LEN);
        
        // 绑定buffer并发送
        self.control_queue.disable_irq().lock()
            .add_dma_buf(&[&req_slice], &[&resp_slice])
            .expect("GPU query failed");
        if self.control_queue.lock().should_notify() {
            self.control_queue.lock().notify();
        }
        while !self.control_queue.lock().can_pop() {
            spin_loop();
        }
        let _ = self.control_queue.lock().pop_used();
        resp_buffer.sync(0..HDR_LEN).unwrap();

        // 接收消息
        resp_slice.sync().unwrap();
        let resp_msg: VirtioGpuCtrlHdr = resp_slice.read_val(0).unwrap();
        Ok(resp_msg.type_)
    }

    fn attach_resource(&mut self, width: u32, height: u32) -> Result<u32, VirtioDeviceError> {
        let req_slice = {
            let req_slice = DmaStreamSlice::new(&self.control_buffer, 0, ATTACH_RESOURCE_LEN+ATTACH_RESOURCE_ENTRY_LEN);
            // 构造数据包
            let req = VirtioGpuResourceAttachBacking {
                hdr: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING as u32,
                    flags: 0,
                    fence_id: 0,
                    ctx_id: 0,
                    ring_idx: 0,
                    padding: [0; 3],
                }, 
                resource_id: 1,
                nr_entries: 1,
            };
            req_slice.write_val(0, &req).unwrap();
            let segment = FrameAllocOptions::new().alloc_segment(1024).unwrap();            
            let addr = segment.start_paddr() as u64;
            let length = segment.size() as u32;
            let buffer = DmaStream::map(segment.into(), DmaDirection::Bidirectional, false).unwrap();
            let slice = DmaStreamSlice::new(&buffer, 0, length as usize);
            // 填充内存段
            let white_pixel = 0xFFFFFFFFu32;
            let red_pixel = 0xFFFF0000u32;
            let green_pixel = 0xFF00FF00u32;
            let yellow_pixel = 0xFFFFFF00u32;
            let blue_pixel = 0xFF0000FFu32;
            let cyan_pixel = 0xFF00FFFFu32;
            // 白色横线
            for i in ((width*4*200) as usize..(width*4*210) as usize).step_by(4) {
                slice.write_val(i, &white_pixel).unwrap();
            }
            // 红色竖线
            for i in (500 as usize..((width-1)*height*4+500) as usize).step_by((width*4) as usize) {
                slice.write_val(i, &red_pixel).unwrap();
            }
            slice.sync().unwrap();

            let entry = VirtioGpuMemEntry  {
                addr: addr,
                length: length,
                padding: 0,
            };
            req_slice.write_val(ATTACH_RESOURCE_LEN, &entry).unwrap();
            req_slice.sync().unwrap();
            req_slice
        };
        
        // 创建临时接收buffer
        let resp_buffer = {
            let segment = FrameAllocOptions::new()
                .zeroed(false)
                .alloc_segment(1)
                .unwrap();
            DmaStream::map(segment.into(), DmaDirection::FromDevice, false).unwrap()
        };
        let resp_slice = DmaStreamSlice::new(&resp_buffer, 0, HDR_LEN);
        
        // 绑定buffer并发送
        self.control_queue.disable_irq().lock()
            .add_dma_buf(&[&req_slice], &[&resp_slice])
            .expect("GPU query failed");
        if self.control_queue.lock().should_notify() {
            self.control_queue.lock().notify();
        }
        while !self.control_queue.lock().can_pop() {
            spin_loop();
        }
        let _ = self.control_queue.lock().pop_used();
        resp_buffer.sync(0..HDR_LEN).unwrap();

        // 接收消息
        resp_slice.sync().unwrap();
        let resp_msg: VirtioGpuCtrlHdr = resp_slice.read_val(0).unwrap();
        Ok(resp_msg.type_)
    }

    fn set_scanout(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<u32, VirtioDeviceError> {
        let req_slice = {
            let req_slice = DmaStreamSlice::new(&self.control_buffer, 0, SET_SCANOUT_LEN);
            // 构造数据包
            let req = VirtioGpuSetScanout {
                hdr: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VIRTIO_GPU_CMD_SET_SCANOUT as u32,
                    flags: 0,
                    fence_id: 0,
                    ctx_id: 0,
                    ring_idx: 0,
                    padding: [0; 3],
                }, 
                r: VirtioGpuRect {
                    x: x,
                    y: y,
                    width,
                    height,
                },
                scanout_id: 0, 
                resource_id: 1,
            };
            req_slice.write_val(0, &req).unwrap();
            req_slice.sync().unwrap();
            req_slice
        };
        // 创建临时接收buffer
        let resp_buffer = {
            let segment = FrameAllocOptions::new()
                .zeroed(false)
                .alloc_segment(1)
                .unwrap();
            DmaStream::map(segment.into(), DmaDirection::FromDevice, false).unwrap()
        };
        let resp_slice = DmaStreamSlice::new(&resp_buffer, 0, HDR_LEN);
        
        // 绑定buffer并发送
        self.control_queue.disable_irq().lock()
            .add_dma_buf(&[&req_slice], &[&resp_slice])
            .expect("GPU query failed");
        if self.control_queue.lock().should_notify() {
            self.control_queue.lock().notify();
        }
        while !self.control_queue.lock().can_pop() {
            spin_loop();
        }
        let _ = self.control_queue.lock().pop_used();
        resp_buffer.sync(0..HDR_LEN).unwrap();

        // 接收消息
        resp_slice.sync().unwrap();
        let resp_msg: VirtioGpuCtrlHdr = resp_slice.read_val(0).unwrap();
        Ok(resp_msg.type_)
    }

    // 5.7.6.2
    fn transfer_to_host(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<u32, VirtioDeviceError> {
        let req_slice = {
            let req_slice = DmaStreamSlice::new(&self.control_buffer, 0, TRANSFER_TO_HOST_LEN);
            // 构造数据包
            let req = VirtioGpuTransferToHost2d {
                hdr: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D as u32,
                    flags: 0,
                    fence_id: 0,
                    ctx_id: 0,
                    ring_idx: 0,
                    padding: [0; 3],
                }, 
                r: VirtioGpuRect {
                    x: x,
                    y: y,
                    width,
                    height,
                },
                offset: 0,
                resource_id: 1,
                padding: 0,
            };
            req_slice.write_val(0, &req).unwrap();
            req_slice.sync().unwrap();
            req_slice
        };
        // 创建临时接收buffer
        let resp_buffer = {
            let segment = FrameAllocOptions::new()
                .zeroed(false)
                .alloc_segment(1)
                .unwrap();
            DmaStream::map(segment.into(), DmaDirection::FromDevice, false).unwrap()
        };
        let resp_slice = DmaStreamSlice::new(&resp_buffer, 0, HDR_LEN);
        
        // 绑定buffer并发送
        self.control_queue.disable_irq().lock()
            .add_dma_buf(&[&req_slice], &[&resp_slice])
            .expect("GPU query failed");
        if self.control_queue.lock().should_notify() {
            self.control_queue.lock().notify();
        }
        while !self.control_queue.lock().can_pop() {
            spin_loop();
        }
        let _ = self.control_queue.lock().pop_used();
        resp_buffer.sync(0..HDR_LEN).unwrap();

        // 接收消息
        resp_slice.sync().unwrap();
        let resp_msg: VirtioGpuCtrlHdr = resp_slice.read_val(0).unwrap();
        Ok(resp_msg.type_)
    }

    fn flush_resource(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<u32, VirtioDeviceError> {
        let req_slice = {
            let req_slice = DmaStreamSlice::new(&self.control_buffer, 0, FLUSH_RESOURCE_LEN);
            // 构造数据包
            let req = VirtioGpuResourceFlush {
                hdr: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VIRTIO_GPU_CMD_RESOURCE_FLUSH as u32,
                    flags: 0,
                    fence_id: 0,
                    ctx_id: 0,
                    ring_idx: 0,
                    padding: [0; 3],
                }, 
                r: VirtioGpuRect {
                    x: x,
                    y: y,
                    width,
                    height,
                },
                resource_id: 1,
                padding: 0,
            };
            req_slice.write_val(0, &req).unwrap();
            req_slice.sync().unwrap();
            req_slice
        };
        // 创建临时接收buffer
        let resp_buffer = {
            let segment = FrameAllocOptions::new()
                .zeroed(false)
                .alloc_segment(1)
                .unwrap();
            DmaStream::map(segment.into(), DmaDirection::FromDevice, false).unwrap()
        };
        let resp_slice = DmaStreamSlice::new(&resp_buffer, 0, HDR_LEN);
        
        // 绑定buffer并发送
        self.control_queue.disable_irq().lock()
            .add_dma_buf(&[&req_slice], &[&resp_slice])
            .expect("GPU query failed");
        if self.control_queue.lock().should_notify() {
            self.control_queue.lock().notify();
        }
        while !self.control_queue.lock().can_pop() {
            spin_loop();
        }
        let _ = self.control_queue.lock().pop_used();
        resp_buffer.sync(0..HDR_LEN).unwrap();

        // 接收消息
        resp_slice.sync().unwrap();
        let resp_msg: VirtioGpuCtrlHdr = resp_slice.read_val(0).unwrap();
        Ok(resp_msg.type_)
    }

    // 尝试发送头部
    fn send_recv_ctrl(&mut self, ctrl_type: VirtioGpuCtrlType) -> Result<(), VirtioDeviceError> {
        let req_slice = {
            let req_slice = DmaStreamSlice::new(&self.control_buffer, 0, HDR_LEN);
            let req = VirtioGpuCtrlHdr {
                type_: ctrl_type as u32,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                ring_idx: 0,
                padding: [0; 3],
            };
            req_slice.write_val(0, &req).unwrap();
            req_slice.sync().unwrap();
            req_slice
        };
        
        // 创建临时接收buffer
        let display_info_stream = {
            let segment = FrameAllocOptions::new()
                .zeroed(false)
                .alloc_segment(1)
                .unwrap();
            DmaStream::map(segment.into(), DmaDirection::FromDevice, false).unwrap()
        };
        let display_info_slice = DmaStreamSlice::new(&display_info_stream, 0, HDR_LEN);
        
        // 绑定buffer并发送
        self.control_queue.disable_irq().lock()
            .add_dma_buf(&[&req_slice], &[&display_info_slice])
            .expect("GPU query failed");
        if self.control_queue.lock().should_notify() {
            self.control_queue.lock().notify();
        }
        while !self.control_queue.lock().can_pop() {
            spin_loop();
        }
        let _ = self.control_queue.lock().pop_used();
        display_info_stream.sync(0..24).unwrap();

        // 接收消息
        // let value = display_info_stream.reader().unwrap().read_once::<u64>().unwrap();
        display_info_slice.sync().unwrap();
        let display_info: VirtioGpuCtrlHdr = display_info_slice.read_val(0).unwrap();
        early_println!("————————Display Info————————");
        early_println!("Display type:{:x}", display_info.type_);
        early_println!("Display flags:{:x}", display_info.flags);
        early_println!("Display fence_id:{:x}", display_info.fence_id);
        early_println!("Display ctx_id:{:x}", display_info.ctx_id);
        early_println!("Display ring_idx:{:x}", display_info.ring_idx);
        // early_println!("Display padding:{:x}", display_info.padding);
        // 有消息按消息设置，否则默认
        Ok(())
    }
}
