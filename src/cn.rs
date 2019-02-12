#[repr(C)]
pub enum proc_cn_mcast_op {
    LISTEN = 1,
    IGNORE = 2,
}

#[repr(C)]
pub struct cb_id {
    pub idx: u32,
    pub val: u32,
}

#[repr(C)]
pub struct CNHeader {
    pub cb_id: cb_id,
    pub seq: u32,
    pub ack: u32,
    pub len: u16,
    pub flags: u16,
}

#[repr(C)]
pub struct CNMessage<T: Sized> {
    pub header: CNHeader,
    pub payload: T
}
