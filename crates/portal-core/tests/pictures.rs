//! Device pictures between paired computers (Mission 05 §2.3): a picture
//! set here shows on the other computer once checked, the default comes
//! back when it's cleared, and one that fails the checks is refused here.
//! Two cores in one process over loopback.

mod common;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use common::{pair, start};
use portal_core::events::Snapshot;
use portal_core::model::DeviceId;
use portal_core::picture::{Icon, Picture};

fn png_of(w: u32, h: u32) -> String {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().unwrap();
        wr.write_image_data(&vec![90u8; (w * h * 4) as usize]).unwrap();
    }
    B64.encode(out)
}

fn picture_of(s: &Snapshot, id: &DeviceId) -> Option<Picture> {
    s.peers.iter().find(|p| &p.id == id).and_then(|p| p.picture.clone())
}

#[tokio::test]
async fn a_picture_set_here_shows_on_the_paired_computer() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    let mac_id = mac.id().await;

    let mark = Picture::Mark { color: 0x3366ff, icon: Icon::Laptop };
    mac.core.set_picture(Some(mark.clone())).await.unwrap();
    pc.wait_for("the Mac's colour and icon", |s| picture_of(s, &mac_id) == Some(mark.clone())).await;

    let photo = Picture::Photo { png: png_of(96, 96) };
    mac.core.set_picture(Some(photo.clone())).await.unwrap();
    pc.wait_for("the Mac's photo", |s| picture_of(s, &mac_id) == Some(photo.clone())).await;

    mac.core.set_picture(None).await.unwrap();
    pc.wait_for("the default again", |s| s.peers.iter().any(|p| p.id == mac_id) && picture_of(s, &mac_id).is_none())
        .await;
}

#[tokio::test]
async fn a_picture_that_fails_the_checks_is_refused_here() {
    let mac = start("MacBook Pro", |_| {}).await;
    for bad in [
        Picture::Photo { png: png_of(512, 512) },
        Picture::Photo { png: B64.encode(b"<svg xmlns='http://www.w3.org/2000/svg'/>") },
        Picture::Photo { png: "not base64 at all!".into() },
    ] {
        assert!(mac.core.set_picture(Some(bad)).await.is_err());
    }
    assert_eq!(mac.core.snapshot().await.unwrap().me.picture, None, "nothing kept");
}
