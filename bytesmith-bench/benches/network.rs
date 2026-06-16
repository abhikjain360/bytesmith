//! Ethernet / IPv4 / IPv6 / UDP / TCP: bytesmith vs etherparse (hand-written
//! zero-copy) vs pnet_packet (macro-generated zero-copy).
//!
//! Each routine parses the header and reads a realistic set of fields, folding
//! byte-array fields (MAC / IP addresses) into a checksum so every library does
//! equivalent work. bytesmith exposes fixed byte arrays as fallible iterators, so
//! its address reads pay a per-byte `Result` cost — that is its real API and is
//! included deliberately.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use bytesmith_bench::*;

fn sum_bp(it: impl Iterator<Item = bytesmith::ParseResult<u8>>) -> u64 {
    it.map(|b| b.unwrap() as u64).fold(0, u64::wrapping_add)
}

fn fold(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes
        .into_iter()
        .fold(0u64, |a, b| a.wrapping_add(b as u64))
}

fn ethernet(c: &mut Criterion) {
    let mut g = c.benchmark_group("ethernet");
    g.bench_function("bytesmith", |b| {
        b.iter(|| {
            let (mut eth, _) =
                bytesmith_protocols::ethernet::EthernetII::parse(black_box(ETH_FRAME)).unwrap();
            black_box(
                eth.ethertype() as u64 ^ sum_bp(eth.dst().unwrap()) ^ sum_bp(eth.src().unwrap()),
            )
        })
    });
    g.bench_function("etherparse", |b| {
        b.iter(|| {
            let eth =
                etherparse::Ethernet2Slice::from_slice_without_fcs(black_box(ETH_FRAME)).unwrap();
            black_box(
                u16::from(eth.ether_type()) as u64 ^ fold(eth.destination()) ^ fold(eth.source()),
            )
        })
    });
    g.bench_function("pnet", |b| {
        b.iter(|| {
            let eth = pnet_packet::ethernet::EthernetPacket::new(black_box(ETH_FRAME)).unwrap();
            black_box(
                eth.get_ethertype().0 as u64
                    ^ fold(eth.get_destination().octets())
                    ^ fold(eth.get_source().octets()),
            )
        })
    });
    g.finish();
}

fn ipv4(c: &mut Criterion) {
    let mut g = c.benchmark_group("ipv4");
    g.bench_function("bytesmith", |b| {
        b.iter(|| {
            let (mut ip, _) = bytesmith_protocols::ip::Ipv4::parse(black_box(IPV4_PACKET)).unwrap();
            black_box(
                ip.version() as u64
                    ^ ip.ihl() as u64
                    ^ ip.total_len() as u64
                    ^ ip.proto() as u64
                    ^ sum_bp(ip.src().unwrap())
                    ^ sum_bp(ip.dst().unwrap()),
            )
        })
    });
    g.bench_function("etherparse", |b| {
        b.iter(|| {
            let ip = etherparse::Ipv4Slice::from_slice(black_box(IPV4_PACKET)).unwrap();
            let h = ip.header();
            black_box(
                h.version() as u64
                    ^ h.ihl() as u64
                    ^ h.total_len() as u64
                    ^ u8::from(h.protocol()) as u64
                    ^ fold(h.source())
                    ^ fold(h.destination()),
            )
        })
    });
    g.bench_function("pnet", |b| {
        b.iter(|| {
            let ip = pnet_packet::ipv4::Ipv4Packet::new(black_box(IPV4_PACKET)).unwrap();
            black_box(
                ip.get_version() as u64
                    ^ ip.get_header_length() as u64
                    ^ ip.get_total_length() as u64
                    ^ ip.get_next_level_protocol().0 as u64
                    ^ fold(ip.get_source().octets())
                    ^ fold(ip.get_destination().octets()),
            )
        })
    });
    g.finish();
}

