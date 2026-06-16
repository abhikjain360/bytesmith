//! DNS message: bytesmith vs simple-dns, in two regimes.
//!
//! - `bytesmith` / `simple-dns` (partial): read only id + query name + answer A
//!   address. Favors a lazy zero-copy reader (bytesmith): most of the packet is
//!   never decoded, so bytesmith pays for ~3 fields while simple-dns eagerly
//!   parses the whole packet into owned `Vec`s up front.
//! - `bytesmith-full` / `simple-dns-full`: read EVERY field (a Wireshark-style
//!   full dissect). Fairer when the consumer needs the whole packet — bytesmith
//!   must now compute every offset and decode both names, and simple-dns's eager
//!   parse is fully used rather than mostly wasted.
//!
//! Both name reads fold over the actual label bytes (not just lengths), matching
//! a dissector that renders the name.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use bytesmith_bench::*;

fn dns(c: &mut Criterion) {
    let mut g = c.benchmark_group("dns");

    g.bench_function("bytesmith", |b| {
        use bytesmith_protocols::dns::{Dns, Dns_rdata};
        b.iter(|| {
            let (mut dns, _) = Dns::parse(black_box(DNS_RESPONSE)).unwrap();
            let qlen: u64 = dns.qname().unwrap().labels().map(|l| l.len() as u64).sum();
            let addr = match dns.rdata().unwrap() {
                Dns_rdata::A(mut a) => a.addr().unwrap().map(|b| b.unwrap() as u64).sum(),
                _ => 0u64,
            };
            black_box(dns.id() as u64 ^ qlen ^ addr)
        })
    });

    g.bench_function("simple-dns", |b| {
        use simple_dns::{Packet, rdata::RData};
        b.iter(|| {
            let pkt = Packet::parse(black_box(DNS_RESPONSE)).unwrap();
            let qlen: u64 = pkt.questions[0]
                .qname
                .as_bytes()
                .map(|l| l.len() as u64)
                .sum();
            let addr = match &pkt.answers[0].rdata {
                RData::A(a) => a.address as u64,
                _ => 0,
            };
            black_box(pkt.id() as u64 ^ qlen ^ addr)
        })
    });

    g.bench_function("bytesmith-full", |b| {
        use bytesmith_protocols::dns::{Dns, Dns_rdata};
        b.iter(|| {
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
                Dns_rdata::A(mut a) => a.addr().unwrap().map(|b| b.unwrap() as u64).sum(),
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
            acc ^= q
                .qname
                .as_bytes()
                .flat_map(|l| l.iter().copied())
                .map(u64::from)
                .sum::<u64>();
            acc ^= u16::from(q.qtype) as u64;
            acc ^= u16::from(q.qclass) as u64;
            let a = &pkt.answers[0];
            acc ^= a
                .name
                .as_bytes()
                .flat_map(|l| l.iter().copied())
                .map(u64::from)
                .sum::<u64>();
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

// Write-path group: serialize a structured value to fresh owned bytes (alloc +
// encode), mirroring bytesmith's `to_vec`, the mirror image of the read group.
// bytesmith's `DnsContent` borrows the field slices (the `&'static` name fixture,
// built untimed) and its timed work is purely encode-into-a-fresh-Vec, deriving
// atype/rdlength and the answer-name compression pointer; simple-dns holds an
// owned `Packet` (allocating `Name`/`Vec`), built untimed, and the timed routine
// builds a fresh compressed byte vec. Both sides pay one output allocation + one
// encode pass.
fn dns_write(c: &mut Criterion) {
    use bytesmith_protocols::dns::{AContent, DnsContent, DnsRdataContent, DnsWriter};

    const NAME: &[u8] = &[
        7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0,
    ];

    let content = DnsContent {
        id: 0x1234,
        qr: 1,
        opcode: 0,
        aa: 0,
        tc: 0,
        rd: 1,
        ra: 1,
        z: 0,
        rcode: 0,
        qdcount: 1,
        ancount: 1,
        nscount: 0,
        arcount: 0,
        qname: NAME,
        qtype: 1,
        qclass: 1,
        aname: NAME,
        aclass: 1,
        ttl: 0x0e10,
        rdata: DnsRdataContent::A(AContent {
            addr: [0x5d, 0xb8, 0xd8, 0x22],
        }),
    };
    assert_eq!(DnsWriter::to_vec(&content), DNS_RESPONSE);

    {
        use simple_dns::rdata::{A, RData};
        use simple_dns::{CLASS, Name, Packet, PacketFlag, Question, ResourceRecord, TYPE};
        let mut pkt = Packet::new_reply(0x1234);
        pkt.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        pkt.questions.push(Question::new(
            Name::new("example.com").unwrap(),
            TYPE::A.into(),
            CLASS::IN.into(),
            false,
        ));
        pkt.answers.push(ResourceRecord::new(
            Name::new("example.com").unwrap(),
            CLASS::IN,
            3600,
            RData::A(A {
                address: 0x5db8d822,
            }),
        ));
        let bytes = pkt.build_bytes_vec_compressed().unwrap();
        if bytes != DNS_RESPONSE {
            eprintln!(
                "dns_write: simple-dns output differs from fixture: {:02x?} vs {:02x?}",
                bytes, DNS_RESPONSE
            );
        }
    }

    let mut g = c.benchmark_group("dns_write");
    g.bench_function("bytesmith", |b| {
        b.iter(|| black_box(DnsWriter::to_vec(black_box(&content))))
    });
    g.bench_function("simple-dns", |b| {
        use simple_dns::rdata::{A, RData};
        use simple_dns::{CLASS, Name, Packet, PacketFlag, Question, ResourceRecord, TYPE};
        b.iter_batched(
            || {
                let mut pkt = Packet::new_reply(0x1234);
                pkt.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
                pkt.questions.push(Question::new(
                    Name::new("example.com").unwrap(),
                    TYPE::A.into(),
                    CLASS::IN.into(),
                    false,
                ));
                pkt.answers.push(ResourceRecord::new(
                    Name::new("example.com").unwrap(),
                    CLASS::IN,
                    3600,
                    RData::A(A {
                        address: 0x5db8d822,
                    }),
                ));
                pkt
            },
            |pkt| black_box(pkt.build_bytes_vec_compressed().unwrap()),
            criterion::BatchSize::SmallInput,
        )
    });
    g.finish();
}

criterion_group!(benches, dns, dns_write);
criterion_main!(benches);
