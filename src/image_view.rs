//! Image files in the viewer panel, kept off the UI thread.
//!
//! Decoding an image and encoding it for the terminal's graphics protocol both
//! take long enough to stall a redraw — a large photo re-encoded during a
//! render freezes the whole TUI. So every open image gets a worker thread: it
//! decodes the file once, then answers the resize/encode requests the widget
//! makes whenever the panel changes size. The UI thread only ever draws what
//! the worker has already produced, and shows a placeholder until then.
//!
//! The worker is owned by the [`ImageView`]: dropping the view (the file list
//! moved to another entry, or the panel closed) drops both channel ends, which
//! is what tells the worker to stop. A late answer from a worker for a file the
//! viewer has left cannot be mistaken for the current one, because it arrives on
//! a receiver that no longer exists.

use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};

use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::thread::{ResizeRequest, ResizeResponse, ThreadProtocol};

use crate::model::{Model, ViewContent};

/// Upper bound on the size of an image file the viewer will decode. Higher than
/// the text limit because encoded images are compressed, but still bounded:
/// moving the selection decodes whatever it lands on.
const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;

/// What a worker sends back to the UI thread.
enum ImageMsg {
    /// The file was read and decoded, or could not be.
    Decoded(Result<Box<StatefulProtocol>, String>),
    /// The image was re-encoded for the panel's current size.
    Resized(Box<ResizeResponse>),
    /// Re-encoding failed — an unsupported protocol, or a size the terminal
    /// rejected.
    Failed(String),
}

/// An image displayed in the viewer panel, plus the worker producing it.
pub struct ImageView {
    /// Holds the decoded image between renders and hands it to the worker while
    /// a resize is outstanding.
    pub protocol: ThreadProtocol,
    rx: Receiver<ImageMsg>,
    /// Whether the worker has answered the initial decode. Until it has, the
    /// panel shows a placeholder rather than an empty box.
    decoded: bool,
    pub error: Option<String>,
}

impl ImageView {
    /// Whether the worker still owes an answer, so the event loop must keep
    /// polling instead of blocking on the keyboard.
    pub fn is_pending(&self) -> bool {
        self.protocol.protocol_type().is_none() && self.error.is_none()
    }

    /// Whether the initial decode has come back, either way.
    pub fn is_decoded(&self) -> bool {
        self.decoded
    }
}

/// Start a worker for `path` and return the view it feeds, or `None` when the
/// path is not an image the `image` crate recognises by extension — that entry
/// belongs to the viewer's text path instead.
pub fn open(picker: &Picker, path: &Path) -> Option<ImageView> {
    if image::ImageFormat::from_path(path).is_err() {
        return None;
    }
    let (msg_tx, rx) = channel();
    let (resize_tx, resize_rx) = channel();
    let picker = picker.clone();
    let path = path.to_path_buf();
    std::thread::spawn(move || worker(&picker, &path, &resize_rx, &msg_tx));
    Some(ImageView {
        protocol: ThreadProtocol::new(resize_tx, None),
        rx,
        decoded: false,
        error: None,
    })
}

/// Decode `path` once, then serve resize requests until the view goes away.
fn worker(
    picker: &Picker,
    path: &Path,
    requests: &Receiver<ResizeRequest>,
    out: &Sender<ImageMsg>,
) {
    if out.send(ImageMsg::Decoded(decode(picker, path))).is_err() {
        return;
    }
    while let Ok(request) = requests.recv() {
        let msg = match request.resize_encode() {
            Ok(response) => ImageMsg::Resized(Box::new(response)),
            Err(err) => ImageMsg::Failed(err.to_string()),
        };
        if out.send(msg).is_err() {
            return;
        }
    }
}

/// Read and decode the file, and build the protocol the terminal can draw.
fn decode(picker: &Picker, path: &Path) -> Result<Box<StatefulProtocol>, String> {
    let format = image::ImageFormat::from_path(path).map_err(|e| e.to_string())?;
    let len = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if len > MAX_IMAGE_BYTES {
        return Err(format!("image too large to view ({len} bytes)"));
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let image = image::load_from_memory_with_format(&bytes, format).map_err(|e| e.to_string())?;
    Ok(Box::new(picker.new_resize_protocol(image)))
}

/// Move whatever the worker has produced into the open image view. Returns
/// whether the panel changed and should be redrawn.
pub fn drain(model: &mut Model) -> bool {
    let Some(view) = image_view_mut(model) else {
        return false;
    };
    let mut changed = false;
    while let Ok(msg) = view.rx.try_recv() {
        changed = true;
        match msg {
            ImageMsg::Decoded(Ok(protocol)) => {
                view.decoded = true;
                view.error = None;
                view.protocol.replace_protocol(*protocol);
            }
            ImageMsg::Decoded(Err(err)) | ImageMsg::Failed(err) => {
                view.decoded = true;
                view.error = Some(err);
            }
            // A response for a size the panel has since moved past is dropped by
            // the protocol itself, which tracks the request it is waiting on.
            ImageMsg::Resized(response) => {
                view.protocol.update_resized_protocol(*response);
            }
        }
    }
    changed
}

/// Whether an open image is still waiting on its worker.
pub fn pending(model: &Model) -> bool {
    match model.file_view.as_ref().map(|v| &v.content) {
        Some(ViewContent::Image(view)) => view.is_pending(),
        _ => false,
    }
}

fn image_view_mut(model: &mut Model) -> Option<&mut ImageView> {
    match &mut model.file_view.as_mut()?.content {
        ViewContent::Image(view) => Some(view),
        ViewContent::Text(_) => None,
    }
}
