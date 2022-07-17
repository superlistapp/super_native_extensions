use std::{
    ffi::c_void,
    mem::ManuallyDrop,
    os::raw::c_char,
    rc::{Rc, Weak},
    slice,
};

use cocoa::{
    appkit::NSPasteboard,
    base::{id, nil},
    foundation::{NSArray, NSString, NSUInteger},
};
use core_foundation::runloop::{kCFRunLoopDefaultMode, CFRunLoopRunInMode};
use nativeshell_core::{platform::value::ValueObjcConversion, util::Late, IsolateId};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{Class, Object, Protocol, Sel},
    sel, sel_impl,
};
use once_cell::sync::Lazy;

use crate::{
    error::NativeExtensionsResult,
    log::OkLog,
    value_promise::ValuePromiseResult,
    writer_data::{DataSource, ClipboardWriterItemData},
    writer_manager::PlatformDataSourceDelegate,
};

pub struct PlatformClipboardWriter {
    weak_self: Late<Weak<Self>>,
    delegate: Weak<dyn PlatformDataSourceDelegate>,
    isolate_id: IsolateId,
    data: DataSource,
}

impl PlatformClipboardWriter {
    pub fn new(
        delegate: Weak<dyn PlatformDataSourceDelegate>,
        isolate_id: IsolateId,
        data: DataSource,
    ) -> Self {
        Self {
            delegate,
            data,
            isolate_id,
            weak_self: Late::new(),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    pub fn create_items(&self) -> Vec<id> {
        let mut items = Vec::<id>::new();
        for item in self.data.items.iter().enumerate() {
            let state = Rc::new(ItemState {
                clipboard: self.weak_self.clone(),
                index: item.0,
            });
            let item = state.create_item();
            items.push(item.autorelease());
        }
        items
    }

    pub async fn write_to_clipboard(&self) -> NativeExtensionsResult<()> {
        autoreleasepool(|| unsafe {
            let items = self.create_items();
            let array = NSArray::arrayWithObjects(nil, &items);
            let pasteboard = NSPasteboard::generalPasteboard(nil);
            NSPasteboard::clearContents(pasteboard);
            NSPasteboard::writeObjects(pasteboard, array);
        });
        Ok(())
    }
}

struct ItemState {
    clipboard: Weak<PlatformClipboardWriter>,
    index: usize,
}

impl ItemState {
    fn create_item(self: Rc<Self>) -> StrongPtr {
        unsafe {
            let item: id = msg_send![*PASTEBOARD_WRITER_CLASS, alloc];
            let () = msg_send![item, init];
            (*item).set_ivar("sneState", Rc::into_raw(self) as *mut c_void);
            StrongPtr::new(item)
        }
    }

    fn writable_types(&self) -> id {
        match self.clipboard.upgrade() {
            Some(clipboard) => {
                let item = &clipboard.data.items[self.index];
                let types: Vec<_> = item
                    .data
                    .iter()
                    .filter_map(|d| match d {
                        ClipboardWriterItemData::Simple { types, data: _ } => Some(
                            types
                                .iter()
                                .map(|t| to_nsstring(t).autorelease())
                                .collect::<Vec<_>>(),
                        ),
                        ClipboardWriterItemData::Lazy { types, id: _ } => Some(
                            types
                                .iter()
                                .map(|t| to_nsstring(t).autorelease())
                                .collect::<Vec<_>>() as Vec<_>,
                        ),
                        ClipboardWriterItemData::VirtualFile {
                            file_size: _,
                            file_name: _,
                        } => None,
                    })
                    .flatten()
                    .collect();
                unsafe { NSArray::arrayWithObjects(nil, &types) }
            }
            None => nil,
        }
    }

    fn object_for_type(&self, pasteboard_type: id) -> id {
        match self.clipboard.upgrade() {
            Some(clipboard) => {
                let ty = unsafe { from_nsstring(pasteboard_type) };
                let item = &clipboard.data.items[self.index];
                for data in &item.data {
                    match data {
                        ClipboardWriterItemData::Simple { types, data } => {
                            if types.contains(&ty) {
                                return data
                                    .to_objc()
                                    .ok_log()
                                    .map(|a| a.autorelease())
                                    .unwrap_or(nil);
                            }
                        }
                        ClipboardWriterItemData::Lazy { types, id } => {
                            if types.contains(&ty) {
                                if let Some(delegate) = clipboard.delegate.upgrade() {
                                    let promise =
                                        delegate.get_lazy_data(clipboard.isolate_id, *id, None);
                                    loop {
                                        if let Some(result) = promise.try_take() {
                                            match result {
                                                ValuePromiseResult::Ok { value } => {
                                                    return value
                                                        .to_objc()
                                                        .ok_log()
                                                        .map(|a| a.autorelease())
                                                        .unwrap_or(nil);
                                                }
                                                ValuePromiseResult::Cancelled => {
                                                    return nil;
                                                }
                                            }
                                        }

                                        Context::get().run_loop().platform_run_loop.poll_once();
                                    }
                                }
                            }
                        }
                        ClipboardWriterItemData::VirtualFile {
                            file_size: _,
                            file_name: _,
                        } => {}
                    }
                }
                nil
            }
            None => nil,
        }
    }
}

fn item_state(this: &Object) -> Rc<ItemState> {
    unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("sneState");
            state_ptr as *const ItemState
        };
        let ptr = Rc::from_raw(state_ptr);
        let res = ptr.clone();
        let _ = ManuallyDrop::new(ptr);
        res
    }
}

