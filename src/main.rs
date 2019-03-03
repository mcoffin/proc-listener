extern crate bytes;
extern crate byteorder;
extern crate libc;
extern crate netlink_packet;
extern crate netlink_sys;
extern crate tokio;
extern crate futures;
extern crate tokio_codec;

mod ffi;
mod nl;
mod cn;
#[cfg(test)]
mod test;

use futures::future;
use netlink_packet::{NetlinkMessage};
use netlink_proto::{NetlinkFramed};
use netlink_sys::{Protocol, SocketAddr, TokioSocket};
use std::mem;
use std::process;
use std::io;
use std::fs;

pub fn cgroup_add_pid<S: AsRef<str>>(group: S, pid: u32) -> io::Result<()> {
    use io::Write;
    let filename = format!("/sys/fs/cgroup/cpuset/{}/cgroup.procs", group.as_ref());
    fs::OpenOptions::new()
        .append(true)
        .open(filename)
        .map(io::LineWriter::new)
        .and_then(|mut procs_file| writeln!(&mut procs_file, "{}", pid))
}

#[inline(always)]
fn nl_bind_address() -> SocketAddr {
    SocketAddr::new(process::id(), ffi::CN_IDX_PROC)
}

fn proc_ev_enable_message(enable: bool) -> (nl::NetlinkMessageHeader, bytes::BytesMut) {
    let v = if enable {
        cn::proc_cn_mcast_op::LISTEN
    } else {
        cn::proc_cn_mcast_op::IGNORE
    };
    let cn_msg = cn::CNMessage {
        header: cn::CNHeader {
            cb_id: cn::cb_id {
                idx: ffi::CN_IDX_PROC,
                val: ffi::CN_VAL_PROC,
            },
            seq: 0,
            ack: 0,
            len: mem::size_of::<cn::proc_cn_mcast_op>() as u16,
            flags: 0,
        },
        payload: v,
    };
    let nl_header = nl::NetlinkMessageHeader {
        len: (mem::size_of::<nl::NetlinkMessageHeader>() + mem::size_of::<cn::CNMessage<cn::proc_cn_mcast_op>>()) as u32,
        ty: nl::NLMSG_DONE,
        flags: 0,
        seq: 0,
        port: process::id(),
    };
    unsafe {
        let cn_msg_bytes: [u8; 24] = mem::transmute(cn_msg);
        (nl_header, From::from(&cn_msg_bytes[..]))
    }
}

fn main() {
    use cn::ProcEventData;
    use futures::{Future, Stream, Sink};
    use tokio::codec::Decoder;
    use tokio::fs as tfs;

    let socket = TokioSocket::new(Protocol::Connector)
        .and_then(|mut s| s.bind(&nl_bind_address()).map(|_| s))
        .unwrap();

    let stream = NetlinkFramed::new(socket, nl::NetlinkCodec);
    let stream = stream.send((proc_ev_enable_message(true), SocketAddr::new(0, 0))).wait().unwrap();

    let handle_messages = stream
        .map(|(msg, _)| msg)
        .filter_map(|(header, payload)| {
            if payload.len() < mem::size_of::<cn::CNMessage<cn::proc_event>>() {
                None
            } else {
                Some((header, unsafe {
                    let payload_ptr: *const cn::CNMessage<cn::proc_event> = mem::transmute(payload.as_ref().as_ptr());
                    *payload_ptr
                }))
            }
        })
        .map(|(header, payload)| payload.payload)
        .filter_map(|payload| payload.data())
        .filter_map(|event_data| match event_data {
            ProcEventData::None => None,
            ProcEventData::Fork { .. } => None,
            ProcEventData::Exec { process_tgid, .. } => Some(process_tgid),
        })
        .and_then(|pid| {
            tfs::File::open(format!("/proc/{}/status", &pid))
                .and_then(|f| {
                    tokio::codec::LinesCodec::new()
                        .framed(f)
                        .take(1)
                        .into_future()
                        .map_err(|(io_err, _)| io_err)
                })
                .map(|(v, _)| v.unwrap())
                .map(move |mut s| (pid, s.split_off(6)))
                .then(|res| match res {
                    Ok(v) => Ok(Some(v)),
                    Err(e) => match e.kind() {
                        io::ErrorKind::NotFound => Ok(None),
                        _ => Err(e),
                    },
                })
        })
        .filter_map(|v| v)
        .filter(|&(pid, ref name)| match name.as_ref() {
            "League of Legen" => true,
            other => false,
        })
        .map_err(|e| panic!("{:?}", e))
        .for_each(|(pid, name)| {
            println!("{}:{}", &name, pid);
            let group_name = if name.starts_with("League of") {
                "league_game"
            } else {
                "."
            };
            std::thread::spawn(move || cgroup_add_pid(&group_name, pid).unwrap());
            future::ok(())
        });

    // Pass future off to tokio runtime
    tokio::run(handle_messages);
}
