use serialize::json::{Json, ToJson};

bitflags! {
    #[deriving(Show)] flags WSHeader: u16 {
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

        // Helper masks
        const WS_OPCTRL  = 0b0000100000000000, // if matches with &, this is a control code
        const WS_LEN16   = 0b0000000001111110, // if &WS_LEN equals this, it has 16-bit length
        const WS_LEN64   = 0b0000000001111111, // if &WS_LEN equals this, it has 32-bit length
    }
}

#[deriving(Show)]
pub struct WSMessage {
    pub header: WSHeader,
    pub data: Vec<u8>
}

impl WSMessage {
    pub fn to_string(&self) -> String {
        String::from_utf8_lossy(self.data.as_slice()).into_string()
    }
}

impl ToJson for WSMessage {
    fn to_json(&self) -> Json {
        from_str::<Json>(self.to_string().as_slice()).unwrap()
    }
}

