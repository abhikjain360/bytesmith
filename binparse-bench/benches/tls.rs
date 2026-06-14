//! TLS record: binparse vs tls-parser (nom-based, rusticata).
//!
//! Compared against `parse_tls_raw_record`, which — like binparse's `TlsRecord`
//! — only frames the record header and exposes the fragment as a raw slice
//! (it does not parse the handshake messages inside). The fragment bytes are
//! folded into a checksum so both do equivalent work.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use binparse_bench::*;

fn sum_bp(it: impl Iterator<Item = binparse::ParseResult<u8>>) -> u64 {
    it.map(|b| b.unwrap() as u64).fold(0, u64::wrapping_add)
}

fn fold(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0u64, |a, &b| a.wrapping_add(b as u64))
}

fn tls_record(c: &mut Criterion) {
    let mut g = c.benchmark_group("tls_record");
    g.bench_function("binparse", |b| {
        b.iter(|| {
            let (rec, _) = binparse_protocols::tls::TlsRecord::parse(black_box(TLS_RECORD)).unwrap();
            black_box(
                rec.content_type() as u64
                    ^ rec.legacy_major() as u64
                    ^ rec.legacy_minor() as u64
                    ^ rec.length() as u64
                    ^ sum_bp(rec.fragment().unwrap()),
            )
        })
    });
    g.bench_function("tls-parser", |b| {
        b.iter(|| {
            let (_, rec) = tls_parser::parse_tls_raw_record(black_box(TLS_RECORD)).unwrap();
            black_box(
                rec.hdr.record_type.0 as u64
                    ^ rec.hdr.version.0 as u64
                    ^ rec.hdr.len as u64
                    ^ fold(rec.data),
            )
        })
    });
    g.finish();
}

criterion_group!(benches, tls_record);
criterion_main!(benches);
