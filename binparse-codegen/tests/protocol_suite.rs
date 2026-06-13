use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn generated_code(dsl: &str) -> String {
    let ast = binparse_dsl_parse::parse_str(dsl).expect("failed to parse DSL");
    binparse_codegen::CodeGen::generate(&ast).expect("failed to generate code")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("binparse-codegen should have a workspace parent")
        .to_path_buf()
}

fn write_runtime_crate(code: &str) -> PathBuf {
    let root = workspace_root();
    let test_dir = root
        .join("target")
        .join("generated-runtime-tests")
        .join(format!("protocols-{}", std::process::id()));

    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(test_dir.join("src")).expect("failed to create runtime test crate");

    let snapshot_dir = root.join("binparse-codegen").join("tests").join("snapshots");
    fs::create_dir_all(&snapshot_dir).expect("failed to create snapshot dir");

    let binparse_path = root.join("binparse");
    fs::write(
        test_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "protocol-suite-test"
version = "0.0.0"
edition = "2024"

[dependencies]
binparse = {{ path = "{}" }}

[workspace]
"#,
            binparse_path.display()
        ),
    )
    .expect("failed to write runtime Cargo.toml");

    fs::write(
        test_dir.join("src/lib.rs"),
        format!(
            r#"
fn parse_dns_name(_data: &[u8], ctx: binparse::HookContext<'_>) -> binparse::ParseResult<(String, usize)> {{
    let msg = ctx.enclosing;
    let mut labels: Vec<String> = Vec::new();
    let mut pos = ctx.offset;
    let mut consumed = None;
    let mut jumps = 0;
    loop {{
        let len_byte = *msg.get(pos).ok_or(binparse::ParseError::NotEnoughData {{
            expected: pos + 1,
            got: msg.len(),
        }})?;
        if len_byte & 0xC0 == 0xC0 {{
            let second = *msg.get(pos + 1).ok_or(binparse::ParseError::NotEnoughData {{
                expected: pos + 2,
                got: msg.len(),
            }})?;
            if consumed.is_none() {{
                consumed = Some(pos + 2 - ctx.offset);
            }}
            jumps += 1;
            if jumps > 8 {{
                return Err(binparse::ParseError::HookFailed {{
                    field: ctx.field,
                    reason: "too many DNS compression jumps",
                }});
            }}
            pos = (usize::from(len_byte & 0x3F) << 8) | usize::from(second);
        }} else if len_byte == 0 {{
            let consumed = consumed.unwrap_or_else(|| pos + 1 - ctx.offset);
            return Ok((labels.join("."), consumed));
        }} else {{
            let end = pos + 1 + usize::from(len_byte);
            let label = msg.get(pos + 1..end).ok_or(binparse::ParseError::NotEnoughData {{
                expected: end,
                got: msg.len(),
            }})?;
            labels.push(String::from_utf8_lossy(label).to_string());
            pos = end;
        }}
    }}
}}

{code}

#[cfg(test)]
mod tests {{
    use super::*;

    const SNAPSHOT_DIR: &str = "{snapshot_dir}";

    fn assert_tree_snapshot(name: &str, tree: binparse::FieldNode<'_>) {{
        let rendered = tree.render();
        let path = std::path::Path::new(SNAPSHOT_DIR).join(format!("{{name}}.txt"));
        if std::env::var_os("BLESS").is_some() {{
            std::fs::write(&path, &rendered).expect("failed to write snapshot");
            return;
        }}
        let expected = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            rendered, expected,
            "tree snapshot mismatch for {{name}}; rerun with BLESS=1 to regenerate"
        );
    }}

    fn assert_parse_no_panic<F>(name: &str, data: &[u8], parse: F)
    where
        F: Fn(&[u8]),
    {{
        for len in 0..=data.len() {{
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {{
                parse(&data[..len]);
            }}));
            assert!(result.is_ok(), "{{name}} panicked at len {{len}}");
        }}
    }}

    fn arp_request() -> Vec<u8> {{
        vec![
            0x00, 0x01, 0x08, 0x00, 0x06, 0x04, 0x00, 0x01,
            0x00, 0x0b, 0x82, 0x01, 0xfc, 0x42, 0xc0, 0xa8, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0xa8, 0x00, 0x02,
        ]
    }}

    fn ethernet_arp_frame() -> Vec<u8> {{
        let mut frame = vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
        frame.extend([0x00, 0x0b, 0x82, 0x01, 0xfc, 0x42]);
        frame.extend([0x08, 0x06]);
        frame.extend(arp_request());
        frame
    }}

    fn vlan_frame() -> Vec<u8> {{
        let mut frame = vec![0x00, 0x1b, 0x21, 0x3c, 0x9d, 0xf8];
        frame.extend([0x00, 0x19, 0x06, 0xea, 0xb8, 0xc1]);
        frame.extend([0x81, 0x00, 0x00, 0x64, 0x08, 0x00]);
        frame.extend([0xde, 0xad, 0xbe, 0xef]);
        frame
    }}

    fn icmp_echo_request() -> Vec<u8> {{
        let mut packet = vec![0x08, 0x00, 0xf7, 0x4b, 0x00, 0x01, 0x00, 0x09];
        packet.extend((0..32u8).map(|i| b'a' + (i % 26)));
        packet
    }}

    fn ipv4_ping_packet() -> Vec<u8> {{
        let mut packet = vec![
            0x45, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00, 0x40, 0x01, 0xb1, 0xe6,
            0xac, 0x10, 0x0a, 0x63, 0xac, 0x10, 0x0a, 0x0c,
        ];
        packet.extend(icmp_echo_request());
        packet
    }}

    fn ipv4_igmp_packet() -> Vec<u8> {{
        vec![
            0x46, 0xc0, 0x00, 0x20, 0x00, 0x00, 0x40, 0x00, 0x01, 0x02, 0x41, 0x2c,
            0xc0, 0xa8, 0x01, 0x64, 0xe0, 0x00, 0x00, 0xfb,
            0x94, 0x04, 0x00, 0x00,
            0x16, 0x00, 0x09, 0x04, 0xe0, 0x00, 0x00, 0xfb,
        ]
    }}

    fn udp_datagram() -> Vec<u8> {{
        vec![0xc3, 0x50, 0x00, 0x35, 0x00, 0x0c, 0x1a, 0x2b, 0xde, 0xad, 0xbe, 0xef]
    }}

    fn ipv6_udp_packet() -> Vec<u8> {{
        let mut packet = vec![0x60, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x11, 0x40];
        packet.extend([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01]);
        packet.extend([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x02]);
        packet.extend(udp_datagram());
        packet
    }}

    fn tcp_syn() -> Vec<u8> {{
        vec![
            0xc2, 0x09, 0x00, 0x50, 0x5e, 0x4a, 0x1b, 0x3c, 0x00, 0x00, 0x00, 0x00,
            0x70, 0x02, 0xfa, 0xf0, 0xbe, 0xef, 0x00, 0x00,
            0x02, 0x04, 0x05, 0xb4, 0x01, 0x01, 0x04, 0x02,
        ]
    }}

    fn tcp_ack_with_payload() -> Vec<u8> {{
        vec![
            0xc2, 0x09, 0x00, 0x50, 0x5e, 0x4a, 0x1b, 0x3d, 0x11, 0x22, 0x33, 0x44,
            0x50, 0x10, 0xfa, 0xf0, 0x12, 0x34, 0x00, 0x00,
            b'G', b'E', b'T', b' ',
        ]
    }}

    fn dns_response() -> Vec<u8> {{
        vec![
            0x12, 0x34, 0x81, 0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
            7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0,
            0x00, 0x01, 0x00, 0x01,
            0xc0, 0x0c,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x0e, 0x10, 0x00, 0x04,
            0x5d, 0xb8, 0xd8, 0x22,
        ]
    }}

    fn tls_client_hello_record() -> Vec<u8> {{
        vec![0x16, 0x03, 0x01, 0x00, 0x05, 0x01, 0x00, 0x00, 0x01, 0x00]
    }}

    fn tls_record_stream() -> Vec<u8> {{
        vec![
            0x14, 0x03, 0x03, 0x00, 0x01, 0x01,
            0x16, 0x03, 0x03, 0x00, 0x02, 0xaa, 0xbb,
        ]
    }}

    #[test]
    fn ethernet_frame_decodes() {{
        let frame = ethernet_arp_frame();
        let (packet, rem) = EthernetII::parse(&frame).unwrap();
        assert!(rem.is_empty());
        let dst = packet
            .dst()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(dst, vec![0xff; 6]);
        let src = packet
            .src()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(src, vec![0x00, 0x0b, 0x82, 0x01, 0xfc, 0x42]);
        assert_eq!(packet.ethertype(), 0x0806);
        let payload = packet
            .payload()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(payload, arp_request());
        assert_parse_no_panic("EthernetII", &frame, |data| {{
            let _ = EthernetII::parse(data);
        }});
    }}

    #[test]
    fn ethernet_truncated_errors() {{
        let frame = ethernet_arp_frame();
        assert!(matches!(
            EthernetII::parse(&frame[..13]),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
    }}

    #[test]
    fn vlan_frame_decodes() {{
        let frame = vlan_frame();
        let (packet, rem) = Vlan::parse(&frame).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.pcp(), 0);
        assert_eq!(packet.dei(), 0);
        let vid = (u16::from(packet.vid_hi()) << 8) | u16::from(packet.vid_lo());
        assert_eq!(vid, 100);
        assert_eq!(packet.ethertype(), 0x0800);
        let payload = packet
            .payload()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(payload, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_parse_no_panic("Vlan", &frame, |data| {{
            let _ = Vlan::parse(data);
        }});
    }}

    #[test]
    fn vlan_bad_tpid_errors() {{
        let mut frame = vlan_frame();
        frame[12] = 0x91;
        assert_eq!(
            Vlan::parse(&frame).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "Vlan.tpid",
                actual: 0x9100,
            }}
        );
    }}

    #[test]
    fn arp_request_decodes() {{
        let packet_bytes = arp_request();
        let (packet, rem) = Arp::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.htype(), 1);
        assert_eq!(packet.ptype(), 0x0800);
        assert_eq!(packet.hlen(), 6);
        assert_eq!(packet.plen(), 4);
        assert_eq!(packet.oper(), 1);
        let sha = packet
            .sha()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(sha, vec![0x00, 0x0b, 0x82, 0x01, 0xfc, 0x42]);
        let spa = packet
            .spa()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(spa, vec![192, 168, 0, 1]);
        let tha = packet
            .tha()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(tha, vec![0; 6]);
        let tpa = packet
            .tpa()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(tpa, vec![192, 168, 0, 2]);
        assert_parse_no_panic("Arp", &packet_bytes, |data| {{
            let _ = Arp::parse(data);
        }});
    }}

    #[test]
    fn arp_bad_oper_errors() {{
        let mut packet_bytes = arp_request();
        packet_bytes[7] = 3;
        assert_eq!(
            Arp::parse(&packet_bytes).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "Arp.oper",
                actual: 3,
            }}
        );
    }}

    #[test]
    fn arp_truncated_errors() {{
        let packet_bytes = arp_request();
        assert!(matches!(
            Arp::parse(&packet_bytes[..27]),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
    }}

    #[test]
    fn ipv4_base_header_decodes() {{
        let packet_bytes = ipv4_ping_packet();
        let (packet, rem) = Ipv4::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.version(), 4);
        assert_eq!(packet.ihl(), 5);
        assert_eq!(packet.dscp(), 0);
        assert_eq!(packet.ecn(), 0);
        assert_eq!(packet.total_len(), 60);
        assert_eq!(packet.ident(), 0x1c46);
        assert_eq!(packet.flags(), 2);
        assert_eq!(packet.frag_hi(), 0);
        assert_eq!(packet.frag_lo(), 0);
        assert_eq!(packet.ttl(), 64);
        assert_eq!(packet.proto(), 1);
        assert_eq!(packet.checksum(), 0xb1e6);
        let src = packet
            .src()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(src, vec![172, 16, 10, 99]);
        let dst = packet
            .dst()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(dst, vec![172, 16, 10, 12]);
        assert!(packet.options().is_none());
        let payload = packet
            .payload()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(payload, icmp_echo_request());
        assert_eq!(
            payload.len(),
            usize::from(packet.total_len()) - usize::from(packet.ihl()) * 4
        );
        let mut padded = packet_bytes.clone();
        padded.extend([0xde, 0xad]);
        let (packet, rem) = Ipv4::parse(&padded).unwrap();
        assert_eq!(rem, &[0xde, 0xad]);
        let payload = packet
            .payload()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(payload, icmp_echo_request());
        assert_parse_no_panic("Ipv4", &packet_bytes, |data| {{
            let _ = Ipv4::parse(data);
        }});
    }}

    #[test]
    fn ipv4_options_decode_when_ihl_exceeds_five() {{
        let packet_bytes = ipv4_igmp_packet();
        let (packet, rem) = Ipv4::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.version(), 4);
        assert_eq!(packet.ihl(), 6);
        assert_eq!(packet.dscp(), 48);
        assert_eq!(packet.total_len(), 32);
        assert_eq!(packet.ttl(), 1);
        assert_eq!(packet.proto(), 2);
        let options = packet
            .options()
            .expect("options should be present")
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(options, vec![0x94, 0x04, 0x00, 0x00]);
        let payload = packet
            .payload()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(payload, vec![0x16, 0x00, 0x09, 0x04, 0xe0, 0x00, 0x00, 0xfb]);
        assert_parse_no_panic("Ipv4 options", &packet_bytes, |data| {{
            let _ = Ipv4::parse(data);
        }});
    }}

    #[test]
    fn ipv4_bad_version_errors() {{
        let mut packet_bytes = ipv4_ping_packet();
        packet_bytes[0] = 0x65;
        assert_eq!(
            Ipv4::parse(&packet_bytes).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "Ipv4.version",
                actual: 6,
            }}
        );
    }}

    #[test]
    fn ipv4_bad_total_len_errors() {{
        let mut packet_bytes = ipv4_ping_packet();
        packet_bytes[2] = 0x00;
        packet_bytes[3] = 0x10;
        assert_eq!(
            Ipv4::parse(&packet_bytes).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "Ipv4.total_len",
                actual: 16,
            }}
        );
    }}

    #[test]
    fn ipv4_truncated_errors() {{
        let packet_bytes = ipv4_ping_packet();
        assert!(matches!(
            Ipv4::parse(&packet_bytes[..19]),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
        assert!(matches!(
            Ipv4::parse(&packet_bytes[..40]),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
    }}

    #[test]
    fn ipv6_header_decodes() {{
        let packet_bytes = ipv6_udp_packet();
        let (packet, rem) = Ipv6::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.version(), 6);
        assert_eq!(packet.tc_hi(), 0);
        assert_eq!(packet.tc_lo(), 0);
        assert_eq!(packet.flow_hi(), 0);
        assert_eq!(packet.flow_lo(), 0);
        assert_eq!(packet.payload_len(), 12);
        assert_eq!(packet.next_header(), 17);
        assert_eq!(packet.hop_limit(), 64);
        let src = packet
            .src()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(src[..4], [0x20, 0x01, 0x0d, 0xb8]);
        assert_eq!(src[15], 1);
        let dst = packet
            .dst()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(dst[15], 2);
        let payload = packet
            .payload()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(payload, udp_datagram());
        assert_parse_no_panic("Ipv6", &packet_bytes, |data| {{
            let _ = Ipv6::parse(data);
        }});
    }}

    #[test]
    fn ipv6_bad_version_errors() {{
        let mut packet_bytes = ipv6_udp_packet();
        packet_bytes[0] = 0x40;
        assert_eq!(
            Ipv6::parse(&packet_bytes).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "Ipv6.version",
                actual: 4,
            }}
        );
    }}

    #[test]
    fn ipv6_truncated_errors() {{
        let packet_bytes = ipv6_udp_packet();
        assert!(matches!(
            Ipv6::parse(&packet_bytes[..30]),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
    }}

    #[test]
    fn udp_datagram_decodes() {{
        let packet_bytes = udp_datagram();
        let (packet, rem) = Udp::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.src_port(), 50000);
        assert_eq!(packet.dst_port(), 53);
        assert_eq!(packet.length(), 12);
        assert_eq!(packet.checksum(), 0x1a2b);
        let payload = packet
            .payload()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(payload, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_parse_no_panic("Udp", &packet_bytes, |data| {{
            let _ = Udp::parse(data);
        }});
    }}

    #[test]
    fn udp_bad_length_errors() {{
        let mut packet_bytes = udp_datagram();
        packet_bytes[4] = 0x00;
        packet_bytes[5] = 0x04;
        assert_eq!(
            Udp::parse(&packet_bytes).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "Udp.length",
                actual: 4,
            }}
        );
    }}

    #[test]
    fn udp_truncated_errors() {{
        let mut packet_bytes = udp_datagram();
        packet_bytes[5] = 0x14;
        assert!(matches!(
            Udp::parse(&packet_bytes),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
    }}

    #[test]
    fn tcp_syn_with_options_decodes() {{
        let packet_bytes = tcp_syn();
        let (packet, rem) = Tcp::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.src_port(), 49673);
        assert_eq!(packet.dst_port(), 80);
        assert_eq!(packet.seq(), 0x5e4a1b3c);
        assert_eq!(packet.ack_no(), 0);
        assert_eq!(packet.data_offset(), 7);
        assert_eq!(packet.reserved(), 0);
        assert_eq!(packet.ns(), 0);
        assert_eq!(packet.cwr(), 0);
        assert_eq!(packet.ece(), 0);
        assert_eq!(packet.urg(), 0);
        assert_eq!(packet.ack(), 0);
        assert_eq!(packet.psh(), 0);
        assert_eq!(packet.rst(), 0);
        assert_eq!(packet.syn(), 1);
        assert_eq!(packet.fin(), 0);
        assert_eq!(packet.window(), 64240);
        assert_eq!(packet.checksum(), 0xbeef);
        assert_eq!(packet.urgent_ptr(), 0);

        let option_list = packet.options().unwrap();
        let opts = option_list
            .opts()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(opts.len(), 4);
        assert_eq!(opts[0].kind(), 2);
        match opts[0].body().unwrap() {{
            TcpOption_body::Generic(mss) => {{
                assert_eq!(mss.len(), 4);
                let data = mss
                    .data()
                    .unwrap()
                    .collect::<binparse::ParseResult<Vec<_>>>()
                    .unwrap();
                assert_eq!(data, vec![0x05, 0xb4]);
            }}
            _ => panic!("expected Generic option"),
        }}
        match opts[1].body().unwrap() {{
            TcpOption_body::Nop(_) => {{}}
            _ => panic!("expected Nop option"),
        }}
        match opts[2].body().unwrap() {{
            TcpOption_body::Nop(_) => {{}}
            _ => panic!("expected Nop option"),
        }}
        assert_eq!(opts[3].kind(), 4);
        match opts[3].body().unwrap() {{
            TcpOption_body::Generic(sack_ok) => {{
                assert_eq!(sack_ok.len(), 2);
                let data = sack_ok
                    .data()
                    .unwrap()
                    .collect::<binparse::ParseResult<Vec<_>>>()
                    .unwrap();
                assert!(data.is_empty());
            }}
            _ => panic!("expected Generic option"),
        }}

        let payload = packet
            .payload()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert!(payload.is_empty());
        assert_parse_no_panic("Tcp", &packet_bytes, |data| {{
            let _ = Tcp::parse(data);
        }});
    }}

    #[test]
    fn tcp_without_options_decodes() {{
        let packet_bytes = tcp_ack_with_payload();
        let (packet, rem) = Tcp::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.data_offset(), 5);
        assert_eq!(packet.ack(), 1);
        assert_eq!(packet.syn(), 0);
        assert_eq!(packet.ack_no(), 0x11223344);
        let option_list = packet.options().unwrap();
        let opts = option_list
            .opts()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert!(opts.is_empty());
        let payload = packet
            .payload()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(payload, b"GET ");
        assert_parse_no_panic("Tcp no options", &packet_bytes, |data| {{
            let _ = Tcp::parse(data);
        }});
    }}

    #[test]
    fn tcp_truncated_options_error() {{
        let packet_bytes = tcp_syn();
        assert!(matches!(
            Tcp::parse(&packet_bytes[..24]),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
    }}

    #[test]
    fn icmpv4_echo_decodes() {{
        let packet_bytes = icmp_echo_request();
        let (packet, rem) = Icmpv4::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.icmp_type(), 8);
        assert_eq!(packet.code(), 0);
        assert_eq!(packet.checksum(), 0xf74b);
        match packet.body().unwrap() {{
            Icmpv4_body::Echo(echo) => {{
                assert_eq!(echo.id(), 1);
                assert_eq!(echo.seq(), 9);
                let data = echo
                    .data()
                    .unwrap()
                    .collect::<binparse::ParseResult<Vec<_>>>()
                    .unwrap();
                assert_eq!(data, packet_bytes[8..].to_vec());
            }}
            _ => panic!("expected Echo body"),
        }}
        assert_parse_no_panic("Icmpv4", &packet_bytes, |data| {{
            let _ = Icmpv4::parse(data);
        }});
    }}

    #[test]
    fn icmpv4_dest_unreach_decodes() {{
        let mut packet_bytes = vec![0x03, 0x03, 0xa2, 0xb1, 0x00, 0x00, 0x00, 0x00];
        packet_bytes.extend(&ipv4_ping_packet()[..28]);
        let (packet, rem) = Icmpv4::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        match packet.body().unwrap() {{
            Icmpv4_body::DestUnreach(unreach) => {{
                assert_eq!(unreach.unused(), 0);
                let original = unreach
                    .original()
                    .unwrap()
                    .collect::<binparse::ParseResult<Vec<_>>>()
                    .unwrap();
                assert_eq!(original, ipv4_ping_packet()[..28].to_vec());
            }}
            _ => panic!("expected DestUnreach body"),
        }}
        assert_parse_no_panic("Icmpv4 unreach", &packet_bytes, |data| {{
            let _ = Icmpv4::parse(data);
        }});
    }}

    #[test]
    fn icmpv4_time_exceeded_and_raw_decode() {{
        let exceeded = [0x0b, 0x00, 0x12, 0x34, 0x00, 0x00, 0x00, 0x00];
        let (packet, rem) = Icmpv4::parse(&exceeded).unwrap();
        assert!(rem.is_empty());
        match packet.body().unwrap() {{
            Icmpv4_body::TimeExceeded(exceeded) => assert_eq!(exceeded.unused(), 0),
            _ => panic!("expected TimeExceeded body"),
        }}

        let raw = [40, 0, 0xde, 0xad];
        let (packet, rem) = Icmpv4::parse(&raw).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.checksum(), 0xdead);
        match packet.body().unwrap() {{
            Icmpv4_body::Raw(_) => {{}}
            _ => panic!("expected Raw body"),
        }}
    }}

    #[test]
    fn icmpv4_truncated_errors() {{
        let packet_bytes = icmp_echo_request();
        assert!(matches!(
            Icmpv4::parse(&packet_bytes[..6]),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
    }}

    #[test]
    fn dns_response_decodes_with_compression() {{
        let packet_bytes = dns_response();
        let (packet, rem) = Dns::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.id(), 0x1234);
        assert_eq!(packet.qr(), 1);
        assert_eq!(packet.opcode(), 0);
        assert_eq!(packet.aa(), 0);
        assert_eq!(packet.tc(), 0);
        assert_eq!(packet.rd(), 1);
        assert_eq!(packet.ra(), 1);
        assert_eq!(packet.z(), 0);
        assert_eq!(packet.rcode(), 0);
        assert_eq!(packet.qdcount(), 1);
        assert_eq!(packet.ancount(), 1);
        assert_eq!(packet.nscount(), 0);
        assert_eq!(packet.arcount(), 0);
        assert_eq!(packet.qname().unwrap(), "example.com");
        assert_eq!(packet.qtype(), 1);
        assert_eq!(packet.qclass(), 1);
        assert_eq!(packet.aname().unwrap(), "example.com");
        assert_eq!(packet.atype(), 1);
        assert_eq!(packet.aclass(), 1);
        assert_eq!(packet.ttl(), 3600);
        assert_eq!(packet.rdlength(), 4);
        match packet.rdata().unwrap() {{
            Dns_rdata::A(a) => {{
                let addr = a
                    .addr()
                    .unwrap()
                    .collect::<binparse::ParseResult<Vec<_>>>()
                    .unwrap();
                assert_eq!(addr, vec![93, 184, 216, 34]);
            }}
            _ => panic!("expected A record"),
        }}
        assert!(packet.rdata_rest().unwrap().is_empty());
        assert_parse_no_panic("Dns", &packet_bytes, |data| {{
            let _ = Dns::parse(data);
        }});
    }}

    #[test]
    fn dns_rdata_trailing_bytes_exposed_via_rest() {{
        let mut packet_bytes = dns_response();
        packet_bytes.extend([0xde, 0xad]);
        let rdlen = packet_bytes.len() - 41;
        packet_bytes[39] = (rdlen >> 8) as u8;
        packet_bytes[40] = rdlen as u8;
        let (packet, rem) = Dns::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        match packet.rdata().unwrap() {{
            Dns_rdata::A(a) => {{
                let addr = a
                    .addr()
                    .unwrap()
                    .collect::<binparse::ParseResult<Vec<_>>>()
                    .unwrap();
                assert_eq!(addr, vec![93, 184, 216, 34]);
            }}
            _ => panic!("expected A record"),
        }}
        assert_eq!(packet.rdata_rest().unwrap(), &[0xde, 0xad]);
    }}

    #[test]
    fn dns_rdlength_larger_than_packet_errors_not_panics() {{
        let mut packet_bytes = dns_response();
        packet_bytes[39] = 0xff;
        packet_bytes[40] = 0xff;
        assert!(matches!(
            Dns::parse(&packet_bytes),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
        assert_parse_no_panic("Dns", &packet_bytes, |data| {{
            let _ = Dns::parse(data);
            let _ = Dns::dissect(data);
        }});
    }}

    #[test]
    fn dns_truncated_rdata_dissect_partial_tree() {{
        let packet_bytes = dns_response();
        let tree = Dns::dissect(&packet_bytes[..44]);
        assert!(matches!(tree.status, binparse::Status::Error(_)));
        let rdlength = tree
            .children
            .iter()
            .find(|c| c.name == "rdlength")
            .expect("rdlength present");
        assert!(matches!(rdlength.status, binparse::Status::Ok));
        let rdata = tree
            .children
            .iter()
            .find(|c| c.name == "rdata")
            .expect("rdata node present");
        assert!(matches!(rdata.status, binparse::Status::Error(_)));
    }}

    #[test]
    fn dns_pointer_loop_errors() {{
        let mut packet_bytes = dns_response();
        packet_bytes[29] = 0xc0;
        packet_bytes[30] = 29;
        assert!(matches!(
            Dns::parse(&packet_bytes),
            Err(binparse::ParseError::HookFailed {{
                field: "Dns.aname",
                ..
            }})
        ));
    }}

    #[test]
    fn dns_dangling_pointer_errors() {{
        let mut packet_bytes = dns_response();
        packet_bytes[30] = 200;
        assert!(matches!(
            Dns::parse(&packet_bytes),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
    }}

    #[test]
    fn dns_truncated_rdata_errors() {{
        let packet_bytes = dns_response();
        assert!(matches!(
            Dns::parse(&packet_bytes[..43]),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
    }}

    #[test]
    fn tls_record_decodes() {{
        let packet_bytes = tls_client_hello_record();
        let (packet, rem) = TlsRecord::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.content_type(), 22);
        assert_eq!(packet.legacy_major(), 3);
        assert_eq!(packet.legacy_minor(), 1);
        assert_eq!(packet.length(), 5);
        let fragment = packet
            .fragment()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(fragment, vec![0x01, 0x00, 0x00, 0x01, 0x00]);
        assert_eq!(fragment.len(), usize::from(packet.length()));
        let mut padded = packet_bytes.clone();
        padded.extend([0x77, 0x88]);
        let (packet, rem) = TlsRecord::parse(&padded).unwrap();
        assert_eq!(rem, &[0x77, 0x88]);
        let fragment = packet
            .fragment()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(fragment, vec![0x01, 0x00, 0x00, 0x01, 0x00]);
        assert_parse_no_panic("TlsRecord", &packet_bytes, |data| {{
            let _ = TlsRecord::parse(data);
        }});
    }}

    #[test]
    fn tls_stream_decodes_multiple_records() {{
        let packet_bytes = tls_record_stream();
        let (packet, rem) = TlsStream::parse(&packet_bytes).unwrap();
        assert!(rem.is_empty());
        let records = packet
            .records()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].content_type(), 20);
        assert_eq!(records[0].length(), 1);
        assert_eq!(records[1].content_type(), 22);
        assert_eq!(records[1].legacy_minor(), 3);
        let fragment = records[1]
            .fragment()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(fragment, vec![0xaa, 0xbb]);
        assert_parse_no_panic("TlsStream", &packet_bytes, |data| {{
            let _ = TlsStream::parse(data);
        }});
    }}

    #[test]
    fn tls_bad_content_type_errors() {{
        let mut packet_bytes = tls_client_hello_record();
        packet_bytes[0] = 0x99;
        assert_eq!(
            TlsRecord::parse(&packet_bytes).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "TlsRecord.content_type",
                actual: 0x99,
            }}
        );
    }}

    #[test]
    fn tls_truncated_fragment_errors() {{
        let packet_bytes = [0x16, 0x03, 0x01, 0x00, 0x10, 0x01, 0x02, 0x03];
        assert!(matches!(
            TlsRecord::parse(&packet_bytes),
            Err(binparse::ParseError::NotEnoughData {{ .. }})
        ));
    }}

    fn ethernet_ipv4_udp_frame() -> Vec<u8> {{
        let mut frame = vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
        frame.extend([0x00, 0x0b, 0x82, 0x01, 0xfc, 0x42]);
        frame.extend([0x08, 0x00]);
        frame.extend([
            0x45, 0x00, 0x00, 0x20, 0x1c, 0x46, 0x40, 0x00, 0x40, 0x11, 0x00, 0x00,
            0xac, 0x10, 0x0a, 0x63, 0xac, 0x10, 0x0a, 0x0c,
        ]);
        frame.extend(udp_datagram());
        frame
    }}

    fn parse_for_key(key: u128, payload: &[u8]) -> Option<binparse::Handoff<'_>> {{
        match key {{
            0x0800 => Ipv4::parse(payload).ok().and_then(|(p, _)| p.handoff()),
            17 => Udp::parse(payload).ok().and_then(|(p, _)| p.handoff()),
            _ => None,
        }}
    }}

    #[test]
    fn handoff_chains_ethernet_ipv4_udp() {{
        let frame = ethernet_ipv4_udp_frame();
        let (eth, _) = EthernetII::parse(&frame).unwrap();
        let first = eth.handoff().expect("ethernet declares a payload");
        assert_eq!(first.keys, vec![0x0800]);

        let mut keys = first.keys;
        let mut payload: &[u8] = first.payload;
        while let Some(key) = keys.first().copied() {{
            match parse_for_key(key, payload) {{
                Some(next) => {{
                    keys = next.keys;
                    payload = next.payload;
                }}
                None => break,
            }}
        }}

        assert_eq!(keys, vec![50000, 53]);
        assert_eq!(payload, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_parse_no_panic("EthernetII handoff", &frame, |data| {{
            if let Ok((eth, _)) = EthernetII::parse(data) {{
                let _ = eth.handoff();
            }}
        }});
    }}

    fn assert_dissect_no_panic<F>(name: &str, data: &[u8], dissect: F)
    where
        F: for<'b> Fn(&'b [u8]) -> binparse::FieldNode<'b>,
    {{
        for len in 0..=data.len() {{
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {{
                let _ = dissect(&data[..len]).errors();
            }}));
            assert!(result.is_ok(), "{{name}} dissect panicked at len {{len}}");
        }}
    }}

    #[test]
    fn dissect_truncated_ipv4_yields_partial_tree() {{
        let packet = ipv4_ping_packet();
        let cut = 12;
        let tree = Ipv4::dissect(&packet[..cut]);
        assert_eq!(tree.children[0].name, "version");
        assert_eq!(tree.children[0].value, binparse::Value::UInt(4));
        let last = tree.children.last().unwrap();
        assert!(matches!(last.status, binparse::Status::Error(_)));
        assert!(matches!(tree.status, binparse::Status::Error(_)));
        assert!(!tree.errors().is_empty());
        assert_dissect_no_panic("Ipv4", &packet, |d| Ipv4::dissect(d));
    }}

    #[test]
    fn dissect_vlan_bad_tpid_is_recoverable_and_decodes_later_fields() {{
        let mut frame = vlan_frame();
        frame[12] = 0x88;
        frame[13] = 0x88;
        let tree = Vlan::dissect(&frame);
        let tpid = tree
            .children
            .iter()
            .find(|c| c.name == "tpid")
            .expect("tpid node present");
        assert!(matches!(
            tpid.status,
            binparse::Status::Error(binparse::ParseError::ValidationFailed {{
                field: "Vlan.tpid",
                ..
            }})
        ));
        assert!(tree.children.iter().any(|c| c.name == "ethertype"));
        assert_eq!(tree.status, binparse::Status::Ok);
        let errors = tree.errors();
        assert!(errors.iter().any(|(path, _)| *path == "Vlan.tpid"));
        assert_dissect_no_panic("Vlan", &frame, |d| Vlan::dissect(d));
    }}

    #[test]
    fn dissect_tls_truncated_fragment_yields_partial_tree() {{
        let record = tls_client_hello_record();
        let cut = record.len() - 2;
        let tree = TlsRecord::dissect(&record[..cut]);
        assert_eq!(tree.children[0].name, "content_type");
        assert!(tree.children.iter().any(|c| c.name == "fragment"));
        assert!(matches!(
            tree.status,
            binparse::Status::Error(binparse::ParseError::NotEnoughData {{ .. }})
        ));
        assert_dissect_no_panic("TlsRecord", &record, |d| TlsRecord::dissect(d));
    }}

    #[test]
    fn ethernet_tree_snapshot() {{
        assert_tree_snapshot("ethernet", EthernetII::dissect(&ethernet_arp_frame()));
    }}

    #[test]
    fn vlan_tree_snapshot() {{
        assert_tree_snapshot("vlan", Vlan::dissect(&vlan_frame()));
    }}

    #[test]
    fn arp_tree_snapshot() {{
        assert_tree_snapshot("arp", Arp::dissect(&arp_request()));
    }}

    #[test]
    fn ipv4_tree_snapshot() {{
        assert_tree_snapshot("ipv4", Ipv4::dissect(&ipv4_ping_packet()));
    }}

    #[test]
    fn ipv4_options_tree_snapshot() {{
        assert_tree_snapshot("ipv4_options", Ipv4::dissect(&ipv4_igmp_packet()));
    }}

    #[test]
    fn ipv4_truncated_tree_snapshot() {{
        assert_tree_snapshot("ipv4_truncated", Ipv4::dissect(&ipv4_ping_packet()[..12]));
    }}

    #[test]
    fn ipv6_tree_snapshot() {{
        assert_tree_snapshot("ipv6", Ipv6::dissect(&ipv6_udp_packet()));
    }}

    #[test]
    fn udp_tree_snapshot() {{
        assert_tree_snapshot("udp", Udp::dissect(&udp_datagram()));
    }}

    #[test]
    fn tcp_tree_snapshot() {{
        assert_tree_snapshot("tcp", Tcp::dissect(&tcp_ack_with_payload()));
    }}

    #[test]
    fn tcp_options_tree_snapshot() {{
        assert_tree_snapshot("tcp_options", Tcp::dissect(&tcp_syn()));
    }}

    #[test]
    fn icmpv4_tree_snapshot() {{
        assert_tree_snapshot("icmpv4", Icmpv4::dissect(&icmp_echo_request()));
    }}

    #[test]
    fn dns_tree_snapshot() {{
        assert_tree_snapshot("dns", Dns::dissect(&dns_response()));
    }}

    #[test]
    fn tls_record_tree_snapshot() {{
        assert_tree_snapshot("tls_record", TlsRecord::dissect(&tls_client_hello_record()));
    }}

    #[test]
    fn tls_stream_tree_snapshot() {{
        assert_tree_snapshot("tls_stream", TlsStream::dissect(&tls_record_stream()));
    }}
}}
"#,
            snapshot_dir = snapshot_dir.display(),
        ),
    )
    .expect("failed to write runtime lib.rs");

    test_dir
}

