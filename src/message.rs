use std::str::FromStr;
use std::num::{FromPrimitive, ToPrimitive};
use rustc_serialize::json::{Json, ToJson};

bitflags! {
    #[derive(Show)] flags WSHeader: u16 {
        // Main structure, mask with & to get header parts
        const WS_FIN     = 0b1000000000000000, // final flag
        const WS_RSV     = 0b0111000000000000, // reserved
        const WS_OPCODE  = 0b0000111100000000, // opcode
        const WS_MASK    = 0b0000000010000000, // mask flag
        const WS_LEN     = 0b0000000001111111, // length

        // Opcodes, check for equality after masking with WS_OPCODE
        const WS_OPCONT  = 0b0000000000000000,
        const WS_OPTEXT  = 0b0000000100000000,
        const WS_OPBIN   = 0b0000001000000000,
        const WS_OPTERM  = 0b0000100000000000,
        const WS_OPPING  = 0b0000100100000000,
        const WS_OPPONG  = 0b0000101000000000,

        // Bits reserved for extensions, check for equality after &-ing with OP_RSV
        const WS_RSV1 = 0b0100000000000000,
        const WS_RSV2 = 0b0010000000000000,
        const WS_RSV3 = 0b0001000000000000,

        // Opcodes reserved for extensions, check for equality after &-ing with OP_OPCODE
        const WS_OPEXT1 = 0b0000001100000000,
        const WS_OPEXT2 = 0b0000010000000000,
        const WS_OPEXT3 = 0b0000010100000000,
        const WS_OPEXT4 = 0b0000011000000000,
        const WS_OPEXT5 = 0b0000011100000000,

        // Helper masks
        const WS_OPCTRL  = 0b0000100000000000, // if matches with &, this is a control code
        const WS_LEN16   = 0b0000000001111110, // if &WS_LEN equals this, it has 16-bit length
        const WS_LEN64   = 0b0000000001111111, // if &WS_LEN equals this, it has 32-bit length
    }
}

// TODO: use this instead of u16
#[derive(Copy, Show)]
pub enum WSStatusCode {
    NoError, // = 1000,
    GoneAway, // = 1001,
    ProtocolError, // = 1002,
    UnsupportedData, // = 1003,

    // RESERVERD = 1004,
    NoCode, // = 1005, // reserved
    Aborted, // = 1006, // reserved

    InvalidData, // = 1007,
    ClientError, // = 1008,
    TooLargeData, // = 1009,
    ExtensionMissing, // = 1010,
    ServerError, // = 1011,

    TlsError, // = 1015 // reserved

    // Also:
    // 0-999 - cannot be used,
    // 1000-2999 - reserved for protocol,
    ProtocolCode(u16),
    // 3000-3999 - reserved for apps, issued by IANA,
    ApplicationCode(u16),
    // 4000-4999 - for private use
    OtherCode(u16)
}

impl ToPrimitive for WSStatusCode {
    fn to_u64(&self) -> Option<u64> {
        match *self {
            WSStatusCode::NoError => Some(1000u64),
            WSStatusCode::GoneAway => Some(1001u64),
            WSStatusCode::ProtocolError => Some(1002u64),
            WSStatusCode::UnsupportedData => Some(1003u64),

            WSStatusCode::NoCode => Some(1005u64), // reserved
            WSStatusCode::Aborted => Some(1006u64), // reserved

            WSStatusCode::InvalidData => Some(1007u64),
            WSStatusCode::ClientError => Some(1008u64),
            WSStatusCode::TooLargeData => Some(1009u64),
            WSStatusCode::ExtensionMissing => Some(1010u64),
            WSStatusCode::ServerError => Some(1011u64),

            WSStatusCode::TlsError => Some(1015u64), // reserved

            WSStatusCode::ProtocolCode(code) if 1000 <= code && code <= 2999 => Some(code as u64),
            WSStatusCode::ApplicationCode(code) if 3000 <= code && code <= 3999 => Some(code as u64),
            WSStatusCode::OtherCode(code) if 4000 <= code && code <= 4999 => Some(code as u64),
            _ => None
        }
    }
    fn to_i64(&self) -> Option<i64> {
        self.to_u64().map(|v| v as i64)
    }
}

