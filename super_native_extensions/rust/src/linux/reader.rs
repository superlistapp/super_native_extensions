use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    os::raw::c_uint,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use gdk::{glib::SignalHandlerId, prelude::ObjectExt, Atom, Display, DragContext};
use gtk::{traits::WidgetExt, Clipboard, SelectionData, Widget};

use irondash_message_channel::{Late, Value};
use irondash_run_loop::{spawn, util::FutureCompleter};
use url::Url;

use crate::{
    error::{NativeExtensionsError, NativeExtensionsResult},
    reader_manager::{ReadProgress, VirtualFileReader},
};

use super::{
    clipboard_async::ClipboardAsync,
    common::{target_includes_text, TYPE_TEXT, TYPE_URI},
};

pub struct PlatformDataReader {
    reader: Reader,
    initializing: Cell<bool>,
    inner: Late<Inner>,
}

struct Inner {
    targets: Vec<String>,
    uris: Vec<String>,
}

enum Reader {
    Clipboard(ClipboardReader),
    Widget(Rc<WidgetReader>),
}

impl Reader {
    async fn get_targets(&self) -> Vec<String> {
        match self {
            Reader::Clipboard(clipboard) => clipboard.get_targets().await,
            Reader::Widget(widget) => widget.get_targets().await,
        }
    }

    async fn get_uri_list(&self) -> Vec<String> {
        match self {
            Reader::Clipboard(clipboard) => clipboard.get_uri_list().await,
            Reader::Widget(widget) => widget.get_uri_list().await,
        }
    }

    async fn get_text(&self) -> Option<String> {
        match self {
            Reader::Clipboard(clipboard) => clipboard.get_text().await,
            Reader::Widget(widget) => widget.get_text().await,
        }
    }

    async fn get_data(&self, ty: &str) -> Option<Vec<u8>> {
        match self {
            Reader::Clipboard(clipboard) => clipboard.get_data(ty).await,
            Reader::Widget(widget) => widget.get_data(ty).await,
        }
    }
}

pub struct ReaderInfo {
    pub number_of_items: usize,
    pub targets: Vec<String>,
}

impl PlatformDataReader {
    async fn init(&self) {
        if !self.inner.is_set() && !self.initializing.get() {
            self.initializing.set(true);
            let mut targets = self.reader.get_targets().await;
            let has_text = targets
                .iter()
                .any(|t| target_includes_text(&Atom::intern(t)));
            if has_text {
                // framework part only recognizes text/plain as text. Make sure
                // to include it in types.
                let has_text_type = targets.iter().any(|t| t == TYPE_TEXT);
                if !has_text_type {
                    targets.push(TYPE_TEXT.into());
                }
            }
            let uris = if targets.iter().any(|t| t == TYPE_URI) {
                self.reader.get_uri_list().await
            } else {
                Vec::new()
            };
            // double check - we might have been preempted
            if !self.inner.is_set() {
                self.inner.set(Inner { targets, uris })
            }
        }
    }

    pub fn reader_info(self: &Rc<Self>) -> Option<ReaderInfo> {
        if self.inner.is_set() {
            Some(ReaderInfo {
                number_of_items: 1.max(self.inner.uris.len()),
                targets: self.inner.targets.clone(),
            })
        } else {
            let this = self.clone();
            spawn(async move {
                this.init().await;
            });
            None
        }
    }

    pub async fn get_items(&self) -> NativeExtensionsResult<Vec<i64>> {
        self.init().await;
        // uris from urilist are represented as separate items
        let num_items = 1.max(self.inner.uris.len());
        Ok((0..num_items as i64).collect())
    }

