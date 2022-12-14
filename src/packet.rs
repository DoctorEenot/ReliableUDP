#![allow(arithmetic_overflow)]
use crate::errors::*;

pub const HEADER_SIZE: usize = 14;
pub const MAX_PACKET_SIZE: usize = 65507;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum PType {
    Syn = 1,
    SynAck,
    Ack,
    Psh,
    Fin,
}

pub struct Header {
    pub seq: u32,
    pub ack: u32,
    // padding 1 byte
    pub ptype: PType,
    pub header_checksum: u16,
    pub checksum: u16,
}

impl Header {
    pub fn parse(data: &[u8]) -> Result<Header> {
        if data.len() < HEADER_SIZE {
            return Err(packet_parsing_errors::TooSmallPacket.into());
        } else if data.len() > MAX_PACKET_SIZE {
            return Err(packet_parsing_errors::TooBigPacket::new(data.len()).into());
        }

        let seq: u32 = u32::from_be_bytes(data[0..4].try_into()?);

        let ack: u32 = u32::from_be_bytes(data[4..8].try_into()?);

        let ptype: PType = match data[9] {
            1 => PType::Syn,
            2 => PType::SynAck,
            3 => PType::Ack,
            4 => PType::Psh,
            5 => PType::Fin,
            _ => return Err(packet_parsing_errors::UknownPType::new(data[8]).into()),
        };

        let header_checksum: u16 = u16::from_be_bytes(data[10..12].try_into()?);

        let checksum: u16 = u16::from_be_bytes(data[12..14].try_into()?);

        Ok(Header {
            seq,
            ack,
            ptype,
            header_checksum,
            checksum,
        })
    }

    pub fn calculate_header_checksum(seq: u32, ack: u32, ptype: PType) -> u16 {
        let mut checksum: u16 = 0;

        checksum += (seq >> 16) as u16;
        checksum += seq as u16;

        checksum += (ack >> 16) as u16;
        checksum += ack as u16;

        checksum += ptype as u16;

        checksum
    }

    pub fn calculate_checksum(
        seq: u32,
        ack: u32,
        ptype: PType,
        header_checksum: u16,
        data: Option<&[u8]>,
    ) -> u16 {
        let mut checksum: u16 = header_checksum;

        checksum += (seq >> 16) as u16;
        checksum += seq as u16;

        checksum += (ack >> 16) as u16;
        checksum += ack as u16;

        checksum += ptype as u16;

        if data.is_some() {
            let dt = unsafe { data.unwrap_unchecked() };

            if dt.len() % 2 == 0 {
                for index in (0..dt.len()).step_by(2) {
                    checksum += (dt[index] as u16) << 8;
                    checksum += dt[index + 1] as u16;
                }
            } else {
                for index in (0..dt.len() - 1).step_by(2) {
                    checksum += (dt[index] as u16) << 8;
                    checksum += dt[index + 1] as u16;
                }
                checksum += (dt[dt.len() - 1] as u16) << 8;
            }
        }

        checksum
    }

    pub fn verify_header_checksum(&self) -> bool {
        let calculated_checksum = Header::calculate_header_checksum(self.seq, self.ack, self.ptype);

        self.header_checksum == calculated_checksum
    }

    pub fn verify_checksum(&self, data: Option<&[u8]>) -> bool {
        let calculated_checksum =
            Header::calculate_checksum(self.seq, self.ack, self.ptype, self.header_checksum, data);

        self.checksum == calculated_checksum
    }
}

/// needs rewriting
pub fn packet_to_binary(header: Header, data: Option<&[u8]>) -> Vec<u8> {
    let mut to_return: Vec<u8>;
    if data.is_some() {
        to_return = Vec::with_capacity(HEADER_SIZE + unsafe { data.unwrap_unchecked().len() });
    } else {
        to_return = Vec::with_capacity(HEADER_SIZE);
    }

    for b in header.seq.to_be_bytes() {
        to_return.push(b);
    }

    for b in header.ack.to_be_bytes() {
        to_return.push(b);
    }

    to_return.push(0);

    to_return.push(header.ptype as u8);

    for b in header.header_checksum.to_be_bytes() {
        to_return.push(b);
    }

    for b in header.checksum.to_be_bytes() {
        to_return.push(b);
    }

    if data.is_some() {
        for b in unsafe { data.unwrap_unchecked() } {
            to_return.push(*b);
        }
    }

    to_return
}
