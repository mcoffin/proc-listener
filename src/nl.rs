use bytes::BytesMut;
use byteorder::{NativeEndian, ReadBytesExt};
use tokio_codec::{Decoder, Encoder};
use std::io;
use std::mem;

pub const NLMSG_DONE: u16 = 0x3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct NetlinkMessageHeader {
    pub len: u32,
    pub ty: u16,
    pub flags: u16,
    pub seq: u32,
    pub port: u32,
}

impl NetlinkMessageHeader {
    pub fn payload_len(&self) -> usize {
        (self.len as usize) - mem::size_of::<Self>()
    }
}

pub struct NetlinkCodec;

impl Decoder for NetlinkCodec {
    type Item = (NetlinkMessageHeader, BytesMut);
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < mem::size_of::<NetlinkMessageHeader>() {
            Ok(None)
        } else {
            let len_res = {
                let mut reader = io::Cursor::new(src.as_ref());
                reader.read_u32::<NativeEndian>()
            };
            len_res.map(|len| len as usize).and_then(|len| {
                if src.len() >= len {
                    let mut payload = src.split_to(len); // Take the bytes for this message
                    let header_bytes = payload.split_to(mem::size_of::<NetlinkMessageHeader>());
                    let header = unsafe {
                        let src_ptr: *const NetlinkMessageHeader = mem::transmute(header_bytes.as_ref().as_ptr());
                        *src_ptr
                    };
                    Ok(Some((header, payload)))
                } else {
                    Ok(None)
                }
            })
        }
    }
}

impl Encoder for NetlinkCodec {
    type Item = (NetlinkMessageHeader, BytesMut);
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use byteorder::WriteBytesExt;
        let (header, payload) = item;
        dst.reserve(header.len as usize);
        if header.payload_len() == payload.len() {
            // First write the header
            let header_bytes: [u8; 16] = unsafe { mem::transmute(header) };
            dst.extend_from_slice(&header_bytes);
            // Then just write the payload bytes immediately
            dst.extend_from_slice(payload.as_ref());
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "payload size does not match header"))
        }
    }
}
