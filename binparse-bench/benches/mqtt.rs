//! MQTT 3.1.1 + v5: binparse vs the rumqtt codecs.
//!
//! Three rumqtt codecs are compared: the shipping standalone `mqttbytes 0.6`
//! (what rumqttc depends on today), the in-progress next-line client codec
//! bundled in `rumqttc-v4-next`/`rumqttc-v5-next`, and the broker-side codec in
//! `rumqttd 0.20` (`protocol::v4::V4`/`v5::V5` via the `Protocol::read_mut`
//! trait method). All three decode a whole packet out of a `BytesMut` into owned
//! structs (allocating topic/payload), whereas binparse borrows the input and
//! reads fields lazily — so this is decode-into-owned vs zero-copy, not a
//! like-for-like memory model. The rumqtt buffers are rebuilt per iteration via
//! `iter_batched` setup (untimed) because `read()` drains them; binparse borrows
//! a `'static` slice and needs no setup.

use std::hint::black_box;

use bytes::BytesMut;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

use binparse_bench::*;

const MAX: usize = 256 * 1024;

fn sum_bp(it: impl Iterator<Item = binparse::ParseResult<u8>>) -> u64 {
    it.map(|b| b.unwrap() as u64).fold(0, u64::wrapping_add)
}

fn buf(bytes: &[u8]) -> BytesMut {
    let mut b = BytesMut::with_capacity(bytes.len());
    b.extend_from_slice(bytes);
    b
}

