//! DNS message: binparse vs simple-dns, in two regimes.
//!
//! - `binparse` / `simple-dns` (partial): read only id + query name + answer A
//!   address. Favors a lazy zero-copy reader (binparse): most of the packet is
//!   never decoded, so binparse pays for ~3 fields while simple-dns eagerly
//!   parses the whole packet into owned `Vec`s up front.
//! - `binparse-full` / `simple-dns-full`: read EVERY field (a Wireshark-style
//!   full dissect). Fairer when the consumer needs the whole packet — binparse
//!   must now compute every offset and decode both names, and simple-dns's eager
//!   parse is fully used rather than mostly wasted.
//!
//! Both name reads fold over the actual label bytes (not just lengths), matching
//! a dissector that renders the name.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use binparse_bench::*;

fn dns(c: &mut Criterion) {
    let mut g = c.benchmark_group("dns");

    g.bench_function("binparse", |b| {
        use binparse_protocols::dns::{Dns, Dns_rdata};
        b.iter(|| {
            let (dns, _) = Dns::parse(black_box(DNS_RESPONSE)).unwrap();
            let qlen: u64 = dns.qname().unwrap().labels().map(|l| l.len() as u64).sum();
            let addr = match dns.rdata().unwrap() {
                Dns_rdata::A(a) => a.addr().unwrap().map(|b| b.unwrap() as u64).sum(),
                _ => 0u64,
            };
            black_box(dns.id() as u64 ^ qlen ^ addr)
        })
    });

    g.bench_function("simple-dns", |b| {
        use simple_dns::{Packet, rdata::RData};
        b.iter(|| {
            let pkt = Packet::parse(black_box(DNS_RESPONSE)).unwrap();
            let qlen: u64 = pkt.questions[0].qname.as_bytes().map(|l| l.len() as u64).sum();
            let addr = match &pkt.answers[0].rdata {
                RData::A(a) => a.address as u64,
                _ => 0,
            };
            black_box(pkt.id() as u64 ^ qlen ^ addr)
        })
    });

    g.bench_function("binparse-full", |b| {
        use binparse_protocols::dns::{Dns, Dns_rdata};
        b.iter(|| {
            let (d, _) = Dns::parse(black_box(DNS_RESPONSE)).unwrap();
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
            acc ^= d.qname().unwrap().labels().flat_map(|l| l.iter().copied()).map(u64::from).sum::<u64>();
            acc ^= d.qtype() as u64;
            acc ^= d.qclass() as u64;
            acc ^= d.aname().unwrap().labels().flat_map(|l| l.iter().copied()).map(u64::from).sum::<u64>();
            acc ^= d.atype() as u64;
            acc ^= d.aclass() as u64;
            acc ^= d.ttl() as u64;
            acc ^= d.rdlength() as u64;
            acc ^= match d.rdata().unwrap() {
                Dns_rdata::A(a) => a.addr().unwrap().map(|b| b.unwrap() as u64).sum(),
                _ => 0u64,
            };
            black_box(acc)
        })
    });

    g.bench_function("simple-dns-full", |b| {
        use simple_dns::{Packet, PacketFlag, QCLASS, rdata::RData};
        b.iter(|| {
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
            black_box(acc)
        })
    });

    g.finish();
}

criterion_group!(benches, dns);
criterion_main!(benches);
