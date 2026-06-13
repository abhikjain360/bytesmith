#![no_main]

use libfuzzer_sys::fuzz_target;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

fn double_it(value: u16, _ctx: binparse::HookContext<'_>) -> binparse::ParseResult<u32> {
    Ok(u32::from(value) * 2)
}

fn parse_cstring(data: &[u8], ctx: binparse::HookContext<'_>) -> binparse::ParseResult<(String, usize)> {
    binparse::hooks::cstring(data, ctx)
}

fn read_leb128(data: &[u8], ctx: binparse::HookContext<'_>) -> binparse::ParseResult<(u64, usize)> {
    binparse::hooks::leb128_unsigned(data, ctx)
}

fn parse_dns_name(_data: &[u8], ctx: binparse::HookContext<'_>) -> binparse::ParseResult<(String, usize)> {
    let msg = ctx.enclosing;
    let mut labels: Vec<String> = Vec::new();
    let mut pos = ctx.offset;
    let mut consumed = None;
    let mut jumps = 0;
    loop {
        let len_byte = *msg.get(pos).ok_or(binparse::ParseError::NotEnoughData {
            expected: pos + 1,
            got: msg.len(),
        })?;
        if len_byte & 0xC0 == 0xC0 {
            let second = *msg.get(pos + 1).ok_or(binparse::ParseError::NotEnoughData {
                expected: pos + 2,
                got: msg.len(),
            })?;
            if consumed.is_none() {
                consumed = Some(pos + 2 - ctx.offset);
            }
            jumps += 1;
            if jumps > 8 {
                return Err(binparse::ParseError::HookFailed {
                    field: ctx.field,
                    reason: "too many DNS compression jumps",
                });
            }
            pos = (usize::from(len_byte & 0x3F) << 8) | usize::from(second);
        } else if len_byte == 0 {
            let consumed = consumed.unwrap_or_else(|| pos + 1 - ctx.offset);
            return Ok((labels.join("."), consumed));
        } else {
            let end = pos + 1 + usize::from(len_byte);
            let label = msg.get(pos + 1..end).ok_or(binparse::ParseError::NotEnoughData {
                expected: end,
                got: msg.len(),
            })?;
            labels.push(String::from_utf8_lossy(label).to_string());
            pos = end;
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let _ = Baseline::dissect(data);
    let _ = Hooked::dissect(data);
    let _ = StructArray::dissect(data);
    let _ = SizeExpr::dissect(data);
    let _ = Mixed::dissect(data);
    let _ = Conditional::dissect(data);
    let _ = Validated::dissect(data).errors();
    let _ = Rest::dissect(data);
    let _ = CStr::dissect(data);
    let _ = Capped::dissect(data);
    let _ = Opts::dissect(data);
    let _ = Padded::dissect(data);
    let _ = Dispatch::dissect(data).errors();
    let _ = ConcatUnion::dissect(data);
    let _ = Bounded::dissect(data);
    let _ = BoundedUnion::dissect(data);
    let _ = Varint::dissect(data);
    let _ = LenVarint::dissect(data);
    let _ = DnsMsg::dissect(data);
    let _ = StructBounded::dissect(data);
    let _ = StructBoundedNested::dissect(data);

    if let Ok((packet, _)) = Baseline::parse(data) {
        let _ = packet.field_tree();
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
            Ok(Baseline_payload::One(one)) => {
                let _ = one.x();
            }
            Ok(Baseline_payload::Unknown(_)) => {}
            Err(_) => {}
        }
    }

    if let Ok((packet, _)) = Hooked::parse(data) {
        let _ = packet.field_tree();
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

    if let Ok((packet, _)) = SizeExpr::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.xs_bit_range();
        if let Ok(xs) = packet.xs() {
            let _ = xs.collect::<binparse::ParseResult<Vec<_>>>();
        }
    }

    if let Ok((packet, _)) = Mixed::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.a();
        let _ = packet.b();
        let _ = packet.c();
        let _ = packet.version();
        let _ = packet.ihl();
        let _ = packet.low();
        let _ = packet.high();
        let _ = packet.vals_bit_range();
        if let Ok(vals) = packet.vals() {
            let _ = vals.collect::<binparse::ParseResult<Vec<_>>>();
        }
    }

    if let Ok((packet, _)) = Conditional::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.version();
        let _ = packet.ihl();
        if let Some(Ok(options)) = packet.options() {
            let _ = options.collect::<binparse::ParseResult<Vec<_>>>();
        }
        let _ = packet.big();
        let _ = packet.tail();
        let _ = packet.options_bit_range();
        let _ = packet.tail_bit_range();
    }

    if let Ok((packet, _)) = Validated::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.magic();
        let _ = packet.version();
        let _ = packet.ihl();
        let _ = packet.total_len();
        let _ = packet.reserved();
        let _ = packet.flags();
    }

    if let Ok((packet, _)) = Rest::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.n();
        let _ = packet.words_bit_range();
        if let Ok(words) = packet.words() {
            let _ = words.collect::<binparse::ParseResult<Vec<_>>>();
        }
    }

    if let Ok((packet, _)) = CStr::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.after();
        let _ = packet.name_bit_range();
        if let Ok(name) = packet.name() {
            let _ = name.collect::<binparse::ParseResult<Vec<_>>>();
        }
    }

    if let Ok((packet, _)) = Capped::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.count();
        let _ = packet.vals_bit_range();
        if let Ok(vals) = packet.vals() {
            let _ = vals.collect::<binparse::ParseResult<Vec<_>>>();
        }
    }

    if let Ok((packet, _)) = Opts::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.opts_bit_range();
        if let Ok(opts) = packet.opts() {
            for opt in opts.flatten() {
                let _ = opt.kind();
                let _ = opt.body();
            }
        }
    }

    if let Ok((packet, _)) = Padded::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.flags();
        let _ = packet.n();
        let _ = packet.tail();
        let _ = packet.data_bit_range();
        let _ = packet.tail_bit_range();
        if let Ok(items) = packet.data() {
            let _ = items.collect::<binparse::ParseResult<Vec<_>>>();
        }
    }

    if let Ok((packet, _)) = Dispatch::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.kind();
        let _ = packet.body_bit_range();
        match packet.body() {
            Ok(Dispatch_body::Msg(msg)) => {
                let _ = msg.msg_len();
                if let Ok(bytes) = msg.data() {
                    let _ = bytes.collect::<binparse::ParseResult<Vec<_>>>();
                }
            }
            Ok(Dispatch_body::Checked(checked)) => {
                let _ = checked.version();
            }
            Err(Error::UNKNOWN_KIND { kind }) => {
                let _ = kind;
            }
            Err(Error::Parse(_)) => {}
        }
    }

    if let Ok((packet, _)) = ConcatUnion::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.tail();
        let _ = packet.pair_bit_range();
        let (first, second, third) = packet.pair();
        let _ = first;
        if let Ok(ConcatUnion_pair_1::Word(word)) = second {
            let _ = word.w();
        }
        if let Ok(ConcatUnion_pair_2::Bytes(bytes)) = third {
            let _ = bytes.count();
            if let Ok(items) = bytes.data() {
                let _ = items.collect::<binparse::ParseResult<Vec<_>>>();
            }
        }
    }

    if let Ok((packet, _)) = Bounded::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.tag();
        let _ = packet.length();
        let _ = packet.value_bit_range();
        if let Ok(inner) = packet.value() {
            let _ = inner.a();
            let _ = inner.b();
        }
        let _ = packet.value_rest();
        let _ = packet.after();
    }

    if let Ok((packet, _)) = BoundedUnion::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.tag();
        let _ = packet.length();
        let _ = packet.value_bit_range();
        let _ = packet.value_rest();
        let _ = packet.after();
        match packet.value() {
            Ok(BoundedUnion_value::Pair(p)) => {
                let _ = p.inner();
            }
            Ok(BoundedUnion_value::Blob(b)) => {
                if let Ok(bytes) = b.bytes() {
                    let _ = bytes.collect::<binparse::ParseResult<Vec<_>>>();
                }
            }
            Ok(BoundedUnion_value::Unknown(_)) => {}
            Err(_) => {}
        }
    }

    if let Ok((packet, _)) = Varint::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.tag();
        let _ = packet.value();
        let _ = packet.value_bit_range();
        let _ = packet.after();
    }

    if let Ok((packet, _)) = LenVarint::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.n();
        let _ = packet.value();
        let _ = packet.value_rest();
        let _ = packet.value_bit_range();
        let _ = packet.after();
    }

    if let Ok((packet, _)) = DnsMsg::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.id();
        let _ = packet.qname();
        let _ = packet.qtype();
        let _ = packet.aname();
        let _ = packet.atype();
        let _ = packet.qname_bit_range();
        let _ = packet.aname_bit_range();
    }

    if let Ok((packet, _)) = StructBounded::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.total_len();
        let _ = packet.payload_bit_range();
        if let Ok(payload) = packet.payload() {
            let _ = payload.collect::<binparse::ParseResult<Vec<_>>>();
        }
    }

    if let Ok((packet, _)) = StructBoundedNested::parse(data) {
        let _ = packet.field_tree();
        let _ = packet.after();
        if let Ok(inner) = packet.inner() {
            let _ = inner.n();
            if let Ok(body) = inner.body() {
                let _ = body.collect::<binparse::ParseResult<Vec<_>>>();
            }
        }
    }
});
