use super::*;

use crate::ocr::testing;

fn frame(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut out = ((body.len() + 1) as u32).to_le_bytes().to_vec();
    out.push(tag);
    out.extend_from_slice(body);
    out
}

fn encode_request(request: Outgoing) -> Vec<u8> {
    let mut out = Vec::new();
    write_request(&mut out, request).unwrap();
    out
}

fn encode_reply(reply: &Reply) -> Vec<u8> {
    let mut out = Vec::new();
    write_reply(&mut out, reply).unwrap();
    out
}

fn roundtrip(reply: Reply) {
    let bytes = encode_reply(&reply);
    assert_eq!(read_reply(&mut bytes.as_slice()).unwrap(), reply);
}

fn malformed(result: Result<Reply, ProtoError>) -> bool {
    matches!(result, Err(ProtoError::Malformed(_)))
}

#[test]
fn requests_roundtrip() {
    let bytes = encode_request(Outgoing::Hello("1.0-abc"));
    match read_request(&mut bytes.as_slice()).unwrap() {
        Request::Hello { proto, build } => {
            assert_eq!(proto, PROTO);
            assert_eq!(build, "1.0-abc");
        }
        _ => panic!("expected hello"),
    }

    let files = testing::files("кириллица");
    let bytes = encode_request(Outgoing::Load(&files));
    match read_request(&mut bytes.as_slice()).unwrap() {
        Request::Load(read) => assert_eq!(read, files),
        _ => panic!("expected load"),
    }

    let image = testing::image(7, 5);
    let bytes = encode_request(Outgoing::Recognize(&image));
    match read_request(&mut bytes.as_slice()).unwrap() {
        Request::Recognize(read) => {
            assert_eq!((read.width, read.height, read.origin), (7, 5, image.origin));
            assert_eq!(read.rgb, image.rgb);
        }
        _ => panic!("expected recognize"),
    }

    let bytes = encode_request(Outgoing::Status);
    assert!(matches!(
        read_request(&mut bytes.as_slice()),
        Ok(Request::Status)
    ));
    let bytes = encode_request(Outgoing::Shutdown);
    assert!(matches!(
        read_request(&mut bytes.as_slice()),
        Ok(Request::Shutdown)
    ));
}

#[test]
fn non_utf8_paths_survive() {
    let files = ModelFiles {
        detector: PathBuf::from(OsStr::from_bytes(b"/m/\xff\xfe.onnx")),
        recognizer: PathBuf::from("/m/r.onnx"),
        dict: PathBuf::from("/m/d.txt"),
    };
    let bytes = encode_request(Outgoing::Load(&files));
    match read_request(&mut bytes.as_slice()).unwrap() {
        Request::Load(read) => assert_eq!(read, files),
        _ => panic!("expected load"),
    }
}

#[test]
fn replies_roundtrip() {
    for state in [
        EngineState::Idle,
        EngineState::Loading { model: "m".into() },
        EngineState::Ready {
            device: "cpu".into(),
            model: "/models/cyrillic.onnx".into(),
        },
        EngineState::Failed("no such file".into()),
        EngineState::NoGpu,
    ] {
        roundtrip(Reply::Welcome(Welcome {
            proto: PROTO,
            build: "b".into(),
            state: state.clone(),
        }));
        for idle_left_secs in [None, Some(42)] {
            roundtrip(Reply::Status(Status {
                pid: 123,
                uptime_secs: 9,
                state: state.clone(),
                served: 7,
                last_ms: 314,
                rss_kb: 64_000,
                idle_left_secs,
            }));
        }
    }
    roundtrip(Reply::Ok);
    roundtrip(Reply::Error("ошибка".into()));
    roundtrip(Reply::Lines(OcrText::default()));
    roundtrip(Reply::Lines(testing::labelled(
        "привет",
        &testing::image(30, 10),
    )));
}

#[test]
fn several_frames_read_back_in_order() {
    let mut bytes = encode_reply(&Reply::Ok);
    bytes.extend(encode_reply(&Reply::Error("x".into())));
    let mut r = bytes.as_slice();
    assert_eq!(read_reply(&mut r).unwrap(), Reply::Ok);
    assert_eq!(read_reply(&mut r).unwrap(), Reply::Error("x".into()));
    assert!(
        matches!(read_reply(&mut r), Err(ProtoError::Io(e)) if e.kind() == io::ErrorKind::UnexpectedEof)
    );
}

