use std::{ffi::c_void, sync::Arc};

use objc2::{
    declare_class, extern_methods,
    ffi::{objc_setAssociatedObject, OBJC_ASSOCIATION_RETAIN},
    msg_send_id, mutability,
    rc::{Id, WeakId},
    runtime::NSObject,
    ClassType, DeclaredClass,
};
use objc2_foundation::{ns_string, NSKeyValueObservingOptions, NSProgress, NSString};

use crate::{reader_manager::ReadProgress, util::Movable};

/// Bridges NSProgress to ReadProgress. Will retain ReadProgress for as long as the
/// NSProgress is alive.
#[allow(dead_code)]
pub fn bridge_progress(ns_progress: Id<NSProgress>, read_progress: Arc<ReadProgress>) {
    let bridge = SNEProgressBridge::new(ProgressBridgeInner {
        ns_progress: WeakId::new(&ns_progress),
        read_progress: read_progress.clone(),
    });
    let key = &ASSOCIATED_OBJECT_KEY as *const _ as *const c_void;
    unsafe {
        objc_setAssociatedObject(
            Id::as_ptr(&ns_progress) as *mut _,
            key,
            Id::as_ptr(&bridge) as *mut _,
            OBJC_ASSOCIATION_RETAIN,
        )
    }
    let cancellable = unsafe { ns_progress.isCancellable() };
    if cancellable {
        let weak = WeakId::<NSProgress>::new(&ns_progress);
        let weak = unsafe { Movable::new(weak) };
        read_progress.set_cancellation_handler(Some(Box::new(move || {
            let progress = weak.load();
            if let Some(progress) = progress {
                unsafe {
                    progress.cancel();
                }
            }
        })));
    }
}

static ASSOCIATED_OBJECT_KEY: char = 'k';

pub struct ProgressBridgeInner {
    ns_progress: WeakId<NSProgress>,
    read_progress: Arc<ReadProgress>,
}

impl ProgressBridgeInner {
    fn update(&mut self) {
        if let Some(ns_progress) = self.ns_progress.load() {
            let fraction = unsafe { ns_progress.fractionCompleted() };
            self.read_progress.report_progress(Some(fraction));
        }
    }
}

declare_class!(
    struct SNEProgressBridge;

    unsafe impl ClassType for SNEProgressBridge {
        type Super = NSObject;
        type Mutability = mutability::Mutable;
        const NAME: &'static str = "SNEProgressBridge";
    }

    impl DeclaredClass for SNEProgressBridge {
        type Ivars = ProgressBridgeInner;
    }

    unsafe impl SNEProgressBridge {
        #[method(observeValueForKeyPath:ofObject:change:context:)]
        fn observe(
            &mut self,
            _path: &NSString,
            _object: &NSObject,
            _change: &NSObject,
            _context: *mut c_void,
        ) {
            self.ivars_mut().update();
        }
    }
);

impl SNEProgressBridge {
    fn new(inner: ProgressBridgeInner) -> Id<Self> {
        let this = Self::alloc();
        let this = this.set_ivars(inner);
        let this: Id<Self> = unsafe { msg_send_id![super(this), init] };
        if let Some(progress) = this.ivars().ns_progress.load() {
            // https://github.com/madsmtm/objc2/issues/531
            unsafe {
                let progress = Id::cast::<NSProgressKVO>(progress);
                progress.addObserver_forKeyPath_options_context(
                    &this,
                    ns_string!("fractionCompleted"),
                    NSKeyValueObservingOptions::NSKeyValueObservingOptionInitial,
                    std::ptr::null_mut(),
                );
            }
        }
        this
    }
}

impl Drop for SNEProgressBridge {
    fn drop(&mut self) {
        if let Some(progress) = self.ivars().ns_progress.load() {
            unsafe {
                let progress = Id::cast::<NSProgressKVO>(progress);
                progress.removeObserver_forKeyPath_context(
                    self,
                    ns_string!("fractionCompleted"),
                    std::ptr::null_mut(),
                );
            }
        }
    }
}

objc2::extern_class!(
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSProgressKVO;

    unsafe impl ClassType for NSProgressKVO {
        type Super = NSObject;
        type Mutability = mutability::Mutable;
    }
);

extern_methods!(
    unsafe impl NSProgressKVO {
        #[method(addObserver:forKeyPath:options:context:)]
        #[allow(non_snake_case)]
        pub unsafe fn addObserver_forKeyPath_options_context(
            &self,
            observer: &NSObject,
            key_path: &NSString,
            options: NSKeyValueObservingOptions,
            context: *mut c_void,
        );

        #[method(removeObserver:forKeyPath:context:)]
        #[allow(non_snake_case)]
        pub unsafe fn removeObserver_forKeyPath_context(
            &self,
            observer: &NSObject,
            key_path: &NSString,
            context: *mut c_void,
        );
    }
);

#[cfg(test)]
mod tests {
    use std::{cell::Cell, sync::Arc};

    use block2::RcBlock;
    use objc2_foundation::NSProgress;

    use crate::{reader_manager::ReadProgress, util::DropNotifier, value_promise::Promise};

    #[test]
    fn test_cancellation() {
        let cancellable = Arc::new(Cell::new(false));

        let cancelled = Arc::new(Promise::new());
        let ns_progress = unsafe { NSProgress::new() };

        let cancellable_clone = cancellable.clone();
        let read_progress = Arc::new(ReadProgress::new(
            Arc::new(DropNotifier::new(move || {})),
            move |c| {
                cancellable_clone.set(c);
            },
            move |_| {},
        ));

        let cancelled_clone = cancelled.clone();
        let handler = RcBlock::new(move || {
            cancelled_clone.set(true);
        });
        unsafe { ns_progress.setCancellationHandler(Some(&handler)) };

        assert!(!cancellable.get());
        super::bridge_progress(ns_progress.clone(), read_progress.clone());
        assert!(cancellable.get());

        assert!(cancelled.try_take().is_none());
        read_progress.cancel();
        assert!(cancelled.wait());
    }

    #[test]
    fn test_progress_bridge() {
        let progress = Arc::new(Cell::new(0.0));
        let dropped = Arc::new(Cell::new(false));
        {
            let ns_progress = unsafe { NSProgress::new() };
            {
                let progress = progress.clone();
                let dropped = dropped.clone();
                let read_progress = Arc::new(ReadProgress::new(
                    Arc::new(DropNotifier::new(move || {
                        dropped.set(true);
                    })),
                    |_| {},
                    move |p| {
                        progress.set(p.unwrap());
                    },
                ));
                super::bridge_progress(ns_progress.clone(), read_progress);
            }
            unsafe {
                assert_eq!(progress.get(), 0.0);
                ns_progress.setCancellable(true);
                ns_progress.setTotalUnitCount(100);
                ns_progress.setCompletedUnitCount(50);
                assert_eq!(progress.get(), 0.5);
            }
            assert!(!dropped.get());
        };
        assert!(dropped.get());
    }
}
