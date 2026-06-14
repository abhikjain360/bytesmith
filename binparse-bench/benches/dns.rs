//! DNS message: binparse vs simple-dns.
//!
//! Both read id + the query name (as a list of labels) + the answer A address.
//! - `simple-dns` reads the name zero-copy via `Name::as_bytes()` — borrowed
//!   label slices into the packet, no allocation.
//! - `binparse`'s `dns_name` hook returns a lazy `NameRef<'a>` view whose
//!   `labels()` iterator borrows the label bytes from the packet — no
//!   allocation either. Combined with `@cache(len)` the offset re-walk is gone,
//!   so this is now borrowed-labels vs borrowed-labels: same shape.

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
    g.finish();
}

criterion_group!(benches, dns);
criterion_main!(benches);
