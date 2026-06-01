/// IEC 60870-5-104 APDU helpers.
///
/// This module keeps the first TCP support small: build common U-format frames
/// and parse basic APDU metadata without implementing full ASDU decoding yet.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Iec104FrameKind {
    IFormat,
    SFormat,
    UFormat(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iec104Apdu {
    pub control: [u8; 4],
    pub asdu: Vec<u8>,
}

impl Iec104Apdu {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 6 {
            return Err("IEC104 APDU too short".to_string());
        }
        if bytes[0] != 0x68 {
            return Err(format!("Unexpected IEC104 start byte: 0x{:02X}", bytes[0]));
        }

        let length = bytes[1] as usize;
        if length < 4 {
            return Err("IEC104 length smaller than control field".to_string());
        }
        if bytes.len() < length + 2 {
            return Err(format!(
                "IEC104 APDU truncated: expected {} bytes, got {}",
                length + 2,
                bytes.len()
            ));
        }

        Ok(Self {
            control: [bytes[2], bytes[3], bytes[4], bytes[5]],
            asdu: bytes[6..length + 2].to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let length = 4 + self.asdu.len();
        let mut bytes = Vec::with_capacity(length + 2);
        bytes.push(0x68);
        bytes.push(length as u8);
        bytes.extend_from_slice(&self.control);
        bytes.extend_from_slice(&self.asdu);
        bytes
    }

    pub fn kind(&self) -> Iec104FrameKind {
        if self.control[0] & 0x01 == 0 {
            Iec104FrameKind::IFormat
        } else if self.control[0] & 0x03 == 0x01 {
            Iec104FrameKind::SFormat
        } else {
            Iec104FrameKind::UFormat(self.control[0])
        }
    }
}

pub fn build_startdt_act() -> Vec<u8> {
    Iec104Apdu {
        control: [0x07, 0x00, 0x00, 0x00],
        asdu: Vec::new(),
    }
    .to_bytes()
}

pub fn build_stopdt_act() -> Vec<u8> {
    Iec104Apdu {
        control: [0x13, 0x00, 0x00, 0x00],
        asdu: Vec::new(),
    }
    .to_bytes()
}

pub fn build_testfr_act() -> Vec<u8> {
    Iec104Apdu {
        control: [0x43, 0x00, 0x00, 0x00],
        asdu: Vec::new(),
    }
    .to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startdt_act_bytes() {
        assert_eq!(
            build_startdt_act(),
            vec![0x68, 0x04, 0x07, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn test_parse_u_format() {
        let frame = Iec104Apdu::parse(&build_testfr_act()).unwrap();
        assert_eq!(frame.kind(), Iec104FrameKind::UFormat(0x43));
        assert!(frame.asdu.is_empty());
    }
}