#[test]
fn truncated_frames_are_errors_not_panics() {
    let full = encode_reply(&Reply::Lines(testing::labelled(
        "abc",
        &testing::image(4, 4),
    )));
    for cut in 0..full.len() {
        assert!(read_reply(&mut &full[..cut]).is_err(), "cut at {cut}");
    }
    let full = encode_request(Outgoing::Recognize(&testing::image(3, 3)));
    for cut in 0..full.len() {
        assert!(read_request(&mut &full[..cut]).is_err(), "cut at {cut}");
    }
}

#[test]
fn oversized_frames_are_refused_before_allocating() {
    let mut huge = ((MAX_FRAME + 1) as u32).to_le_bytes().to_vec();
    huge.push(tag::OK);
    assert!(matches!(
        read_reply(&mut huge.as_slice()),
        Err(ProtoError::TooLarge(_))
    ));

    let mut big_small = ((MAX_SMALL + 1) as u32).to_le_bytes().to_vec();
    big_small.push(tag::STATUS);
    assert!(matches!(
        read_request(&mut big_small.as_slice()),
        Err(ProtoError::TooLarge(_))
    ));

    assert!(read_request(&mut b"GET / HTTP/1.1\r\n\r\n".as_slice()).is_err());
}

#[test]
fn empty_frame_and_unknown_tags() {
    let zero = 0u32.to_le_bytes();
    assert!(matches!(
        read_reply(&mut zero.as_slice()),
        Err(ProtoError::Malformed(_)) | Err(ProtoError::Io(_))
    ));
    let mut zero = zero.to_vec();
    zero.push(0);
    assert!(matches!(
        read_reply(&mut zero.as_slice()),
        Err(ProtoError::Malformed(_))
    ));

    assert!(matches!(
        read_reply(&mut frame(200, &[]).as_slice()),
        Err(ProtoError::UnknownTag(200))
    ));
    assert!(matches!(
        read_request(&mut frame(200, &[]).as_slice()),
        Err(ProtoError::UnknownTag(200))
    ));
    // A reply is not mistaken for a request and vice versa
    assert!(matches!(
        read_request(&mut frame(tag::OK, &[]).as_slice()),
        Err(ProtoError::UnknownTag(_))
    ));
    assert!(matches!(
        read_reply(&mut frame(tag::STATUS, &[]).as_slice()),
        Err(ProtoError::UnknownTag(_))
    ));
}

#[test]
fn trailing_bytes_are_rejected() {
    assert!(malformed(read_reply(&mut frame(tag::OK, &[1]).as_slice())));
    assert!(matches!(
        read_request(&mut frame(tag::STATUS, &[1]).as_slice()),
        Err(ProtoError::Malformed(_))
    ));
}

#[test]
fn hello_from_a_newer_client_may_carry_more() {
    let mut body = Body::default();
    body.u16(PROTO + 1);
    body.str("future");
    body.u64(99);
    let mut bytes = Vec::new();
    body.send(&mut bytes, tag::HELLO).unwrap();
    match read_request(&mut bytes.as_slice()).unwrap() {
        Request::Hello { proto, build } => {
            assert_eq!((proto, build.as_str()), (PROTO + 1, "future"))
        }
        _ => panic!("expected hello"),
    }
}

#[test]
fn welcome_from_another_version_is_still_readable() {
    let mut body = Body::default();
    body.u16(PROTO + 7);
    body.str("old");
    body.u8(250); // state unknown to this version
    body.u64(1);
    let mut bytes = Vec::new();
    body.send(&mut bytes, tag::WELCOME).unwrap();
    match read_reply(&mut bytes.as_slice()).unwrap() {
        Reply::Welcome(w) => assert_eq!((w.proto, w.build.as_str()), (PROTO + 7, "old")),
        other => panic!("expected welcome, got {other:?}"),
    }
}

