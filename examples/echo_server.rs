extern crate reliable_udp;
use rand::Rng;
use std::str;

use reliable_udp::manager;
use reliable_udp::packet;
use std::error::Error;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut rng = rand::thread_rng();

    let socket = UdpSocket::bind("0.0.0.0:5050").await?;

    let mut buffer: [u8; 1024] = [0; 1024];

    // receive Syn packet
    let (_, addr) = socket.recv_from(&mut buffer).await?;
    let packet_header = reliable_udp::packet::Header::parse(&buffer[..packet::HEADER_SIZE])?;
    if packet_header.ptype != packet::PType::Syn {
        println!("Not a Syn packet");
        return Ok(());
    }
    if !packet_header.verify_header_checksum() || !packet_header.verify_checksum(None) {
        println!("Bad checksum");
        return Ok(());
    }

    let seq = rng.gen();
    let mut connection = manager::Connection {
        seq: seq,
        ack: packet_header.seq + 1,
        previous_seq: seq,
        is_open: false,
        last_response: 5,
    };

    // send SynAck packet

    let header_checksum = packet::Header::calculate_header_checksum(
        connection.seq,
        connection.ack,
        packet::PType::SynAck,
    );
    let checksum = packet::Header::calculate_checksum(
        connection.seq,
        connection.ack,
        packet::PType::SynAck,
        header_checksum,
        None,
    );
    let packet_header = packet::Header {
        seq: connection.seq,
        ack: connection.ack,
        ptype: packet::PType::SynAck,
        header_checksum,
        checksum,
    };
    let packet = packet::packet_to_binary(packet_header, None);
    socket.send_to(&packet, addr).await?;

    connection.seq += 1;

    // receive Ack packet
    let (_, addr) = socket.recv_from(&mut buffer).await?;
    let packet_header = reliable_udp::packet::Header::parse(&buffer[..packet::HEADER_SIZE])?;
    if packet_header.ptype != packet::PType::Ack {
        println!("Not a Syn packet");
        return Ok(());
    }
    if !packet_header.verify_header_checksum() || !packet_header.verify_checksum(None) {
        println!("Bad checksum");
        return Ok(());
    }
    if packet_header.ack != connection.seq || packet_header.seq != connection.ack {
        println!("Packet needs to be resent");
        return Ok(());
    }

    connection.is_open = true;

    println!("Connection established");

    // receive Psh packet with data
    let (size, addr) = socket.recv_from(&mut buffer).await?;
    let packet_header = reliable_udp::packet::Header::parse(&buffer[..packet::HEADER_SIZE])?;
    let packet_payload = &buffer[packet::HEADER_SIZE..size];
    if packet_header.ptype != packet::PType::Psh {
        println!("Not a SynAck packet");
        return Ok(());
    }
    if !packet_header.verify_header_checksum()
        || !packet_header.verify_checksum(Some(packet_payload))
    {
        println!("Bad checksum");
        return Ok(());
    }
    if packet_header.ack != connection.seq {
        println!("Packet needs to be resent");
        return Ok(());
    }

    connection.ack += packet_payload.len() as u32;
    println!(
        "Received from the client: {:?}",
        str::from_utf8(packet_payload).unwrap()
    );

    // send Psh packet with data
    let data = Some(packet_payload);
    let header_checksum = packet::Header::calculate_header_checksum(
        connection.seq,
        connection.ack,
        packet::PType::Psh,
    );
    let checksum = packet::Header::calculate_checksum(
        connection.seq,
        connection.ack,
        packet::PType::Psh,
        header_checksum,
        data,
    );
    let packet_header = packet::Header {
        seq: connection.seq,
        ack: connection.ack,
        ptype: packet::PType::Psh,
        header_checksum,
        checksum,
    };
    let packet = packet::packet_to_binary(packet_header, data);
    socket.send_to(&packet, addr).await?;

    connection.seq += unsafe { data.unwrap_unchecked().len() as u32 };
    println!("Message sent");

    Ok(())
}