fn ipv6(c: &mut Criterion) {
    let mut g = c.benchmark_group("ipv6");
    g.bench_function("bytesmith", |b| {
        b.iter(|| {
            let (mut ip, _) = bytesmith_protocols::ip::Ipv6::parse(black_box(IPV6_PACKET)).unwrap();
            black_box(
                ip.version() as u64
                    ^ ip.next_header() as u64
                    ^ ip.payload_len() as u64
                    ^ sum_bp(ip.src().unwrap())
                    ^ sum_bp(ip.dst().unwrap()),
            )
        })
    });
    g.bench_function("etherparse", |b| {
        b.iter(|| {
            let ip = etherparse::Ipv6Slice::from_slice(black_box(IPV6_PACKET)).unwrap();
            let h = ip.header();
            black_box(
                h.version() as u64
                    ^ u8::from(h.next_header()) as u64
                    ^ h.payload_length() as u64
                    ^ fold(h.source())
                    ^ fold(h.destination()),
            )
        })
    });
    g.bench_function("pnet", |b| {
        b.iter(|| {
            let ip = pnet_packet::ipv6::Ipv6Packet::new(black_box(IPV6_PACKET)).unwrap();
            black_box(
                ip.get_version() as u64
                    ^ ip.get_next_header().0 as u64
                    ^ ip.get_payload_length() as u64
                    ^ fold(ip.get_source().octets())
                    ^ fold(ip.get_destination().octets()),
            )
        })
    });
    g.finish();
}

fn udp(c: &mut Criterion) {
    let mut g = c.benchmark_group("udp");
    g.bench_function("bytesmith", |b| {
        b.iter(|| {
            let (mut udp, _) = bytesmith_protocols::udp::Udp::parse(black_box(UDP_PACKET)).unwrap();
            black_box(udp.src_port() as u64 ^ udp.dst_port() as u64 ^ udp.length() as u64)
        })
    });
    g.bench_function("etherparse", |b| {
        b.iter(|| {
            let udp = etherparse::UdpHeaderSlice::from_slice(black_box(UDP_PACKET)).unwrap();
            black_box(
                udp.source_port() as u64 ^ udp.destination_port() as u64 ^ udp.length() as u64,
            )
        })
    });
    g.bench_function("pnet", |b| {
        b.iter(|| {
            let udp = pnet_packet::udp::UdpPacket::new(black_box(UDP_PACKET)).unwrap();
            black_box(
                udp.get_source() as u64 ^ udp.get_destination() as u64 ^ udp.get_length() as u64,
            )
        })
    });
    g.finish();
}

fn tcp(c: &mut Criterion) {
    let mut g = c.benchmark_group("tcp");
    g.bench_function("bytesmith", |b| {
        b.iter(|| {
            let (mut tcp, _) = bytesmith_protocols::tcp::Tcp::parse(black_box(TCP_PACKET)).unwrap();
            black_box(
                tcp.src_port() as u64
                    ^ tcp.dst_port() as u64
                    ^ tcp.seq() as u64
                    ^ tcp.data_offset() as u64
                    ^ tcp.ack() as u64
                    ^ tcp.window() as u64,
            )
        })
    });
    g.bench_function("etherparse", |b| {
        b.iter(|| {
            let tcp = etherparse::TcpHeaderSlice::from_slice(black_box(TCP_PACKET)).unwrap();
            black_box(
                tcp.source_port() as u64
                    ^ tcp.destination_port() as u64
                    ^ tcp.sequence_number() as u64
                    ^ tcp.data_offset() as u64
                    ^ tcp.ack() as u64
                    ^ tcp.window_size() as u64,
            )
        })
    });
    g.bench_function("pnet", |b| {
        b.iter(|| {
            let tcp = pnet_packet::tcp::TcpPacket::new(black_box(TCP_PACKET)).unwrap();
            black_box(
                tcp.get_source() as u64
                    ^ tcp.get_destination() as u64
                    ^ tcp.get_sequence() as u64
                    ^ tcp.get_data_offset() as u64
                    ^ ((tcp.get_flags() & 0x10) != 0) as u64
                    ^ tcp.get_window() as u64,
            )
        })
    });
    g.finish();
}

criterion_group!(benches, ethernet, ipv4, ipv6, udp, tcp);
criterion_main!(benches);