#[test]
fn protocol_suite_compiles_and_parses_real_packets() {
    let dsl = r#"
struct EthernetII {
    dst: [u8; 6],
    src: [u8; 6],
    @discriminator ethertype: u16,
    @greedy(unsafe_eof) @payload payload: [u8],
}

struct Vlan {
    dst: [u8; 6],
    src: [u8; 6],
    tpid = x8100,
    pcp: b<3>,
    dei: b<1>,
    vid_hi: b<4>,
    vid_lo: u8,
    ethertype: u16,
    @greedy(unsafe_eof) payload: [u8],
}

struct Arp {
    htype: u16,
    ptype: u16,
    hlen: u8,
    plen: u8,
    @range(1, 2) oper: u16,
    sha: [u8; hlen],
    spa: [u8; plen],
    tha: [u8; hlen],
    tpa: [u8; plen],
}

@len(total_len)
struct Ipv4 {
    @check(version == 4) version: b<4>,
    @range(5, 15) ihl: b<4>,
    dscp: b<6>,
    ecn: b<2>,
    @range(20, 65535) total_len: u16,
    ident: u16,
    flags: b<3>,
    frag_hi: b<5>,
    frag_lo: u8,
    ttl: u8,
    @discriminator proto: u8,
    checksum: u16,
    src: [u8; 4],
    dst: [u8; 4],
    if (ihl > 5) {
        options: [u8; (ihl - 5) * 4],
    }
    @payload payload: [u8],
}

struct Ipv6 {
    @check(version == 6) version: b<4>,
    tc_hi: b<4>,
    tc_lo: b<4>,
    flow_hi: b<4>,
    flow_lo: u16,
    payload_len: u16,
    next_header: u8,
    hop_limit: u8,
    src: [u8; 16],
    dst: [u8; 16],
    payload: [u8; payload_len],
}

struct Udp {
    @discriminator src_port: u16,
    @discriminator dst_port: u16,
    @range(8, 65535) length: u16,
    checksum: u16,
    @payload payload: [u8; length - 8],
}

struct TcpOption {
    kind: u8,
    body: union(kind) {
        0 => Eol { },
        1 => Nop { },
        _ => Generic { len: u8, data: [u8; len - 2] },
    },
}

struct TcpOptionList {
    @greedy(unsafe_eof) @max_iter(40) opts: [TcpOption],
}

struct Tcp {
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack_no: u32,
    @range(5, 15) data_offset: b<4>,
    reserved: b<3>,
    ns: b<1>,
    cwr: b<1>,
    ece: b<1>,
    urg: b<1>,
    ack: b<1>,
    psh: b<1>,
    rst: b<1>,
    syn: b<1>,
    fin: b<1>,
    window: u16,
    checksum: u16,
    urgent_ptr: u16,
    @len((data_offset - 5) * 4) options: TcpOptionList,
    @greedy(unsafe_eof) payload: [u8],
}

struct Icmpv4 {
    icmp_type: u8,
    code: u8,
    checksum: u16,
    body: union(icmp_type, code) {
        (0, 0) | (8, 0) => Echo { id: u16, seq: u16, @greedy(unsafe_eof) data: [u8] },
        (3, _) => DestUnreach { unused: u32, @greedy(unsafe_eof) original: [u8] },
        (11, _) => TimeExceeded { unused: u32 },
        (_, _) => Raw { },
    },
}

struct Dns {
    id: u16,
    qr: b<1>,
    opcode: b<4>,
    aa: b<1>,
    tc: b<1>,
    rd: b<1>,
    ra: b<1>,
    z: b<3>,
    rcode: b<4>,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
    @hook(parse_dns_name, String) qname: [u8],
    qtype: u16,
    qclass: u16,
    @hook(parse_dns_name, String) aname: [u8],
    atype: u16,
    aclass: u16,
    ttl: u32,
    rdlength: u16,
    @len(rdlength) rdata: union(atype) {
        1 => A { addr: [u8; 4] },
        28 => Aaaa { addr: [u8; 16] },
        5 => Cname { @greedy(unsafe_eof) labels: [u8] },
        2 => Ns { @greedy(unsafe_eof) labels: [u8] },
        _ => Raw { @greedy(unsafe_eof) bytes: [u8] },
    },
}

@len(length + 5)
struct TlsRecord {
    @range(20, 23) content_type: u8,
    @check(legacy_major == 3) legacy_major: u8,
    legacy_minor: u8,
    @range(1, 16640) length: u16,
    fragment: [u8],
}

struct TlsStream {
    @greedy(unsafe_eof) @max_iter(8) records: [TlsRecord],
}
"#;

    let code = generated_code(dsl);
    let test_dir = write_runtime_crate(&code);
    let output = Command::new("cargo")
        .arg("test")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(test_dir.join("Cargo.toml"))
        .output()
        .expect("failed to run protocol suite runtime tests");

    assert!(
        output.status.success(),
        "protocol suite runtime tests failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
