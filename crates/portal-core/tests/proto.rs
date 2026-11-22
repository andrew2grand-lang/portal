//! Frames and control messages (brief §5 protocol, §9 "protocol encode/decode").

use portal_core::model::{DeviceId, Incoming, Os, Role};
use portal_core::proto::{
    self, B32, CancelReason, DeclineReason, DirEntry, ErrorCode, Frame, Hello, ItemKind, Msg, Mtime, Offer, OfferItem,
    Share, SkipNote, SkipReason, Summary,
};

fn id() -> DeviceId {
    DeviceId::parse("0123456789abcdef0123456789abcdef").unwrap()
}

fn samples() -> Vec<Msg> {
    let tid = 0x1234_5678_9abc_def0;
    vec![
        Msg::Hello(Hello {
            proto: 1,
            id: id(),
            name: "Desk PC".into(),
            os: Os::Windows,
            role: Role::Both,
            app: "0.1.0".into(),
            port: 47920,
        }),
        Msg::Error { code: ErrorCode::IncompatibleVersion, msg: "update".into() },
        Msg::Ping,
        Msg::Pong,
        Msg::Bye,
        Msg::Superseded,
        Msg::Status { role: Role::Send, name: "Studio".into(), accepts: Some(Incoming::Never) },
        Msg::Welcome { paired: false },
        Msg::PairAsk,
        Msg::PairReady,
        Msg::PairKey { share: Share([4; 65]) },
        Msg::PairCheck { mac: B32([5; 32]) },
        Msg::PairWrong { tries_left: 2 },
        Msg::PairDone,
        Msg::PairStop,
        Msg::Offer(Offer {
            tid,
            items: vec![
                OfferItem { name: "Photos".into(), kind: ItemKind::Folder, bytes: 10, files: 2 },
                OfferItem { name: "a.pdf".into(), kind: ItemKind::File, bytes: 5, files: 1 },
            ],
            total_bytes: 15,
            file_count: 3,
            dir_count: 1,
            skipped: vec![SkipNote { path: vec!["Photos".into(), "link".into()], reason: SkipReason::Symlink }],
            sender_role: Role::Both,
        }),
        Msg::Accept { tid },
        Msg::Decline { tid, reason: DeclineReason::NoSpace { needed: 10, available: 5 } },
        Msg::Decline { tid, reason: DeclineReason::NotAccepting },
        Msg::Dirs { tid, entries: vec![DirEntry { path: vec!["Photos".into()], mtime: Some(Mtime { s: 1, ns: 2 }) }] },
        Msg::File { tid, idx: 0, path: vec!["Photos".into(), "é.jpg".into()], size: 7, mtime: None, from: 0 },
        Msg::FileEnd { tid, idx: 0, blake3: B32([9; 32]) },
        Msg::Done { tid },
        Msg::Complete {
            tid,
            summary: Summary {
                files: 3,
                bytes: 15,
                renamed: 1,
                skipped: 0,
                long_paths: 0,
                saved_in: None,
                renames: vec![],
            },
        },
        Msg::Progress { tid, bytes: 99 },
        Msg::Cancel { tid, reason: CancelReason::User },
        Msg::Cancel { tid, reason: CancelReason::Error { code: "disk_full".into(), msg: "no space".into() } },
    ]
}

#[tokio::test]
async fn every_message_survives_a_round_trip_through_frames() {
    let (mut a, mut b) = tokio::io::duplex(1 << 20);
    for msg in samples() {
        proto::write_frame(&mut a, &Frame::Control(msg.clone())).await.unwrap();
        let got = proto::read_frame(&mut b).await.unwrap();
        assert_eq!(got, Frame::Control(msg));
    }
}

#[tokio::test]
async fn data_frames_carry_raw_bytes() {
    let (mut a, mut b) = tokio::io::duplex(1 << 21);
    let payload: Vec<u8> = (0..proto::MAX_DATA_LEN as usize).map(|i| (i % 251) as u8).collect();
    proto::write_frame(&mut a, &Frame::Data(payload.clone())).await.unwrap();
    assert_eq!(proto::read_frame(&mut b).await.unwrap(), Frame::Data(payload));
}

#[tokio::test]
async fn oversized_frames_are_rejected_before_reading_the_body() {
    let (mut a, mut b) = tokio::io::duplex(64);
    use tokio::io::AsyncWriteExt;
    // Control frame claiming 64 MiB: must fail from the header alone.
    a.write_all(&[1, 0x04, 0x00, 0x00, 0x00]).await.unwrap();
    assert!(proto::read_frame(&mut b).await.is_err());

    let (mut a, mut b) = tokio::io::duplex(64);
    a.write_all(&[2, 0x00, 0x20, 0x00, 0x01]).await.unwrap();
    assert!(proto::read_frame(&mut b).await.is_err());
}

#[tokio::test]
async fn unknown_frame_kinds_are_rejected() {
    let (mut a, mut b) = tokio::io::duplex(64);
    use tokio::io::AsyncWriteExt;
    a.write_all(&[7, 0, 0, 0, 1, 0]).await.unwrap();
    assert!(proto::read_frame(&mut b).await.is_err());
}