impl FromPrimitive for WSStatusCode {
    fn from_u64(n: u64) -> Option<WSStatusCode> {
        match n {
            1000u64 => Some(WSStatusCode::NoError),
            1001u64 => Some(WSStatusCode::GoneAway),
            1002u64 => Some(WSStatusCode::ProtocolError),
            1003u64 => Some(WSStatusCode::UnsupportedData),

            1005u64 => Some(WSStatusCode::NoCode), // reserved
            1006u64 => Some(WSStatusCode::Aborted), // reserved

            1007u64 => Some(WSStatusCode::InvalidData),
            1008u64 => Some(WSStatusCode::ClientError),
            1009u64 => Some(WSStatusCode::TooLargeData),
            1010u64 => Some(WSStatusCode::ExtensionMissing),
            1011u64 => Some(WSStatusCode::ServerError),

            1015u64 => Some(WSStatusCode::TlsError), // reserved

            code if 1000 <= code && code <= 2999 => Some(WSStatusCode::ProtocolCode(code as u16)),
            code if 3000 <= code && code <= 3999 => Some(WSStatusCode::ApplicationCode(code as u16)),
            code if 4000 <= code && code <= 4999 => Some(WSStatusCode::OtherCode(code as u16)),
            _ => None
        }
    }
    fn from_i64(n: i64) -> Option<WSStatusCode> {
        if n < 0i64 {
            None
        } else {
            FromPrimitive::from_u64(n as u64)
        }
    }
}

// TODO
// pub struct WSMessage<T=Vec<u8>> {
//     ...
//     data: T
// }
// pub struct WSDataWithStatus<T=Vec<u8>> {
//     pub status: WSStatusCode,
//     pub payload: Vec
// }
#[derive(Show)]
pub struct WSMessage {
    pub header: WSHeader,
    pub data: Vec<u8>,
    pub status: Option<WSStatusCode>
}

impl WSMessage {
    pub fn to_string(&self) -> String {
        String::from_utf8_lossy(self.data[]).into_owned()
    }

    pub fn push(&mut self, msg: WSMessage) {
        self.data.push_all(msg.data[]);
    }

