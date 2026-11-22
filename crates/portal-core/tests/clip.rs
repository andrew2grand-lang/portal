//! The clipboard's pure parts: limits, how it travels, and what's said about
//! it (never the contents in the log).

use portal_core::clip::{self, Assembly, Contents, TooBig};

fn text(s: &str) -> Contents {
    Contents { text: Some(s.into()), ..Default::default() }
}

#[test]
fn every_representation_survives_the_wire() {
    let c = Contents {
        text: Some("meeting notes — café".into()),
        rtf: Some(b"{\\rtf1 meeting}".to_vec()),
        html: Some("<b>meeting</b>".into()),
        png: Some(vec![0x89, b'P', b'N', b'G', 1, 2, 3]),
    };
    assert_eq!(clip::decode(&clip::encode(&c)).unwrap(), c);
    assert_eq!(clip::decode(&clip::encode(&text("x"))).unwrap(), text("x"));
    assert!(clip::decode(b"\x07garbage").is_err());
}

#[test]
fn big_contents_travel_in_parts_and_come_back_whole() {
    let big = text(&"lorem ipsum ".repeat(300_000)); // 3.6 MB
    let msgs = clip::parts(7, &big, false);
    assert!(msgs.len() > 1);
    let mut asm: Option<Assembly> = None;
    let mut done = None;
    for m in msgs {
        let portal_core::proto::Msg::Clip { id, n, of, auto, data, .. } = m else { panic!() };
        assert!(data.len() <= 1024 * 1024, "each part fits a control frame comfortably");
        if n == 0 {
            asm = Some(Assembly::new(id, of, auto).unwrap());
        }
        done = asm.as_mut().unwrap().push(id, n, &data).unwrap();
    }
    assert_eq!(done.unwrap(), big);
}

#[test]
fn parts_out_of_order_or_too_many_are_refused() {
    assert!(Assembly::new(1, 10_000, false).is_err(), "more parts than any allowed clipboard needs");
    let mut a = Assembly::new(1, 3, false).unwrap();
    assert!(a.push(1, 1, "AAAA").is_err(), "skipped part 0");
    let mut a = Assembly::new(1, 2, false).unwrap();
    assert!(a.push(2, 0, "AAAA").is_err(), "another clipboard's part");
}

#[test]
fn limits_send_big_images_and_huge_text_as_files() {
    assert_eq!(clip::check(&text("small")), Ok(()));
    let huge = text(&"x".repeat(clip::TEXT_LIMIT + 1));
    assert_eq!(clip::check(&huge), Err(TooBig::Text(clip::TEXT_LIMIT as u64 + 1)));
    let image = Contents { png: Some(vec![0; clip::IMAGE_LIMIT + 1]), ..Default::default() };
    assert_eq!(clip::check(&image), Err(TooBig::Image(clip::IMAGE_LIMIT as u64 + 1)));
}

#[test]
fn the_log_says_what_kind_and_how_big_never_what() {
    assert_eq!(clip::summary(&text("meeting notes")), "text, 13 bytes");
    let rich = Contents { text: Some("a".repeat(1200)), rtf: Some(vec![0; 100]), ..Default::default() };
    assert_eq!(clip::summary(&rich), "rich text, 1.3 KB");
    let png = png_of(1200, 800);
    let image = Contents { png: Some(png.clone()), ..Default::default() };
    assert!(clip::summary(&image).starts_with("image, "), "{}", clip::summary(&image));
}

#[test]
fn the_notice_shows_a_short_first_line() {
    assert_eq!(clip::preview(&text("meeting notes")), "“meeting notes”");
    assert_eq!(
        clip::preview(&text("A long first line that goes on and on and on\nsecond line")),
        "“A long first line that goes on…”"
    );
    assert_eq!(clip::preview(&text("   \n  trimmed  ")), "“trimmed”");
    let image = Contents { png: Some(png_of(1200, 800)), ..Default::default() };
    assert_eq!(clip::preview(&image), "An image, 1200 × 800");
}

/// A real PNG header of the given size (no pixels needed for these tests).
fn png_of(w: u32, h: u32) -> Vec<u8> {
    let mut v = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    v.extend_from_slice(&w.to_be_bytes());
    v.extend_from_slice(&h.to_be_bytes());
    v.extend_from_slice(&[8, 6, 0, 0, 0]);
    v
}
