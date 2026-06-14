//! Hot loop: parse a DNS response with simple-dns and read EVERY field
//! (identical to the `dns` criterion bench's `simple-dns-full` arm) — a
//! Wireshark-style full dissect. Profiled externally with perf/callgrind.
//! Iterations come from argv[1].

use std::hint::black_box;

use simple_dns::{Packet, PacketFlag, QCLASS, rdata::RData};

const DNS_RESPONSE: &[u8] = &[
    0x12, 0x34, 0x81, 0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 7, b'e', b'x', b'a',
    b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0, 0x00, 0x01, 0x00, 0x01, 0xc0, 0x0c, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x00, 0x0e, 0x10, 0x00, 0x04, 0x5d, 0xb8, 0xd8, 0x22,
];

fn work() -> u64 {
    let pkt = Packet::parse(black_box(DNS_RESPONSE)).unwrap();
    let mut acc = pkt.id() as u64;
    acc ^= pkt.has_flags(PacketFlag::RESPONSE) as u64;
    acc ^= pkt.has_flags(PacketFlag::AUTHORITATIVE_ANSWER) as u64;
    acc ^= pkt.has_flags(PacketFlag::TRUNCATION) as u64;
    acc ^= pkt.has_flags(PacketFlag::RECURSION_DESIRED) as u64;
    acc ^= pkt.has_flags(PacketFlag::RECURSION_AVAILABLE) as u64;
    acc ^= pkt.has_flags(PacketFlag::AUTHENTIC_DATA) as u64;
    acc ^= pkt.has_flags(PacketFlag::CHECKING_DISABLED) as u64;
    acc ^= pkt.opcode() as u16 as u64;
    acc ^= pkt.rcode() as u16 as u64;
    acc ^= pkt.questions.len() as u64;
    acc ^= pkt.answers.len() as u64;
    acc ^= pkt.name_servers.len() as u64;
    acc ^= pkt.additional_records.len() as u64;
    let q = &pkt.questions[0];
    acc ^= q.qname.as_bytes().flat_map(|l| l.iter().copied()).map(u64::from).sum::<u64>();
    acc ^= u16::from(q.qtype) as u64;
    acc ^= u16::from(q.qclass) as u64;
    let a = &pkt.answers[0];
    acc ^= a.name.as_bytes().flat_map(|l| l.iter().copied()).map(u64::from).sum::<u64>();
    acc ^= u16::from(QCLASS::from(a.class)) as u64;
    acc ^= a.ttl as u64;
    acc ^= match &a.rdata {
        RData::A(r) => r.address as u64,
        _ => 0,
    };
    acc
}

fn main() {
    let iters: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(30_000_000);
    let mut acc = 0u64;
    let t = std::time::Instant::now();
    for _ in 0..iters {
        acc = acc.wrapping_add(black_box(work()));
    }
    let el = t.elapsed();
    black_box(acc);
    eprintln!(
        "simple-dns: {iters} iters in {el:?} = {:.1} ns/iter",
        el.as_nanos() as f64 / iters as f64
    );
}