    #[inline] pub fn text(data: &str) -> WSMessage {
        WSMessage {
            header: WS_FIN | WS_OPTEXT,
            data: data.as_bytes().to_vec(),
            status: None
        }
    }

    #[inline] pub fn ext(extn: u8, data: &[u8]) -> WSMessage {
        WSMessage {
            header: WS_FIN | WSHeader::from_bits_truncate((extn & 0x7u8) as u16 << 8),
            data: data.to_vec(),
            status: None
        }
    }

    #[inline] pub fn binary(data: &[u8]) -> WSMessage {
        WSMessage {
            header: WS_FIN | WS_OPBIN,
            data: data.to_vec(),
            status: None
        }
    }

    pub fn first(mut self) -> WSMessage {
        self.header.remove(WS_FIN);
        self
    }

    pub fn more(mut self) -> WSMessage {
        self.header.remove(WS_FIN | WS_OPCODE);
        self.header.insert(WS_OPCONT);
        self
    }

    pub fn last(mut self) -> WSMessage {
        self.header.remove(WS_OPCODE);
        self.header.insert(WS_FIN | WS_OPCONT);
        self
    }

    pub fn is_first(&self) -> bool {
        !self.is_final() && !self.is_cont()
    }

    pub fn is_last(&self) -> bool {
        self.is_final() && self.is_cont()
    }

    pub fn is_more(&self) -> bool {
        !self.is_final() && self.is_cont()
    }

    pub fn is_whole(&self) -> bool {
        self.is_final() && !self.is_cont()
    }

    #[inline] pub fn is_final(&self) -> bool {
        self.header.contains(WS_FIN)
    }

    #[inline] pub fn opcode(&self) -> WSHeader {
        self.header & WS_OPCODE
    }

    pub fn mask(mut self) -> WSMessage {
        self.header.insert(WS_MASK);
        self
    }

    pub fn unmask(mut self) -> WSMessage {
        self.header.remove(WS_MASK);
        self
    }

    pub fn is_masked(&self) -> bool {
        self.header.contains(WS_MASK)
    }

    #[inline] pub fn close(status: WSStatusCode, data: &[u8]) -> WSMessage {
        WSMessage {
            header: WS_FIN | WS_OPTERM,
            data: data.to_vec(),
            status: Some(status)
        }
    }

    #[inline] pub fn ping(data: &[u8]) -> WSMessage {
        WSMessage {
            header: WS_FIN | WS_OPPING,
            data: data.to_vec(),
            status: None
        }
    }

    #[inline] pub fn pong(data: &[u8]) -> WSMessage {
        WSMessage {
            header: WS_FIN | WS_OPPONG,
            data: data.to_vec(),
            status: None
        }
    }

    pub fn is_control(&self) -> bool {
        self.header.contains(WS_OPCTRL)
    }

    pub fn rsv(mut self, n: u8) -> WSMessage {
        self.header.insert(WSHeader::from_bits_truncate((n & 0x7u8) as u16 << 12));
        self
    }

    pub fn is_rsv(&self, n: u8) -> bool {
        (self.header.bits() >> 12) as u8 & 0x7u8 == n
    }

    #[inline] pub fn is_text(&self) -> bool { self.opcode() == WS_OPTEXT }
    #[inline] pub fn is_binary(&self) -> bool { self.opcode() == WS_OPBIN }
    #[inline] pub fn is_isext(&self, n: u8) -> bool { ((self.opcode().bits() >> 8) as u8 & 0x7u8) == n }
    #[inline] pub fn is_ping(&self) -> bool { self.opcode() == WS_OPPING }
    #[inline] pub fn is_pong(&self) -> bool { self.opcode() == WS_OPPONG }
    #[inline] pub fn is_close(&self) -> bool { self.opcode() == WS_OPTERM }
    #[inline] pub fn is_cont(&self) -> bool { self.opcode() == WS_OPCONT }

    pub fn split(self, maxlen: uint) -> WSFragmentedMessage {
        WSFragmentedMessage {
            size: self.data.len() + if self.status.is_none() { 0 } else { 2 },
            original: self,
            maxsize: maxlen,
            pos: 0
        }
    }
}

pub struct WSFragmentedMessage {
    original: WSMessage,
    maxsize: uint,
    pos: uint,
    size: uint
}

impl Iterator for WSFragmentedMessage {
    type Item = WSMessage;
    fn next(&mut self) -> Option<WSMessage> {
        if self.size == 0 {
            None
        } else if self.size <= self.maxsize { // last
            self.size = 0;
            Some(WSMessage {
                header: self.original.header | WS_FIN,
                status: self.original.status,
                data: self.original.data[self.pos..].to_vec()
            })
        } else if self.pos == 0 { // first
            let maxsize = self.maxsize - if self.original.status.is_none() { 0 } else { 2 };
            let result = Some(WSMessage {
                header: self.original.header - WS_FIN,
                status: self.original.status,
                data: self.original.data[..maxsize].to_vec()
            });
            self.original.header.remove(WS_FIN | WS_OPCODE);
            self.original.status = None;
            self.pos = maxsize;
            self.size -= self.maxsize;
            result
        } else { // middle
            let pos = self.pos;
            self.pos += self.maxsize;
            self.size -= self.maxsize;
            Some(WSMessage {
                header: self.original.header,
                status: None,
                data: self.original.data[pos..pos+self.maxsize].to_vec()
            })
        }
    }
}

impl ToJson for WSMessage {
    fn to_json(&self) -> Json {
        self.to_string()[].parse::<Json>().unwrap()
    }
}

impl FromStr for WSMessage {
    #[inline] fn from_str(s: &str) -> Option<WSMessage> {
        Some(WSMessage::text(s))
    }
}
