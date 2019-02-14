use std::fmt::Debug;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum proc_cn_mcast_op {
    LISTEN = 1,
    IGNORE = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct cb_id {
    pub idx: u32,
    pub val: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CNHeader {
    pub cb_id: cb_id,
    pub seq: u32,
    pub ack: u32,
    pub len: u16,
    pub flags: u16,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CNMessage<T: Sized> {
    pub header: CNHeader,
    pub payload: T
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum proc_event_what {
	NONE = 0x00000000,
	FORK = 0x00000001,
	EXEC = 0x00000002,
	UID  = 0x00000004,
	GID  = 0x00000040,
	SID  = 0x00000080,
	PTRACE = 0x00000100,
	COMM = 0x00000200,
	/* "next" should be 0x00000400 */
	/* "last" is the last process event: exit,
	 * while "next to last" is coredumping event */
	COREDUMP = 0x40000000,
	EXIT = 0x80000000
}

#[derive(Debug, Clone, Copy)]
pub enum ProcEventData {
    None,
    Fork {
        parent_pid: u32,
        parent_tgid: u32,
        child_pid: u32,
        child_tgid: u32
    },
    Exec {
        process_pid: u32,
        process_tgid: u32,
    },
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct proc_event {
    pub what: proc_event_what,
    pub cpu: u32,
    pub timestamp_ns: u64,
    event_data: [u8; 24],
}

impl proc_event {
    pub fn data(&self) -> Option<ProcEventData> {
        use std::io;
        use byteorder::{NativeEndian, ReadBytesExt};

        match self.what {
            proc_event_what::NONE => Some(ProcEventData::None),
            proc_event_what::FORK => {
                let mut rdr = io::Cursor::new(&self.event_data);
                let parent_pid = rdr.read_u32::<NativeEndian>().unwrap();
                let parent_tgid = rdr.read_u32::<NativeEndian>().unwrap();
                let child_pid = rdr.read_u32::<NativeEndian>().unwrap();
                let child_tgid = rdr.read_u32::<NativeEndian>().unwrap();
                Some(ProcEventData::Fork {
                    parent_pid: parent_pid,
                    parent_tgid: parent_tgid,
                    child_pid: child_pid,
                    child_tgid: child_tgid,
                })
            },
            proc_event_what::EXEC => {
                let mut rdr = io::Cursor::new(&self.event_data);
                Some(ProcEventData::Exec {
                    process_pid: rdr.read_u32::<NativeEndian>().unwrap(),
                    process_tgid: rdr.read_u32::<NativeEndian>().unwrap()
                })
            },
            _ => None
        }
    }
}
