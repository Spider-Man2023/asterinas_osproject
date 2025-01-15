use bitflags::bitflags;
use int_to_c_enum::TryFromInt;
use ostd::Pod;

const VIRTIO_GPU_FLAG_FENCE: u32 = 1 << 0;
const VIRTIO_GPU_FLAG_INFO_RING_IDX: u32 = 1 << 1;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuCtrlHdr {
    pub type_: u32, // 声明请求类型：VIRTIO_GPU_CMD_*或VIRTIO_GPU_RESP_*.
    pub flags: u32, // requst/response
    pub fence_id: u64, // 
    pub ctx_id: u32, // 仅3D模式
    pub ring_idx: u8,
    pub padding: [u8; 3],
}

pub const HDR_LEN: usize = size_of::<VirtioGpuCtrlHdr>();

#[repr(u32)]
#[derive(Default, Debug, Clone, Copy, TryFromInt)]
#[allow(non_camel_case_types)]
pub enum VirtioGpuCtrlType {
    #[default]
    /* 2d commands */
    VIRTIO_GPU_CMD_GET_DISPLAY_INFO = 0x0100,
    VIRTIO_GPU_CMD_RESOURCE_CREATE_2D = 0x0101,
    VIRTIO_GPU_CMD_RESOURCE_UNREF = 0x0102,
    VIRTIO_GPU_CMD_SET_SCANOUT = 0x0103,
    VIRTIO_GPU_CMD_RESOURCE_FLUSH = 0x0104,
    VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D = 0x0105,
    VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING = 0x0106,
    VIRTIO_GPU_CMD_RESOURCE_DETACH_BACKING = 0x0107,
    VIRTIO_GPU_CMD_GET_CAPSET_INFO = 0x0108,
    VIRTIO_GPU_CMD_GET_CAPSET = 0x0109,
    VIRTIO_GPU_CMD_GET_EDID = 0x010A,
    VIRTIO_GPU_CMD_RESOURCE_ASSIGN_UUID = 0x010B,
    VIRTIO_GPU_CMD_RESOURCE_CREATE_BLOB = 0x010C,
    VIRTIO_GPU_CMD_SET_SCANOUT_BLOB = 0x010D,
    /* 3d commands */
    VIRTIO_GPU_CMD_CTX_CREATE = 0x0200,
    VIRTIO_GPU_CMD_CTX_DESTROY = 0x0201,
    VIRTIO_GPU_CMD_CTX_ATTACH_RESOURCE = 0x0202,
    VIRTIO_GPU_CMD_CTX_DETACH_RESOURCE = 0x0203,
    VIRTIO_GPU_CMD_RESOURCE_CREATE_3D = 0x0204,
    VIRTIO_GPU_CMD_TRANSFER_TO_HOST_3D = 0x0205,
    VIRTIO_GPU_CMD_TRANSFER_FROM_HOST_3D = 0x0206,
    VIRTIO_GPU_CMD_SUBMIT_3D = 0x0207,
    VIRTIO_GPU_CMD_RESOURCE_MAP_BLOB = 0x0208,
    VIRTIO_GPU_CMD_RESOURCE_UNMAP_BLOB = 0x0209,
    /* cursor commands */
    VIRTIO_GPU_CMD_UPDATE_CURSOR = 0x0300,
    VIRTIO_GPU_CMD_MOVE_CURSOR = 0x0301,
    /* success responses */
    VIRTIO_GPU_RESP_OK_NODATA = 0x1100,
    VIRTIO_GPU_RESP_OK_DISPLAY_INFO = 0x1101,
    VIRTIO_GPU_RESP_OK_CAPSET_INFO = 0x1102,
    VIRTIO_GPU_RESP_OK_CAPSET = 0x1103,
    VIRTIO_GPU_RESP_OK_EDID = 0x1104,
    VIRTIO_GPU_RESP_OK_RESOURCE_UUID = 0x1105,
    VIRTIO_GPU_RESP_OK_MAP_INFO = 0x1106,
    /* error responses */
    VIRTIO_GPU_RESP_ERR_UNSPEC = 0x1200,
    VIRTIO_GPU_RESP_ERR_OUT_OF_MEMORY = 0x1201,
    VIRTIO_GPU_RESP_ERR_INVALID_SCANOUT_ID = 0x1202,
    VIRTIO_GPU_RESP_ERR_INVALID_RESOURCE_ID = 0x1203,
    VIRTIO_GPU_RESP_ERR_INVALID_CONTEXT_ID = 0x1204,
    VIRTIO_GPU_RESP_ERR_INVALID_PARAMETER = 0x1205,
}

