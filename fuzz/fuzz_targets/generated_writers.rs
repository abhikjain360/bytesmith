#![no_main]

use libfuzzer_sys::fuzz_target;

include!(concat!(env!("OUT_DIR"), "/generated_writers.rs"));

fn read_leb128(data: &[u8], ctx: binparse::HookContext<'_>) -> binparse::ParseResult<(u64, usize)> {
    binparse::hooks::leb128_unsigned(data, ctx)
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn byte(&mut self) -> u8 {
        let b = self.data.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b
    }

    fn u16(&mut self) -> u16 {
        u16::from(self.byte()) << 8 | u16::from(self.byte())
    }

    fn u32(&mut self) -> u32 {
        u32::from(self.u16()) << 16 | u32::from(self.u16())
    }

    fn i16(&mut self) -> i16 {
        self.u16() as i16
    }

    fn array<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0u8; N];
        for slot in out.iter_mut() {
            *slot = self.byte();
        }
        out
    }

    fn slice(&mut self, max: usize) -> &'a [u8] {
        let want = usize::from(self.byte()) % (max + 1);
        let start = self.pos.min(self.data.len());
        let end = (start + want).min(self.data.len());
        self.pos = end;
        &self.data[start..end]
    }
}

fuzz_target!(|data: &[u8]| {
    let mut c = Cursor::new(data);

    {
        let content = WPrimContent {
            a: c.byte(),
            word: c.u16(),
            be: c.u32(),
            sword: c.i16(),
        };
        let bytes = WPrimWriter::to_vec(&content);
        let mut buf = vec![0u8; WPrimWriter::SIZE];
        WPrimWriter::write_into(&mut buf, &content).unwrap();
        assert_eq!(bytes, buf);
        let (p, _) = WPrim::parse(&bytes).unwrap();
        assert_eq!(p.a(), content.a);
        assert_eq!(p.word(), content.word);
        assert_eq!(p.be(), content.be);
        assert_eq!(p.sword(), content.sword);
    }

    {
        let content = WBitsContent {
            flag_a: c.byte() & 0xf,
            flag_b: c.byte() & 0xf,
            ttl: c.byte(),
            total: c.u16(),
        };
        let bytes = WBitsWriter::to_vec(&content);
        let (p, _) = WBits::parse(&bytes).unwrap();
        assert_eq!(p.flag_a(), content.flag_a);
        assert_eq!(p.flag_b(), content.flag_b);
        assert_eq!(p.ttl(), content.ttl);
        assert_eq!(p.total(), content.total);
    }

    {
        let content = WFixedArrContent {
            tag: c.byte(),
            bytes: c.array::<4>(),
            tail: c.u16(),
        };
        let bytes = WFixedArrWriter::to_vec(&content);
        let mut buf = vec![0u8; WFixedArrWriter::SIZE];
        WFixedArrWriter::write_into(&mut buf, &content).unwrap();
        let (p, _) = WFixedArr::parse(&bytes).unwrap();
        assert_eq!(p.tag(), content.tag);
        assert_eq!(p.tail(), content.tail);
        assert_eq!(
            p.bytes().unwrap().collect::<binparse::ParseResult<Vec<u8>>>().unwrap(),
            content.bytes.to_vec()
        );
    }

    {
        let content = WNestedContent {
            tag: c.byte(),
            inner: WInnerContent {
                a: c.byte(),
                b: c.u16(),
            },
            trailer: c.byte(),
        };
        let bytes = WNestedWriter::to_vec(&content);
        let (p, _) = WNested::parse(&bytes).unwrap();
        assert_eq!(p.tag(), content.tag);
        assert_eq!(p.trailer(), content.trailer);
        let inner = p.inner().unwrap();
        assert_eq!(inner.a(), content.inner.a);
        assert_eq!(inner.b(), content.inner.b);
    }

    {
        let payload = c.slice(u8::MAX as usize);
        let content = WLenTailContent {
            kind: c.byte(),
            payload,
        };
        let lens = WLenTailLens {
            payload: payload.len(),
        };
        let bytes = WLenTailWriter::to_vec(&content);
        let mut buf = vec![0u8; WLenTailWriter::encoded_len(&lens)];
        WLenTailWriter::write_into(&mut buf, &content).unwrap();
        assert_eq!(bytes, buf);
        let (p, _) = WLenTail::parse(&bytes).unwrap();
        assert_eq!(p.kind(), content.kind);
        assert_eq!(usize::from(p.len()), payload.len());
        assert_eq!(
            p.payload().unwrap().collect::<binparse::ParseResult<Vec<u8>>>().unwrap(),
            payload.to_vec()
        );
    }

    {
        let body = c.slice(64);
        let content = WVarintContent {
            tag: c.byte(),
            body,
        };
        let bytes = WVarintWriter::to_vec(&content);
        let (p, _) = WVarint::parse(&bytes).unwrap();
        assert_eq!(p.tag(), content.tag);
        assert_eq!(p.len().unwrap() as usize, body.len());
        assert_eq!(
            p.body().unwrap().collect::<binparse::ParseResult<Vec<u8>>>().unwrap(),
            body.to_vec()
        );
    }

    {
        let body = if c.byte() & 1 == 0 {
            WUnionBodyContent::WConnect(WConnectContent { keep_alive: c.u16() })
        } else {
            WUnionBodyContent::WConnack(WConnackContent {
                ack: c.byte(),
                code: c.byte(),
            })
        };
        let content = WUnionContent { body };
        let bytes = WUnionWriter::to_vec(&content);
        let (p, _) = WUnion::parse(&bytes).unwrap();
        match (content.body, p.body().unwrap()) {
            (WUnionBodyContent::WConnect(cc), WUnion_body::WConnect(rc)) => {
                assert_eq!(p.kind(), 1);
                assert_eq!(rc.keep_alive(), cc.keep_alive);
            }
            (WUnionBodyContent::WConnack(cc), WUnion_body::WConnack(rc)) => {
                assert_eq!(p.kind(), 2);
                assert_eq!(rc.ack(), cc.ack);
                assert_eq!(rc.code(), cc.code);
            }
            _ => panic!("union variant mismatch after round trip"),
        }
    }

    {
        let dst = c.array::<6>();
        let src = c.array::<6>();
        let ethertype = c.u16();
        let payload = c.slice(64);
        let content = WEthernetContent {
            dst,
            src,
            ethertype,
            payload,
        };
        let lens = WEthernetLens {
            payload: payload.len(),
        };
        let bytes = WEthernetWriter::to_vec(&content);
        let mut buf = vec![0u8; WEthernetWriter::encoded_len(&lens)];
        WEthernetWriter::write_into(&mut buf, &content).unwrap();
        assert_eq!(bytes, buf);
        let (p, _) = WEthernet::parse(&bytes).unwrap();
        assert_eq!(p.ethertype(), ethertype);
        assert_eq!(
            p.dst().unwrap().collect::<binparse::ParseResult<Vec<u8>>>().unwrap(),
            dst.to_vec()
        );
        assert_eq!(
            p.src().unwrap().collect::<binparse::ParseResult<Vec<u8>>>().unwrap(),
            src.to_vec()
        );
        assert_eq!(
            p.payload().unwrap().collect::<binparse::ParseResult<Vec<u8>>>().unwrap(),
            payload.to_vec()
        );
    }

    {
        let dst = c.array::<6>();
        let src = c.array::<6>();
        let payload = c.slice(64);
        let content = WVlanContent {
            dst,
            src,
            pcp: c.byte() & 0x7,
            dei: c.byte() & 0x1,
            vid_hi: c.byte() & 0xf,
            vid_lo: c.byte(),
            ethertype: c.u16(),
            payload,
        };
        let bytes = WVlanWriter::to_vec(&content);
        let (p, _) = WVlan::parse(&bytes).unwrap();
        assert_eq!(p.tpid(), 0x8100);
        assert_eq!(p.pcp(), content.pcp);
        assert_eq!(p.dei(), content.dei);
        assert_eq!(p.vid_hi(), content.vid_hi);
        assert_eq!(p.vid_lo(), content.vid_lo);
        assert_eq!(p.ethertype(), content.ethertype);
        assert_eq!(
            p.payload().unwrap().collect::<binparse::ParseResult<Vec<u8>>>().unwrap(),
            payload.to_vec()
        );
    }
});
