//! Hot loop: parse a DNS response with the binparse-generated parser and read
//! EVERY field (identical to the `dns` criterion bench's `binparse-full` arm) —
//! a Wireshark-style full dissect. Profiled externally with perf/callgrind.
//! Iterations come from argv[1].

use std::hint::black_box;

use binparse_protocols::dns::{Dns, Dns_rdata};

const DNS_RESPONSE: &[u8] = &[
    0x12, 0x34, 0x81, 0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 7, b'e', b'x', b'a',
    b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0, 0x00, 0x01, 0x00, 0x01, 0xc0, 0x0c, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x00, 0x0e, 0x10, 0x00, 0x04, 0x5d, 0xb8, 0xd8, 0x22,
];

fn work() -> u64 {
    let (mut d, _) = Dns::parse(black_box(DNS_RESPONSE)).unwrap();
    let mut acc = d.id() as u64;
    acc ^= d.qr() as u64;
    acc ^= d.opcode() as u64;
    acc ^= d.aa() as u64;
    acc ^= d.tc() as u64;
    acc ^= d.rd() as u64;
    acc ^= d.ra() as u64;
    acc ^= d.z() as u64;
    acc ^= d.rcode() as u64;
    acc ^= d.qdcount() as u64;
    acc ^= d.ancount() as u64;
    acc ^= d.nscount() as u64;
    acc ^= d.arcount() as u64;
    acc ^= d
        .qname()
        .unwrap()
        .labels()
        .flat_map(|l| l.iter().copied())
        .map(u64::from)
        .sum::<u64>();
    acc ^= d.qtype() as u64;
    acc ^= d.qclass() as u64;
    acc ^= d
        .aname()
        .unwrap()
        .labels()
        .flat_map(|l| l.iter().copied())
        .map(u64::from)
        .sum::<u64>();
    acc ^= d.atype() as u64;
    acc ^= d.aclass() as u64;
    acc ^= d.ttl() as u64;
    acc ^= d.rdlength() as u64;
    acc ^= match d.rdata().unwrap() {
        Dns_rdata::A(mut a) => a
            .addr()
            .unwrap()
            .map(|b| b.unwrap() as u64)
            .fold(0, u64::wrapping_add),
        _ => 0,
    };
    acc
}

fn main() {
    let iters: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2_000_000);
    let mut acc = 0u64;
    let t = std::time::Instant::now();
    for _ in 0..iters {
        acc = acc.wrapping_add(black_box(work()));
    }
    let el = t.elapsed();
    black_box(acc);
    eprintln!(
        "binparse dns: {iters} iters in {el:?} = {:.1} ns/iter",
        el.as_nanos() as f64 / iters as f64
    );
}
