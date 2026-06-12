#![no_main]

use libfuzzer_sys::fuzz_target;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

fn double_it(value: u16) -> u32 {
    u32::from(value) * 2
}

fn parse_cstring(data: &[u8]) -> (String, usize) {
    binparse::hooks::cstring(data)
}

fuzz_target!(|data: &[u8]| {
    if let Ok((packet, _)) = Baseline::parse(data) {
        let _ = packet.n();
        let _ = packet.word();
        let _ = packet.be();
        let _ = packet.flag_a();
        let _ = packet.flag_b();
        if let Ok(fixed) = packet.fixed() {
            let _ = fixed.collect::<binparse::ParseResult<Vec<_>>>();
        }
        if let Ok(inner) = packet.inner() {
            let _ = inner.a();
            let _ = inner.b();
        }
        if let Ok(dyns) = packet.dyns() {
            let _ = dyns.collect::<binparse::ParseResult<Vec<_>>>();
        }
        let _ = packet.dyns_bit_range();
        let _ = packet.payload_bit_range();
        let _ = packet.pair();
        match packet.payload() {
            Baseline_payload::One(one) => {
                let _ = one.x();
            }
            Baseline_payload::Unknown(_) => {}
        }
    }

    if let Ok((packet, _)) = Hooked::parse(data) {
        let _ = packet.prefix();
        let _ = packet.value();
        let _ = packet.name();
        let _ = packet.name_bit_range();
    }

    if let Ok((packet, _)) = StructArray::parse(data)
        && let Ok(items) = packet.items()
    {
        let _ = packet.items_bit_range();
        for item in items.flatten() {
            let _ = item.a();
            let _ = item.b();
        }
    }
});