extern "C" fn writable_types_for_pasteboard(this: &Object, _sel: Sel, _pasteboard: id) -> id {
    let state = item_state(this);
    state.writable_types()
}

extern "C" fn writing_options_for_type(
    _this: &Object,
    _sel: Sel,
    _type: id,
    _pastaboard: id,
) -> NSUInteger {
    1 << 9 // NSPasteboardWritingPromised
}

extern "C" fn pasteboard_property_list_for_type(this: &Object, _sel: Sel, ty: id) -> id {
    let state = item_state(this);
    state.object_for_type(ty)
}

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let state_ptr = {
            let state_ptr: *mut c_void = *this.get_ivar("sneState");
            state_ptr as *const ItemState
        };
        Rc::from_raw(state_ptr);

        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

static PASTEBOARD_WRITER_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("IMPasteboardWriter", superclass).unwrap();
    decl.add_ivar::<*mut c_void>("sneState");
    if let Some(protocol) = Protocol::get("NSPasteboardWriting") {
        decl.add_protocol(protocol);
    }
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    decl.add_method(
        sel!(writableTypesForPasteboard:),
        writable_types_for_pasteboard as extern "C" fn(&Object, Sel, id) -> id,
    );
    decl.add_method(
        sel!(writingOptionsForType:pasteboard:),
        writing_options_for_type as extern "C" fn(&Object, Sel, id, id) -> NSUInteger,
    );
    decl.add_method(
        sel!(pasteboardPropertyListForType:),
        pasteboard_property_list_for_type as extern "C" fn(&Object, Sel, id) -> id,
    );

    decl.register()
});

pub unsafe fn superclass(this: &Object) -> &Class {
    let superclass: id = msg_send![this, superclass];
    &*(superclass as *const _)
}

pub fn to_nsstring(string: &str) -> StrongPtr {
    unsafe {
        let ptr = NSString::alloc(nil).init_str(string);
        StrongPtr::new(ptr)
    }
}

pub unsafe fn from_nsstring(ns_string: id) -> String {
    let bytes: *const c_char = msg_send![ns_string, UTF8String];
    let bytes = bytes as *const u8;

    let len = ns_string.len();

    let bytes = slice::from_raw_parts(bytes, len);
    std::str::from_utf8(bytes).unwrap().into()
}