// 5.7.5
const VIRTIO_GPU_MAX_SCANOUTS: usize = 16;
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuDisplayOne {
    pub r: VirtioGpuRect,
    pub enabled: u32,
    pub flags: u32,
}
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuRespDisplayInfo {
    pub hdr: VirtioGpuCtrlHdr,
    pub pmodes: [VirtioGpuDisplayOne; VIRTIO_GPU_MAX_SCANOUTS],
}
pub const GET_DISPLAY_INFO_LEN: usize = size_of::<VirtioGpuRespDisplayInfo>();

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuGetEdid {
    pub hdr: VirtioGpuCtrlHdr,
    pub scanout: u32,
    pub padding: u32,
}
pub const GET_EDID_LEN: usize = size_of::<VirtioGpuGetEdid>();

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod)]
pub struct VirtioGpuRespEDID {
    pub hdr: VirtioGpuCtrlHdr,
    pub size: u32,
    pub padding: u32,
    pub edid: [u8; 1024],
}
impl Default for VirtioGpuRespEDID {
    fn default() -> Self {
        VirtioGpuRespEDID {
            hdr: VirtioGpuCtrlHdr::default(),
            size: 0,
            padding: 0,
            edid: [0u8; 1024],
        }
    }
}
pub const RESP_EDID_LEN: usize = size_of::<VirtioGpuRespEDID>();

// 5.7.6.1
#[repr(u32)]
#[derive(Default, Debug, Clone, Copy, TryFromInt)]
#[allow(non_camel_case_types)]
pub enum VirtioGpuFormats {
    #[default]
    VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM = 1,
    VIRTIO_GPU_FORMAT_A8R8G8B8_UNORM = 2,
    VIRTIO_GPU_FORMAT_X8R8G8B8_UNORM = 3,
    VIRTIO_GPU_FORMAT_R8G8B8A8_UNORM = 4,
    VIRTIO_GPU_FORMAT_X8B8G8R8_UNORM = 67,
    VIRTIO_GPU_FORMAT_A8B8G8R8_UNORM = 68,
    VIRTIO_GPU_FORMAT_R8G8B8X8_UNORM = 121,
    VIRTIO_GPU_FORMAT_UNKNOWN = 134,
}
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuResourceCreate2d {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    pub format: u32,
    pub width: u32,
    pub height: u32,
}
pub const CREATE_RESOURCE_LEN: usize = size_of::<VirtioGpuResourceCreate2d>();

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuResourceAttachBacking {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    pub nr_entries: u32,
}
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuMemEntry {
    pub addr: u64,
    pub length: u32,
    pub padding: u32,
}
pub const ATTACH_RESOURCE_LEN: usize = size_of::<VirtioGpuResourceAttachBacking>();
pub const ATTACH_RESOURCE_ENTRY_LEN: usize = size_of::<VirtioGpuResourceAttachBacking>();

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuSetScanout {
    pub hdr: VirtioGpuCtrlHdr,
    pub r: VirtioGpuRect,
    pub scanout_id: u32,
    pub resource_id: u32,
}
pub const SET_SCANOUT_LEN: usize = size_of::<VirtioGpuSetScanout>();

// 5.7.6.2
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuTransferToHost2d {
    pub hdr: VirtioGpuCtrlHdr,
    pub r: VirtioGpuRect,
    pub offset: u64,
    pub resource_id: u32,
    pub padding: u32,
}
pub const TRANSFER_TO_HOST_LEN: usize = size_of::<VirtioGpuTransferToHost2d>();

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuResourceFlush {
    pub hdr: VirtioGpuCtrlHdr,
    pub r: VirtioGpuRect,
    pub resource_id: u32,
    pub padding: u32,
}
pub const FLUSH_RESOURCE_LEN: usize = size_of::<VirtioGpuResourceFlush>();
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod)]
pub struct VirtioGpuCtxCreate {
    pub hdr: VirtioGpuCtrlHdr,
    pub nlen: u32,
    pub context_init: u32,
    pub debug_name: [u8; 64],
}
pub const CREATE_CONTEXT_LEN: usize = size_of::<VirtioGpuCtxCreate>();

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuResourceMapBlob {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    pub padding: u32,
    pub offset: u64,
}
pub const MAP_BLOB_LEN: usize = size_of::<VirtioGpuResourceMapBlob>();

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuRespMapInfo {
    pub hdr: VirtioGpuCtrlHdr,
    pub map_info: u32,
    pub padding: u32,
}
pub const MAP_INFO_LEN: usize = size_of::<VirtioGpuRespMapInfo>();

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuResourceUnmapBlob {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    pub padding: u32,
}
pub const UNMAP_BLOB_LEN: usize = size_of::<VirtioGpuResourceUnmapBlob>();

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuCursorPos {
    pub scanout_id: u32,
    pub x: u32,
    pub y: u32,
    pub padding: u32,
}
pub const CURSOR_POS_LEN: usize = size_of::<VirtioGpuCursorPos>();

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod)]
pub struct VirtioGpuUpdateCursor {
    pub hdr: VirtioGpuCtrlHdr,
    pub pos: VirtioGpuCursorPos,
    pub resource_id: u32,
    pub hot_x: u32,
    pub hot_y: u32,
    pub padding: u32,
}
pub const UPDATE_CURSOR_LEN: usize = size_of::<VirtioGpuUpdateCursor>();