fn v3_connect(c: &mut Criterion) {
    use binparse_protocols::mqtt_v3::{MqttPacket, MqttPacket_body};
    let mut g = c.benchmark_group("mqtt_v3_connect");
    g.bench_function("binparse", |b| {
        b.iter(|| {
            let (mut pkt, _) = MqttPacket::parse(black_box(MQTT_V3_CONNECT)).unwrap();
            let pt = pkt.packet_type() as u64;
            match pkt.body().unwrap() {
                MqttPacket_body::Connect(c) => {
                    black_box(pt ^ c.keep_alive() as u64 ^ sum_bp(c.proto_name().unwrap()))
                }
                _ => unreachable!(),
            }
        })
    });
    g.bench_function("mqttbytes_0.6", |b| {
        b.iter_batched(
            || buf(MQTT_V3_CONNECT),
            |mut buf| match mqttbytes::v4::read(&mut buf, MAX).unwrap() {
                mqttbytes::v4::Packet::Connect(c) => {
                    black_box(c.keep_alive as u64 ^ c.client_id.len() as u64)
                }
                _ => unreachable!(),
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("rumqttc_v4_next", |b| {
        use rumqttc_v4_next::mqttbytes::v4;
        b.iter_batched(
            || buf(MQTT_V3_CONNECT),
            |mut buf| match v4::Packet::read(&mut buf, MAX).unwrap() {
                v4::Packet::Connect(c) => black_box(c.keep_alive as u64 ^ c.client_id.len() as u64),
                _ => unreachable!(),
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("rumqttd_0.20", |b| {
        use rumqttd::protocol::{Packet, Protocol, v4::V4};
        b.iter_batched(
            || buf(MQTT_V3_CONNECT),
            |mut buf| match V4.read_mut(&mut buf, MAX).unwrap() {
                Packet::Connect(c, ..) => black_box(c.keep_alive as u64 ^ c.client_id.len() as u64),
                _ => unreachable!(),
            },
            BatchSize::SmallInput,
        )
    });
    g.finish();
}

fn v3_publish(c: &mut Criterion) {
    use binparse_protocols::mqtt_v3::{MqttPacket, MqttPacket_body};
    let mut g = c.benchmark_group("mqtt_v3_publish");
    g.bench_function("binparse", |b| {
        b.iter(|| {
            let (mut pkt, _) = MqttPacket::parse(black_box(MQTT_V3_PUBLISH)).unwrap();
            match pkt.body().unwrap() {
                MqttPacket_body::Publish(p) => black_box(
                    p.topic_len() as u64
                        ^ sum_bp(p.topic().unwrap())
                        ^ sum_bp(p.payload().unwrap()),
                ),
                _ => unreachable!(),
            }
        })
    });
    g.bench_function("mqttbytes_0.6", |b| {
        b.iter_batched(
            || buf(MQTT_V3_PUBLISH),
            |mut buf| match mqttbytes::v4::read(&mut buf, MAX).unwrap() {
                mqttbytes::v4::Packet::Publish(p) => {
                    black_box(p.topic.len() as u64 ^ p.payload.len() as u64 ^ p.qos as u64)
                }
                _ => unreachable!(),
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("rumqttc_v4_next", |b| {
        use rumqttc_v4_next::mqttbytes::v4;
        b.iter_batched(
            || buf(MQTT_V3_PUBLISH),
            |mut buf| match v4::Packet::read(&mut buf, MAX).unwrap() {
                v4::Packet::Publish(p) => {
                    black_box(p.topic.len() as u64 ^ p.payload.len() as u64 ^ p.qos as u64)
                }
                _ => unreachable!(),
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("rumqttd_0.20", |b| {
        use rumqttd::protocol::{Packet, Protocol, v4::V4};
        b.iter_batched(
            || buf(MQTT_V3_PUBLISH),
            |mut buf| match V4.read_mut(&mut buf, MAX).unwrap() {
                Packet::Publish(p, _) => black_box(p.topic.len() as u64 ^ p.payload.len() as u64),
                _ => unreachable!(),
            },
            BatchSize::SmallInput,
        )
    });
    g.finish();
}

// Write-path groups: serialize a structured value to fresh owned bytes (alloc +
// encode), mirroring binparse's `to_vec`. The model is the mirror image of the
// read benches: binparse's `*Content` borrows the field slices (`&'static`
// fixtures, built untimed), so its timed work is purely encode-into-a-fresh-Vec;
// the rumqtt arms hold owned packet structs (allocating String/Bytes), built in
// `iter_batched` setup (untimed), and the timed routine serializes into a fresh
// `BytesMut`. Both sides pay exactly one output allocation + one encode pass.
fn v3_connect_write(c: &mut Criterion) {
    use binparse_protocols::mqtt_v3::{
        ConnectContent, MqttPacketBodyContent, MqttPacketContent, MqttPacketWriter,
    };

    let content = MqttPacketContent {
        flags: 0,
        body: MqttPacketBodyContent::Connect(ConnectContent {
            proto_name: b"MQTT",
            proto_level: 4,
            connect_flags: 2,
            keep_alive: 60,
            payload: &[0x00, 0x02, b'i', b'd'],
        }),
    };
    assert_eq!(MqttPacketWriter::to_vec(&content), MQTT_V3_CONNECT);

    {
        let mut c0 = mqttbytes::v4::Connect::new("id");
        c0.keep_alive = 60;
        let mut buf = BytesMut::new();
        c0.write(&mut buf).unwrap();
        if buf.as_ref() != MQTT_V3_CONNECT {
            eprintln!(
                "mqtt_v3_connect_write: mqttbytes_0.6 output differs from fixture: {:02x?} vs {:02x?}",
                buf.as_ref(),
                MQTT_V3_CONNECT
            );
        }
    }
    {
        use rumqttc_v4_next::mqttbytes::v4;
        let mut c0 = v4::Connect::new("id");
        c0.keep_alive = 60;
        let mut buf = BytesMut::new();
        c0.write(&mut buf).unwrap();
        if buf.as_ref() != MQTT_V3_CONNECT {
            eprintln!(
                "mqtt_v3_connect_write: rumqttc_v4_next output differs from fixture: {:02x?} vs {:02x?}",
                buf.as_ref(),
                MQTT_V3_CONNECT
            );
        }
    }
    {
        use rumqttd::protocol::{Connect, Packet, Protocol, v4::V4};
        let connect = Connect {
            keep_alive: 60,
            client_id: "id".into(),
            clean_session: true,
        };
        let pkt = Packet::Connect(connect, None, None, None, None);
        let mut buf = BytesMut::new();
        V4.write(pkt, &mut buf).unwrap();
        if buf.as_ref() != MQTT_V3_CONNECT {
            eprintln!(
                "mqtt_v3_connect_write: rumqttd_0.20 output differs from fixture: {:02x?} vs {:02x?}",
                buf.as_ref(),
                MQTT_V3_CONNECT
            );
        }
    }

    let mut g = c.benchmark_group("mqtt_v3_connect_write");
    g.bench_function("binparse", |b| {
        b.iter(|| black_box(MqttPacketWriter::to_vec(black_box(&content))))
    });
    g.bench_function("mqttbytes_0.6", |b| {
        b.iter_batched(
            || {
                let mut c0 = mqttbytes::v4::Connect::new("id");
                c0.keep_alive = 60;
                c0
            },
            |pkt| {
                let mut buf = BytesMut::new();
                pkt.write(&mut buf).unwrap();
                black_box(buf)
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("rumqttc_v4_next", |b| {
        use rumqttc_v4_next::mqttbytes::v4;
        b.iter_batched(
            || {
                let mut c0 = v4::Connect::new("id");
                c0.keep_alive = 60;
                c0
            },
            |pkt| {
                let mut buf = BytesMut::new();
                pkt.write(&mut buf).unwrap();
                black_box(buf)
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("rumqttd_0.20", |b| {
        use rumqttd::protocol::{Connect, Packet, Protocol, v4::V4};
        b.iter_batched(
            || {
                let connect = Connect {
                    keep_alive: 60,
                    client_id: "id".into(),
                    clean_session: true,
                };
                Packet::Connect(connect, None, None, None, None)
            },
            |pkt| {
                let mut buf = BytesMut::new();
                V4.write(pkt, &mut buf).unwrap();
                black_box(buf)
            },
            BatchSize::SmallInput,
        )
    });
    g.finish();
}

fn v3_publish_write(c: &mut Criterion) {
    use binparse_protocols::mqtt_v3::{
        MqttPacketBodyContent, MqttPacketContent, MqttPacketWriter, PublishContent,
    };
    use mqttbytes::QoS;

    let content = MqttPacketContent {
        flags: 0,
        body: MqttPacketBodyContent::Publish(PublishContent {
            topic: b"a/b",
            payload: b"hello",
        }),
    };
    assert_eq!(MqttPacketWriter::to_vec(&content), MQTT_V3_PUBLISH);

    {
        let p = mqttbytes::v4::Publish::new("a/b", QoS::AtMostOnce, "hello");
        let mut buf = BytesMut::new();
        p.write(&mut buf).unwrap();
        if buf.as_ref() != MQTT_V3_PUBLISH {
            eprintln!(
                "mqtt_v3_publish_write: mqttbytes_0.6 output differs from fixture: {:02x?} vs {:02x?}",
                buf.as_ref(),
                MQTT_V3_PUBLISH
            );
        }
    }
    {
        use rumqttc_v4_next::mqttbytes::{QoS, v4};
        let p = v4::Publish::new("a/b", QoS::AtMostOnce, "hello");
        let mut buf = BytesMut::new();
        p.write(&mut buf).unwrap();
        if buf.as_ref() != MQTT_V3_PUBLISH {
            eprintln!(
                "mqtt_v3_publish_write: rumqttc_v4_next output differs from fixture: {:02x?} vs {:02x?}",
                buf.as_ref(),
                MQTT_V3_PUBLISH
            );
        }
    }
    {
        use rumqttd::protocol::{Packet, Protocol, Publish, v4::V4};
        let publish = Publish::new("a/b", "hello", false);
        let pkt = Packet::Publish(publish, None);
        let mut buf = BytesMut::new();
        V4.write(pkt, &mut buf).unwrap();
        if buf.as_ref() != MQTT_V3_PUBLISH {
            eprintln!(
                "mqtt_v3_publish_write: rumqttd_0.20 output differs from fixture: {:02x?} vs {:02x?}",
                buf.as_ref(),
                MQTT_V3_PUBLISH
            );
        }
    }

    let mut g = c.benchmark_group("mqtt_v3_publish_write");
    g.bench_function("binparse", |b| {
        b.iter(|| black_box(MqttPacketWriter::to_vec(black_box(&content))))
    });
    g.bench_function("mqttbytes_0.6", |b| {
        b.iter_batched(
            || mqttbytes::v4::Publish::new("a/b", QoS::AtMostOnce, "hello"),
            |pkt| {
                let mut buf = BytesMut::new();
                pkt.write(&mut buf).unwrap();
                black_box(buf)
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("rumqttc_v4_next", |b| {
        use rumqttc_v4_next::mqttbytes::{QoS, v4};
        b.iter_batched(
            || v4::Publish::new("a/b", QoS::AtMostOnce, "hello"),
            |pkt| {
                let mut buf = BytesMut::new();
                pkt.write(&mut buf).unwrap();
                black_box(buf)
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("rumqttd_0.20", |b| {
        use rumqttd::protocol::{Packet, Protocol, Publish, v4::V4};
        b.iter_batched(
            || Packet::Publish(Publish::new("a/b", "hello", false), None),
            |pkt| {
                let mut buf = BytesMut::new();
                V4.write(pkt, &mut buf).unwrap();
                black_box(buf)
            },
            BatchSize::SmallInput,
        )
    });
    g.finish();
}

fn v5_connack_write(c: &mut Criterion) {
    use binparse_protocols::mqtt_v5::{
        ConnackContent, MqttPacketBodyContent, MqttPacketContent, MqttPacketWriter,
    };

    let content = MqttPacketContent {
        flags: 0,
        body: MqttPacketBodyContent::Connack(ConnackContent {
            ack_flags: 0,
            reason_code: 0,
            properties: &[0x21, 0x00, 0x14],
        }),
    };
    assert_eq!(MqttPacketWriter::to_vec(&content), MQTT_V5_CONNACK);

    {
        use mqttbytes::v5::{ConnAck, ConnAckProperties, ConnectReturnCode};
        let pkt = ConnAck {
            session_present: false,
            code: ConnectReturnCode::Success,
            properties: Some(ConnAckProperties {
                receive_max: Some(20),
                ..ConnAckProperties::new()
            }),
        };
        let mut buf = BytesMut::new();
        pkt.write(&mut buf).unwrap();
        if buf.as_ref() != MQTT_V5_CONNACK {
            eprintln!(
                "mqtt_v5_connack_write: mqttbytes_0.6 output differs from fixture: {:02x?} vs {:02x?}",
                buf.as_ref(),
                MQTT_V5_CONNACK
            );
        }
    }
    {
        use rumqttc_v5_next::mqttbytes::v5;
        let pkt = v5::ConnAck {
            session_present: false,
            code: v5::ConnectReturnCode::Success,
            properties: Some(v5::ConnAckProperties {
                session_expiry_interval: None,
                receive_max: Some(20),
                max_qos: None,
                retain_available: None,
                max_packet_size: None,
                assigned_client_identifier: None,
                topic_alias_max: None,
                reason_string: None,
                user_properties: Vec::new(),
                wildcard_subscription_available: None,
                subscription_identifiers_available: None,
                shared_subscription_available: None,
                server_keep_alive: None,
                response_information: None,
                server_reference: None,
                authentication_method: None,
                authentication_data: None,
            }),
        };
        let mut buf = BytesMut::new();
        pkt.write(&mut buf).unwrap();
        if buf.as_ref() != MQTT_V5_CONNACK {
            eprintln!(
                "mqtt_v5_connack_write: rumqttc_v5_next output differs from fixture: {:02x?} vs {:02x?}",
                buf.as_ref(),
                MQTT_V5_CONNACK
            );
        }
    }
    {
        use rumqttd::protocol::{ConnAck, ConnAckProperties, ConnectReturnCode, Packet, Protocol, v5::V5};
        let connack = ConnAck {
            session_present: false,
            code: ConnectReturnCode::Success,
        };
        let props = ConnAckProperties {
            receive_max: Some(20),
            ..Default::default()
        };
        let pkt = Packet::ConnAck(connack, Some(props));
        let mut buf = BytesMut::new();
        V5.write(pkt, &mut buf).unwrap();
        if buf.as_ref() != MQTT_V5_CONNACK {
            eprintln!(
                "mqtt_v5_connack_write: rumqttd_0.20 output differs from fixture: {:02x?} vs {:02x?}",
                buf.as_ref(),
                MQTT_V5_CONNACK
            );
        }
    }

    let mut g = c.benchmark_group("mqtt_v5_connack_write");
    g.bench_function("binparse", |b| {
        b.iter(|| black_box(MqttPacketWriter::to_vec(black_box(&content))))
    });
    g.bench_function("mqttbytes_0.6", |b| {
        use mqttbytes::v5::{ConnAck, ConnAckProperties, ConnectReturnCode};
        b.iter_batched(
            || ConnAck {
                session_present: false,
                code: ConnectReturnCode::Success,
                properties: Some(ConnAckProperties {
                    receive_max: Some(20),
                    ..ConnAckProperties::new()
                }),
            },
            |pkt| {
                let mut buf = BytesMut::new();
                pkt.write(&mut buf).unwrap();
                black_box(buf)
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("rumqttc_v5_next", |b| {
        use rumqttc_v5_next::mqttbytes::v5;
        b.iter_batched(
            || v5::ConnAck {
                session_present: false,
                code: v5::ConnectReturnCode::Success,
                properties: Some(v5::ConnAckProperties {
                    session_expiry_interval: None,
                    receive_max: Some(20),
                    max_qos: None,
                    retain_available: None,
                    max_packet_size: None,
                    assigned_client_identifier: None,
                    topic_alias_max: None,
                    reason_string: None,
                    user_properties: Vec::new(),
                    wildcard_subscription_available: None,
                    subscription_identifiers_available: None,
                    shared_subscription_available: None,
                    server_keep_alive: None,
                    response_information: None,
                    server_reference: None,
                    authentication_method: None,
                    authentication_data: None,
                }),
            },
            |pkt| {
                let mut buf = BytesMut::new();
                pkt.write(&mut buf).unwrap();
                black_box(buf)
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("rumqttd_0.20", |b| {
        use rumqttd::protocol::{ConnAck, ConnAckProperties, ConnectReturnCode, Packet, Protocol, v5::V5};
        b.iter_batched(
            || {
                let connack = ConnAck {
                    session_present: false,
                    code: ConnectReturnCode::Success,
                };
                let props = ConnAckProperties {
                    receive_max: Some(20),
                    ..Default::default()
                };
                Packet::ConnAck(connack, Some(props))
            },
            |pkt| {
                let mut buf = BytesMut::new();
                V5.write(pkt, &mut buf).unwrap();
                black_box(buf)
            },
            BatchSize::SmallInput,
        )
    });
    g.finish();
}

fn v5_connack(c: &mut Criterion) {
    use binparse_protocols::mqtt_v5::{MqttPacket, MqttPacket_body};
    let mut g = c.benchmark_group("mqtt_v5_connack");
    g.bench_function("binparse", |b| {
        b.iter(|| {
            let (mut pkt, _) = MqttPacket::parse(black_box(MQTT_V5_CONNACK)).unwrap();
            match pkt.body().unwrap() {
                MqttPacket_body::Connack(c) => black_box(
                    c.reason_code() as u64
                        ^ *c.prop_len().unwrap()
                        ^ sum_bp(c.properties().unwrap()),
                ),
                _ => unreachable!(),
            }
        })
    });
    g.bench_function("mqttbytes_0.6", |b| {
        b.iter_batched(
            || buf(MQTT_V5_CONNACK),
            |mut buf| match mqttbytes::v5::read(&mut buf, MAX).unwrap() {
                mqttbytes::v5::Packet::ConnAck(c) => black_box(
                    c.session_present as u64 ^ c.code as u64 ^ c.properties.is_some() as u64,
                ),
                _ => unreachable!(),
            },
            BatchSize::SmallInput,
        )
    });
    g.bench_function("rumqttc_v5_next", |b| {
        use rumqttc_v5_next::mqttbytes::v5;
        b.iter_batched(
            || buf(MQTT_V5_CONNACK),
            |mut buf| match v5::Packet::read(&mut buf, None).unwrap() {
                v5::Packet::ConnAck(c) => black_box(
                    c.session_present as u64 ^ c.code as u64 ^ c.properties.is_some() as u64,
                ),
                _ => unreachable!(),
            },
            BatchSize::SmallInput,
        )
    });
    // No rumqttd arm here: its `protocol::v5` codec is the broker's read path,
    // which only decodes client->server packets. CONNACK is server->client, so
    // `V5::read_mut` reaches `unreachable!()` on it. rumqttd participates in the
    // v3 CONNECT/PUBLISH groups (both client->server) instead.
    g.finish();
}

criterion_group!(
    benches,
    v3_connect,
    v3_publish,
    v5_connack,
    v3_connect_write,
    v3_publish_write,
    v5_connack_write
);
criterion_main!(benches);
