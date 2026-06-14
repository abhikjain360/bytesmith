//! DNS message: binparse vs simple-dns.
//!
//! Both decompress the query name (binparse via its `dns_name` hook, simple-dns
//! internally) and read the answer A address. simple-dns eagerly decodes the
//! whole message into owned `Vec`s; binparse decodes only the fields touched
//! here (id, qname, the A rdata) and leaves the rest untouched — so this is
//! eager-full-decode vs lazy-field-access.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use binparse_bench::*;

fn sum_bp(it: impl Iterator<Item = binparse::ParseResult<u8>>) -> u64 {
    it.map(|b| b.unwrap() as u64).fold(0, u64::wrapping_add)
}

fn dns(c: &mut Criterion) {
    let mut g = c.benchmark_group("dns");
    g.bench_function("binparse", |b| {
        use binparse_protocols::dns::{Dns, Dns_rdata};
        b.iter(|| {
            let (dns, _) = Dns::parse(black_box(DNS_RESPONSE)).unwrap();
            let addr = match dns.rdata().unwrap() {
                Dns_rdata::A(a) => sum_bp(a.addr().unwrap()),
                _ => 0,
            };
            black_box(dns.id() as u64 ^ dns.qname().unwrap().len() as u64 ^ addr)
        })
    });
    g.bench_function("simple-dns", |b| {
        use simple_dns::{Packet, rdata::RData};
        b.iter(|| {
            let pkt = Packet::parse(black_box(DNS_RESPONSE)).unwrap();
            let qlen = pkt.questions[0].qname.to_string().len() as u64;
            let addr = match &pkt.answers[0].rdata {
                RData::A(a) => a.address as u64,
                _ => 0,
            };
            black_box(pkt.id() as u64 ^ qlen ^ addr)
        })
    });
    g.finish();
}

criterion_group!(benches, dns);
criterion_main!(benches);