    pub async fn get_formats_for_item(&self, item: i64) -> NativeExtensionsResult<Vec<String>> {
        self.init().await;
        if item == 0 {
            Ok(self.inner.targets.clone())
        } else if (item as usize) < self.inner.uris.len() {
            Ok(vec![TYPE_URI.into()])
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get_suggested_name_for_item(
        &self,
        item: i64,
    ) -> NativeExtensionsResult<Option<String>> {
        let item = item as usize;
        let uri = self.inner.uris.get(item).and_then(|u| Url::parse(u).ok());
        if let Some(uri) = uri {
            if let Some(segments) = uri.path_segments() {
                let last: Option<&str> = segments.last().filter(|s| !s.is_empty());
                return Ok(last.map(|f| f.to_owned()));
            }
        }
        Ok(None)
    }

    pub async fn get_item_format_for_uri(
        &self,
        item: i64,
    ) -> NativeExtensionsResult<Option<String>> {
        let item = item as usize;
        let uri = self.inner.uris.get(item).and_then(|u| Url::parse(u).ok());
        if let Some(uri) = uri {
            let name: Option<&str> = uri.path_segments().and_then(|s| s.last());
            match name {
                Some(name) => {
                    let format = mime_from_name(name);
                    Ok(Some(format))
                }
                None => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    pub async fn get_data_for_item(
        &self,
        item: i64,
        data_type: String,
        _progress: Option<Arc<ReadProgress>>,
    ) -> NativeExtensionsResult<Value> {
        let item = item as usize;
        if data_type == TYPE_URI && item < self.inner.uris.len() {
            Ok(self.inner.uris[item].clone().into())
        } else if item == 0 {
            let target = Atom::intern(&data_type);
            let is_text = target_includes_text(&target);
            if is_text {
                Ok(self.reader.get_text().await.into())
            } else {
                Ok(self.reader.get_data(&data_type).await.into())
            }
        } else {
            Ok(Value::Null)
        }
    }

    pub fn new_clipboard_reader() -> NativeExtensionsResult<Rc<Self>> {
        unsafe { gtk::set_initialized() };
        let display = Display::default()
            .ok_or_else(|| NativeExtensionsError::OtherError("Display not found".into()))?;
        let clipboard = Clipboard::default(&display)
            .ok_or_else(|| NativeExtensionsError::OtherError("Clipboard not found".into()))?;
        let res = Rc::new(PlatformDataReader {
            reader: Reader::Clipboard(ClipboardReader { clipboard }),
            initializing: Cell::new(false),
            inner: Late::new(),
        });
        Ok(res)
    }

    pub fn new_with_widget_reader(
        widget_reader: Rc<WidgetReader>,
    ) -> NativeExtensionsResult<Rc<Self>> {
        Ok(Rc::new(PlatformDataReader {
            reader: Reader::Widget(widget_reader),
            initializing: Cell::new(false),
            inner: Late::new(),
        }))
    }

    pub fn item_format_is_synthesized(
        &self,
        _item: i64,
        _format: &str,
    ) -> NativeExtensionsResult<bool> {
        Ok(false)
    }

    pub async fn can_copy_virtual_file_for_item(
        &self,
        _item: i64,
        _format: &str,
    ) -> NativeExtensionsResult<bool> {
        Ok(false)
    }

    pub async fn can_read_virtual_file_for_item(
        &self,
        _item: i64,
        _format: &str,
    ) -> NativeExtensionsResult<bool> {
        Ok(false)
    }

    pub async fn create_virtual_file_reader_for_item(
        &self,
        _item: i64,
        _format: &str,
        _progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<Option<Rc<dyn VirtualFileReader>>> {
        Ok(None)
    }

    pub async fn copy_virtual_file_for_item(
        &self,
        _item: i64,
        _format: &str,
        _target_folder: PathBuf,
        _progress: Arc<ReadProgress>,
    ) -> NativeExtensionsResult<PathBuf> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }
}

struct ClipboardReader {
    clipboard: Clipboard,
}

impl ClipboardReader {
    async fn get_targets(&self) -> Vec<String> {
        self.clipboard.get_targets().await
    }

    async fn get_uri_list(&self) -> Vec<String> {
        self.clipboard.get_uri_list().await
    }

    async fn get_text(&self) -> Option<String> {
        self.clipboard.get_text().await
    }

    async fn get_data(&self, ty: &str) -> Option<Vec<u8>> {
        self.clipboard.get_data(ty).await
    }
}

pub struct WidgetReader {
    drag_context: DragContext,
    widget: Widget,
    data_received_sig: Cell<Option<SignalHandlerId>>,
    current_time: Cell<u32>,
    pending: RefCell<HashMap<usize, Vec<FutureCompleter<SelectionData>>>>,
    on_all_requests_resolved: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl WidgetReader {
    pub fn new(drag_context: DragContext, widget: Widget) -> Rc<Self> {
        let res = Rc::new(Self {
            drag_context,
            widget: widget.clone(),
            data_received_sig: Cell::new(None),
            current_time: Cell::new(0),
            pending: RefCell::new(HashMap::new()),
            on_all_requests_resolved: RefCell::new(None),
        });
        let weak = Rc::downgrade(&res);
        res.data_received_sig
            .set(Some(widget.connect_drag_data_received(
                move |_, c, _x, _y, s, i, t| {
                    if let Some(this) = weak.upgrade() {
                        this.drag_data_received(c, s, i, t);
                    }
                },
            )));
        res
    }

    pub fn update_current_time(&self, time: u32) {
        self.current_time.set(time);
    }

    async fn get_targets(&self) -> Vec<String> {
        self.drag_context
            .list_targets()
            .iter()
            .map(|a| a.name().as_str().into())
            .collect()
    }

    fn request_data_if_needed(&self, format: Atom, completer: FutureCompleter<SelectionData>) {
        let first = {
            let mut pending = self.pending.borrow_mut();
            let entry = pending.entry(format.value()).or_default();
            let first = entry.is_empty();
            entry.push(completer);
            first
        };
        if first {
            self.widget
                .drag_get_data(&self.drag_context, &format, self.current_time.get());
        }
    }

    async fn get_uri_list(&self) -> Vec<String> {
        let (future, completer) = FutureCompleter::new();
        self.request_data_if_needed(Atom::intern(TYPE_URI), completer);
        let data: SelectionData = future.await;
        data.uris().iter().map(|a| a.as_str().to_owned()).collect()
    }

    async fn get_text(&self) -> Option<String> {
        let first_text_type = self
            .drag_context
            .list_targets()
            .iter()
            .find(|t| target_includes_text(t))
            .cloned()?;
        let (future, completer) = FutureCompleter::new();
        self.request_data_if_needed(first_text_type, completer);
        let data: SelectionData = future.await;
        data.text().map(|t| t.as_str().to_owned())
    }

    async fn get_data(&self, format: &str) -> Option<Vec<u8>> {
        let format = Atom::intern(format);
        let (future, completer) = FutureCompleter::new();
        self.request_data_if_needed(format, completer);
        let data: SelectionData = future.await;
        Some(data.data())
    }

    fn drag_data_received(
        &self,
        _context: &DragContext,
        data: &SelectionData,
        _info: c_uint,
        _time: c_uint,
    ) {
        if let Some(completers) = self.pending.borrow_mut().remove(&data.data_type().value()) {
            for c in completers {
                c.complete(data.clone())
            }
        }
        if self.pending.borrow().is_empty() {
            let cb = self.on_all_requests_resolved.borrow_mut().take();
            if let Some(cb) = cb {
                cb();
            }
        }
    }

    /// Invokes the provided callback when there are no more pending get
    /// requests.
    pub fn on_all_requests_resolved<F>(&self, f: F)
    where
        F: FnOnce() + 'static,
    {
        if self.pending.borrow().is_empty() {
            f();
        } else {
            self.on_all_requests_resolved.replace(Some(Box::new(f)));
        }
    }
}

impl Drop for WidgetReader {
    fn drop(&mut self) {
        self.widget
            .disconnect(self.data_received_sig.replace(None).unwrap());
    }
}

fn mime_from_name(name: &str) -> String {
    let ext = Path::new(name).extension();
    mime_guess::from_path(name)
        .first()
        .map(|m| m.to_string())
        .unwrap_or_else(|| {
            format!(
                "application/octet-stream;extension={}",
                ext.unwrap_or_default().to_string_lossy()
            )
        })
}