#[test]
fn bad_image_sizes() {
    let image_frame = |w: u32, h: u32, pixels: usize| {
        let mut body = Vec::new();
        body.extend_from_slice(&w.to_le_bytes());
        body.extend_from_slice(&h.to_le_bytes());
        body.extend_from_slice(&0f32.to_le_bytes());
        body.extend_from_slice(&0f32.to_le_bytes());
        body.extend(std::iter::repeat_n(0u8, pixels));
        frame(tag::RECOGNIZE, &body)
    };
    let is_malformed = |bytes: Vec<u8>| {
        matches!(
            read_request(&mut bytes.as_slice()),
            Err(ProtoError::Malformed(_))
        )
    };

    assert!(is_malformed(image_frame(2, 2, 11)));
    assert!(is_malformed(image_frame(2, 2, 13)));
    assert!(is_malformed(image_frame(0, 5, 0)));
    // `w * h * 3` can overflow and must not wrap around into an unexpectedly small buffer size.
    assert!(is_malformed(image_frame(u32::MAX, u32::MAX, 12)));
    assert!(matches!(
        read_request(&mut frame(tag::RECOGNIZE, &[0; 3]).as_slice()),
        Err(ProtoError::Malformed(_))
    ));

    let mut nan_origin = image_frame(1, 1, 3);
    nan_origin[13..17].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(is_malformed(nan_origin));
}

#[test]
fn mismatched_image_is_not_sent() {
    let mut image = testing::image(4, 4);
    image.rgb.pop();
    let mut out = Vec::new();
    assert!(write_request(&mut out, Outgoing::Recognize(&image)).is_err());
    assert!(out.is_empty());
}

#[test]
fn lines_are_validated() {
    let line = |text: &[u8], bounds: [f32; 4], char_x: &[f32], declared: Option<u32>| {
        let mut body = Body::default();
        body.u32(1);
        body.bytes(text);
        for v in bounds {
            body.f32(v);
        }
        body.u32(declared.unwrap_or(char_x.len() as u32));
        for &x in char_x {
            body.f32(x);
        }
        let mut bytes = Vec::new();
        body.send(&mut bytes, tag::LINES).unwrap();
        read_reply(&mut bytes.as_slice())
    };
    let ok = [0.0, 0.0, 10.0, 5.0];
    assert!(line(b"ab", ok, &[0.0, 5.0, 10.0], None).is_ok());
    assert!(malformed(line(b"\xc3\x28", ok, &[0.0, 5.0, 10.0], None)));
    assert!(malformed(line(
        b"ab",
        [0.0, 0.0, f32::INFINITY, 5.0],
        &[0.0],
        None
    )));
    assert!(malformed(line(b"ab", [10.0, 0.0, 0.0, 5.0], &[0.0], None)));
    assert!(malformed(line(b"ab", ok, &[0.0, f32::NAN, 10.0], None)));
    assert!(malformed(line(b"ab", ok, &[0.0], Some(u32::MAX))));

    let mut body = Body::default();
    body.u32(u32::MAX);
    let mut bytes = Vec::new();
    body.send(&mut bytes, tag::LINES).unwrap();
    assert!(malformed(read_reply(&mut bytes.as_slice())));
}

#[test]
fn bad_state_and_status_flags() {
    let mut body = Body::default();
    body.u16(PROTO);
    body.str("b");
    body.u8(9);
    let mut bytes = Vec::new();
    body.send(&mut bytes, tag::WELCOME).unwrap();
    assert!(malformed(read_reply(&mut bytes.as_slice())));

    let status = Reply::Status(Status {
        pid: 1,
        uptime_secs: 1,
        state: EngineState::NoGpu,
        served: 0,
        last_ms: 0,
        rss_kb: 0,
        idle_left_secs: None,
    });
    let mut bytes = encode_reply(&status);
    *bytes.last_mut().unwrap() = 2;
    assert!(malformed(read_reply(&mut bytes.as_slice())));
}

#[test]
fn too_many_lines_are_not_sent() {
    let one = testing::labelled("a", &testing::image(2, 2))
        .lines
        .remove(0);
    let text = OcrText {
        lines: vec![one; MAX_LINES + 1],
    };
    assert!(write_reply(&mut Vec::new(), &Reply::Lines(text)).is_err());
}