#[tokio::test]
async fn a_frame_cut_short_is_an_error() {
    let (mut a, mut b) = tokio::io::duplex(64);
    use tokio::io::AsyncWriteExt;
    a.write_all(&[2, 0, 0, 0, 10, 1, 2, 3]).await.unwrap();
    drop(a);
    assert!(proto::read_frame(&mut b).await.is_err());
}

#[test]
fn unknown_message_types_decode_as_unknown_for_forward_compatibility() {
    let msg: Msg = serde_json::from_str(r#"{"t":"something_new","x":1}"#).unwrap();
    assert_eq!(msg, Msg::Unknown);
}

#[test]
fn malformed_messages_are_errors() {
    assert!(serde_json::from_str::<Msg>(r#"{"t":"accept"}"#).is_err());
    assert!(serde_json::from_str::<Msg>(r#"{"t":"pair_check","mac":"zz"}"#).is_err());
    assert!(serde_json::from_str::<Msg>(r#"{"no_tag":true}"#).is_err());
}

#[test]
fn hello_from_another_protocol_version_is_refused() {
    assert!(proto::check_version(proto::PROTOCOL_VERSION).is_ok());
    assert_eq!(proto::check_version(proto::PROTOCOL_VERSION + 1), Err(ErrorCode::IncompatibleVersion));
    assert_eq!(proto::check_version(0), Err(ErrorCode::IncompatibleVersion));
}

/// Mission 01 builds speak the same protocol version: what they send must
/// still read, and what they expect must still be there.
#[test]
fn files_from_settings_travel_in_fields_mission_01_builds_understand() {
    let old_status = r#"{"t":"status","role":"both","name":"Desk PC"}"#;
    let msg: Msg = serde_json::from_str(old_status).expect("an old Status still reads");
    assert!(matches!(msg, Msg::Status { accepts: None, .. }), "{msg:?}");

    let new =
        serde_json::to_value(Msg::Status { role: Role::Send, name: "Desk PC".into(), accepts: Some(Incoming::Never) })
            .unwrap();
    assert_eq!(new["role"], "send", "old builds read Never as a computer set to Send and block before offering");
    let decline = serde_json::to_value(Msg::Decline { tid: 1, reason: DeclineReason::NotAccepting }).unwrap();
    assert_eq!(decline["reason"]["why"], "role_send", "old builds show their own blocked screen");
}

/// Fields added during Mission 03 are optional both ways: a build without
/// them still reads (and is read by) one with them.
#[test]
fn resume_prefix_and_saved_folder_are_optional_on_the_wire() {
    let old = r#"{"t":"resume_from","tid":7,"dirs":0,"file":1,"offset":10,"bytes":20}"#;
    match serde_json::from_str::<Msg>(old).unwrap() {
        Msg::ResumeFrom { prefix, offset, .. } => assert_eq!((prefix, offset), (None, 10)),
        other => panic!("{other:?}"),
    }
    let old = r#"{"t":"complete","tid":7,"summary":{"files":1,"bytes":2,"renamed":0,"skipped":0,"long_paths":0}}"#;
    match serde_json::from_str::<Msg>(old).unwrap() {
        Msg::Complete { summary, .. } => assert_eq!(summary.saved_in, None),
        other => panic!("{other:?}"),
    }
    let none = Msg::ResumeFrom { tid: 7, dirs: 0, file: 1, offset: 0, bytes: 0, prefix: None };
    assert!(!serde_json::to_string(&none).unwrap().contains("prefix"), "left out when there's none");
}

/// Mission 04 §2, no downgrade: the numeric-comparison messages of protocol
/// 2 mean nothing to protocol 3 (they decode as unknown and are dropped).
#[test]
fn the_old_pairing_messages_are_not_understood_any_more() {
    for old in [
        r#"{"t":"pair_request"}"#,
        r#"{"t":"pair_commit","c":"0101010101010101010101010101010101010101010101010101010101010101"}"#,
        r#"{"t":"pair_nonce","n":"0202020202020202020202020202020202020202020202020202020202020202"}"#,
        r#"{"t":"pair_reveal","n":"0303030303030303030303030303030303030303030303030303030303030303"}"#,
        r#"{"t":"pair_confirm","ok":true}"#,
    ] {
        assert_eq!(serde_json::from_str::<Msg>(old).unwrap(), Msg::Unknown, "{old}");
    }
    assert_eq!(proto::PROTOCOL_VERSION, 4, "Mission 05 speaks protocol 4");
}

#[test]
fn a_share_is_exactly_65_bytes() {
    assert!(serde_json::from_str::<Msg>(&format!(r#"{{"t":"pair_key","share":"{}"}}"#, "04".repeat(64))).is_err());
    assert!(serde_json::from_str::<Msg>(&format!(r#"{{"t":"pair_key","share":"{}"}}"#, "04".repeat(66))).is_err());
    assert!(serde_json::from_str::<Msg>(&format!(r#"{{"t":"pair_key","share":"{}"}}"#, "04".repeat(65))).is_ok());
}
