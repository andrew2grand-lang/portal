//! macOS glue: app activation (a notification was clicked) and the early
//! Local Network privacy prompt.

use std::ptr::NonNull;

use block2::RcBlock;
use objc2_app_kit::NSApplicationDidBecomeActiveNotification;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSOperationQueue};

/// Calls `f` on the main thread whenever the app becomes active.
pub fn on_activate(f: impl Fn() + 'static) {
    let block = RcBlock::new(move |_note: NonNull<NSNotification>| f());
    let center = NSNotificationCenter::defaultCenter();
    // SAFETY: the name is a valid AppKit notification name; the block is only
    // called on the main queue, which is where Tauri's UI work must happen.
    let observer = unsafe {
        center.addObserverForName_object_queue_usingBlock(
            Some(NSApplicationDidBecomeActiveNotification),
            None,
            Some(&NSOperationQueue::mainQueue()),
            &block,
        )
    };
    // Observe for the app's whole life.
    std::mem::forget(observer);
}
