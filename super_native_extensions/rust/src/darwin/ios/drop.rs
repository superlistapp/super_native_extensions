use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    iter,
    rc::{Rc, Weak},
};

use block2::RcBlock;
use irondash_engine_context::EngineContext;
use irondash_message_channel::{Late, Value};
use irondash_run_loop::{platform::PollSession, RunLoop};
use objc2::{
    declare_class, msg_send_id, mutability,
    rc::Id,
    runtime::{NSObject, NSObjectProtocol, ProtocolObject},
    ClassType, DeclaredClass,
};
use objc2_foundation::{CGPoint, CGRect, MainThreadMarker};
use objc2_ui_kit::{
    UIDragDropSession, UIDragItem, UIDragPreviewParameters, UIDragPreviewTarget, UIDropInteraction,
    UIDropInteractionDelegate, UIDropOperation, UIDropProposal, UIDropSession,
    UIDropSessionProgressIndicatorStyle, UITargetedDragPreview, UIView, UIViewAnimationOptions,
};

use crate::{
    api_model::{DropOperation, Size},
    drop_manager::{
        BaseDropEvent, DropEvent, DropItem, DropItemId, DropSessionId, ItemPreview,
        ItemPreviewRequest, PlatformDropContextDelegate, PlatformDropContextId,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::OkLog,
    value_promise::PromiseResult,
};

use super::{
    drag_common::DropOperationExt,
    util::{image_view_from_data, CGAffineTransformMakeScale, IgnoreInteractionEvents},
    PlatformDataReader,
};

pub struct PlatformDropContext {
    id: PlatformDropContextId,
    weak_self: Late<Weak<Self>>,
    view: Id<UIView>,
    delegate: Weak<dyn PlatformDropContextDelegate>,
    interaction: Late<Id<UIDropInteraction>>,
    interaction_delegate: Late<Id<SNEDropContext>>,
    sessions: RefCell<HashMap<DropSessionId, Rc<Session>>>,
    mtm: MainThreadMarker,
}

struct Session {
    context_id: PlatformDropContextId,
    context_delegate: Weak<dyn PlatformDropContextDelegate>,
    context_view: Id<UIView>,
    platform_session: Id<ProtocolObject<dyn UIDropSession>>,
    last_operation: Cell<DropOperation>,
    view_containers: RefCell<Vec<Id<UIView>>>,
    mtm: MainThreadMarker,
}

impl Drop for Session {
    fn drop(&mut self) {
        let containers = self.view_containers.borrow();
        for container in containers.iter() {
            unsafe { container.removeFromSuperview() };
        }
    }
}

trait ItemId {
    fn item_id(&self) -> DropItemId;
}

impl ItemId for UIDragItem {
    fn item_id(&self) -> DropItemId {
        (self as *const _ as i64).into()
    }
}

impl Session {
    fn context_delegate(&self) -> NativeExtensionsResult<Rc<dyn PlatformDropContextDelegate>> {
        self.context_delegate
            .upgrade()
            .ok_or_else(|| NativeExtensionsError::OtherError("missing context delegate".into()))
    }

    fn session_id_(platform_session: &ProtocolObject<dyn UIDropSession>) -> DropSessionId {
        (platform_session as *const _ as isize).into()
    }

    fn session_id(&self) -> DropSessionId {
        Self::session_id_(&self.platform_session)
    }

    fn create_drop_event(&self, is_perform_drop: bool) -> NativeExtensionsResult<DropEvent> {
        let delegate = self.context_delegate()?;

        let location = unsafe { self.platform_session.locationInView(&self.context_view) };
        let allows_move = unsafe { self.platform_session.allowsMoveOperation() };
        let allowed_operations = if allows_move {
            vec![DropOperation::Copy, DropOperation::Move]
        } else {
            vec![DropOperation::Copy]
        };

        // local data
        let local_session = unsafe { self.platform_session.localDragSession() };
        let local_data = local_session
            .and_then(|session| {
                let drag_contexts = delegate.get_platform_drag_contexts();
                drag_contexts
                    .iter()
                    .map(|c| c.get_local_data(&session))
                    .find(|c| c.is_some())
                    .flatten()
            })
            .unwrap_or_default();

        // formats
        let mut items = Vec::new();
        let session_items = unsafe { self.platform_session.items() };

        for (item, local_data) in session_items
            .iter()
            .zip(local_data.into_iter().chain(iter::repeat(Value::Null)))
        {
            let item_provider = unsafe { item.itemProvider() };
            let mut formats = Vec::<String>::new();
            for f in unsafe { item_provider.registeredTypeIdentifiers().iter() } {
                let f = f.to_string();
                if !formats.contains(&f) {
                    formats.push(f);
                }
            }
            items.push(DropItem {
                item_id: item.item_id(),
                formats,
                local_data,
            });
        }

        let accepted_operation = if is_perform_drop {
            Some(self.last_operation.get())
        } else {
            None
        };

        let reader = if is_perform_drop {
            let platform_reader = PlatformDataReader::new_with_drop_session_items(session_items)?;
            Some(delegate.register_platform_reader(self.context_id, platform_reader))
        } else {
            None
        };

        Ok(DropEvent {
            session_id: self.session_id(),
            location_in_view: location.into(),
            allowed_operations,
            items,
            accepted_operation,
            reader,
        })
    }

    fn session_did_update(self: &Rc<Self>) -> NativeExtensionsResult<Id<UIDropProposal>> {
        let delegate = self.context_delegate()?;
        let event = self.create_drop_event(false)?;

        let allows_move = unsafe { self.platform_session.allowsMoveOperation() };

        let session_clone = self.clone();
        delegate.send_drop_update(
            self.context_id,
            event,
            Box::new(move |res| {
                let mut res = res.ok_log().unwrap_or(DropOperation::None);
                if res == DropOperation::Move && !allows_move {
                    res = DropOperation::Copy;
                }
                session_clone.last_operation.replace(res);
            }),
        );

        let operation: UIDropOperation = self.last_operation.get().to_platform();

        let proposal = unsafe {
            UIDropProposal::initWithDropOperation(self.mtm.alloc::<UIDropProposal>(), operation)
        };
        Ok(proposal)
    }

    fn perform_drop(&self) -> NativeExtensionsResult<()> {
        let event = self.create_drop_event(true)?;
        let delegate = self.context_delegate()?;
        let done = Rc::new(Cell::new(false));
        let done_clone = done.clone();
        // TODO(knopp): Let user override default progress indicator
        unsafe {
            self.platform_session
                .setProgressIndicatorStyle(UIDropSessionProgressIndicatorStyle::None)
        };
        delegate.send_perform_drop(
            self.context_id,
            event,
            Box::new(move |r| {
                r.ok_log();
                done_clone.set(true);
            }),
        );
        let mut poll_session = PollSession::new();
        let _ignore_events = IgnoreInteractionEvents::new();
        while !done.get() {
            RunLoop::current()
                .platform_run_loop
                .poll_once(&mut poll_session);
        }
        Ok(())
    }

    fn session_did_exit(&self) -> NativeExtensionsResult<()> {
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.send_drop_leave(
                self.context_id,
                BaseDropEvent {
                    session_id: self.session_id(),
                },
            );
        }
        Ok(())
    }

    fn session_did_end(&self) -> NativeExtensionsResult<()> {
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.send_drop_ended(
                self.context_id,
                BaseDropEvent {
                    session_id: self.session_id(),
                },
            );
        }
        Ok(())
    }

    const DEFAULT_FADE_OUT_DELAY: f64 = 0.8;
    const DEFAULT_FADE_OUT_DURATION: f64 = 0.3;

    fn get_drop_preview(
        &self,
        response: ItemPreview,
        original_size: Size,
        default: &UITargetedDragPreview,
    ) -> Id<UITargetedDragPreview> {
        let size = response
            .destination_image
            .as_ref()
            .map(|i| Size {
                width: (i.width as f64) / i.device_pixel_ratio.unwrap_or(1.0),
                height: (i.height as f64) / i.device_pixel_ratio.unwrap_or(1.0),
            })
            .unwrap_or(original_size);

        let center: CGPoint = response.destination_rect.center().into();
        let transform = unsafe {
            CGAffineTransformMakeScale(
                response.destination_rect.width / size.width,
                response.destination_rect.height / size.height,
            )
        };

        let view_container = unsafe {
            let bounds = self.context_view.bounds();
            let container = UIView::initWithFrame(self.mtm.alloc::<UIView>(), bounds);
            container.setUserInteractionEnabled(false);
            self.context_view.addSubview(&container);

            let container_clone = container.clone();
            let animation_block = RcBlock::new(move || {
                container_clone.setAlpha(0.0);
            });

            UIView::animateWithDuration_delay_options_animations_completion(
                response
                    .fade_out_duration
                    .unwrap_or(Self::DEFAULT_FADE_OUT_DURATION),
                response
                    .fade_out_delay
                    .unwrap_or(Self::DEFAULT_FADE_OUT_DELAY),
                UIViewAnimationOptions::empty(),
                &animation_block,
                None,
                self.mtm,
            );
            container
        };
        self.view_containers
            .borrow_mut()
            .push(view_container.clone());

        let target = unsafe {
            UIDragPreviewTarget::initWithContainer_center_transform(
                self.mtm.alloc::<UIDragPreviewTarget>(),
                &view_container,
                center,
                transform,
            )
        };

        match response.destination_image {
            Some(image) => unsafe {
                let image_view = image_view_from_data(image, self.mtm);
                view_container.addSubview(&image_view);
                let frame: CGRect = response
                    .destination_rect
                    .translated(-100000.0, -100000.0)
                    .into();
                image_view.setFrame(frame);

                let parameters =
                    UIDragPreviewParameters::init(self.mtm.alloc::<UIDragPreviewParameters>());
                UITargetedDragPreview::initWithView_parameters_target(
                    self.mtm.alloc::<UITargetedDragPreview>(),
                    &image_view,
                    &parameters,
                    &target,
                )
            },
            None => unsafe { default.retargetedPreviewWithTarget(&target) },
        }
    }

    fn preview_for_dropping_item(
        &self,
        item: &UIDragItem,
        default: &UITargetedDragPreview,
    ) -> NativeExtensionsResult<Option<Id<UITargetedDragPreview>>> {
        let delegate = self.context_delegate()?;
        let view = unsafe { default.view() };
        let frame = view.frame();
        let original_size: Size = frame.size.into();
        let preview_promise = delegate.get_preview_for_item(
            self.context_id,
            ItemPreviewRequest {
                session_id: self.session_id(),
                item_id: item.item_id(),
                size: original_size.clone(),
                fade_out_delay: Self::DEFAULT_FADE_OUT_DELAY,
                fade_out_duration: Self::DEFAULT_FADE_OUT_DURATION,
            },
        );
        let mut poll_session = PollSession::new();
        let _ignore_events = IgnoreInteractionEvents::new();
        loop {
            if let Some(result) = preview_promise.try_take() {
                match result {
                    PromiseResult::Ok { value } => match value.preview {
                        Some(preview) => {
                            return Ok(Some(self.get_drop_preview(preview, original_size, default)))
                        }
                        None => return Ok(None),
                    },
                    PromiseResult::Cancelled => return Ok(None),
                }
            }
            RunLoop::current()
                .platform_run_loop
                .poll_once(&mut poll_session);
        }
    }
}

impl PlatformDropContext {
    pub fn new(
        id: PlatformDropContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformDropContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        let mtm = MainThreadMarker::new().unwrap();
        let view = EngineContext::get()?.get_flutter_view(engine_handle)?;
        Ok(Self {
            id,
            weak_self: Late::new(),
            view: unsafe { Id::cast(view) },
            delegate,
            interaction: Late::new(),
            interaction_delegate: Late::new(),
            sessions: RefCell::new(HashMap::new()),
            mtm,
        })
    }

    pub fn register_drop_formats(&self, _formats: &[String]) -> NativeExtensionsResult<()> {
        Ok(())
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
        let delegate = SNEDropContext::new(weak_self, self.mtm);
        self.interaction_delegate.set(delegate.retain());
        let interaction = unsafe {
            UIDropInteraction::initWithDelegate(
                self.mtm.alloc::<UIDropInteraction>(),
                &Id::cast(delegate),
            )
        };
        unsafe { self.view.addInteraction(&Id::cast(interaction.clone())) };
        self.interaction.set(interaction);
    }

    fn get_session(&self, session: &ProtocolObject<dyn UIDropSession>) -> Rc<Session> {
        let session = unsafe { Id::retain(session as *const _ as *mut _) }.unwrap();
        let session_id = Session::session_id_(&session);
        let mtm = self.mtm;
        self.sessions
            .borrow_mut()
            .entry(session_id)
            .or_insert_with(|| {
                Rc::new(Session {
                    context_id: self.id,
                    context_delegate: self.delegate.clone(),
                    context_view: self.view.clone(),
                    platform_session: session,
                    last_operation: Cell::new(DropOperation::None),
                    view_containers: RefCell::new(Vec::new()),
                    mtm,
                })
            })
            .clone()
    }

    fn session_did_update(
        &self,
        session: &ProtocolObject<dyn UIDropSession>,
    ) -> NativeExtensionsResult<Id<UIDropProposal>> {
        self.get_session(session).session_did_update()
    }

    fn perform_drop(
        &self,
        session: &ProtocolObject<dyn UIDropSession>,
    ) -> NativeExtensionsResult<()> {
        self.get_session(session).perform_drop()
    }

    fn session_did_exit(
        &self,
        session: &ProtocolObject<dyn UIDropSession>,
    ) -> NativeExtensionsResult<()> {
        self.get_session(session).session_did_exit()
    }

    fn session_did_end(
        &self,
        session: &ProtocolObject<dyn UIDropSession>,
    ) -> NativeExtensionsResult<()> {
        let session_id = Session::session_id_(session);
        let session = self.sessions.borrow_mut().remove(&session_id);
        match session {
            Some(session) => session.session_did_end(),
            None => Ok(()),
        }
    }

    fn preview_for_dropping_item(
        &self,
        item: &UIDragItem,
        default: &UITargetedDragPreview,
    ) -> NativeExtensionsResult<Option<Id<UITargetedDragPreview>>> {
        let session_for_item = {
            let sessions = self.sessions.borrow();
            sessions.iter().find_map(|(_, s)| {
                let items = unsafe { s.platform_session.items() };
                let contains = unsafe { items.containsObject(item) };
                if contains {
                    Some(s.clone())
                } else {
                    None
                }
            })
        };
        match session_for_item {
            Some(session) => session.preview_for_dropping_item(item, default),
            None => Ok(None),
        }
    }
}

impl Drop for PlatformDropContext {
    fn drop(&mut self) {
        unsafe {
            self.view
                .removeInteraction(&Id::cast(self.interaction.clone()))
        };
    }
}

pub struct Inner {
    context: Weak<PlatformDropContext>,
}

impl Inner {
    fn with_state<F, FR, R>(&self, callback: F, default: FR) -> R
    where
        F: FnOnce(Rc<PlatformDropContext>) -> R,
        FR: FnOnce() -> R,
    {
        let context = self.context.upgrade();
        match context {
            Some(context) => callback(context),
            None => default(),
        }
    }
}

declare_class!(
    struct SNEDropContext;

    unsafe impl ClassType for SNEDropContext {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "SNEDropContext";
    }

    impl DeclaredClass for SNEDropContext {
        type Ivars = Inner;
    }

    unsafe impl NSObjectProtocol for SNEDropContext {}

    #[allow(non_snake_case)]
    unsafe impl UIDropInteractionDelegate for SNEDropContext {
        #[method(dropInteraction:canHandleSession:)]
        fn dropInteraction_canHandleSession(
            &self,
            _interaction: &UIDropInteraction,
            _session: &ProtocolObject<dyn UIDropSession>,
        ) -> bool {
            true
        }

        #[method_id(dropInteraction:sessionDidUpdate:)]
        fn dropInteraction_sessionDidUpdate(
            &self,
            _interaction: &UIDropInteraction,
            session: &ProtocolObject<dyn UIDropSession>,
        ) -> Id<UIDropProposal> {
            let fallback = || unsafe {
                let mtm = MainThreadMarker::new().unwrap();
                UIDropProposal::initWithDropOperation(
                    mtm.alloc::<UIDropProposal>(),
                    UIDropOperation::Cancel,
                )
            };
            self.ivars().with_state(
                |state| {
                    state
                        .session_did_update(session)
                        .ok_log()
                        .unwrap_or_else(fallback)
                },
                fallback,
            )
        }

        #[method(dropInteraction:sessionDidExit:)]
        fn dropInteraction_sessionDidExit(
            &self,
            _interaction: &UIDropInteraction,
            session: &ProtocolObject<dyn UIDropSession>,
        ) {
            self.ivars().with_state(
                |state| state.session_did_exit(session).ok_log().unwrap_or(()),
                || (),
            )
        }

        #[method(dropInteraction:performDrop:)]
        fn dropInteraction_performDrop(
            &self,
            _interaction: &UIDropInteraction,
            session: &ProtocolObject<dyn UIDropSession>,
        ) {
            self.ivars().with_state(
                |state| state.perform_drop(session).ok_log().unwrap_or(()),
                || (),
            )
        }

        #[method(dropInteraction:sessionDidEnd:)]
        fn dropInteraction_sessionDidEnd(
            &self,
            _interaction: &UIDropInteraction,
            session: &ProtocolObject<dyn UIDropSession>,
        ) {
            self.ivars().with_state(
                |state| state.session_did_end(session).ok_log().unwrap_or(()),
                || (),
            )
        }

        #[method_id(dropInteraction:previewForDroppingItem:withDefault:)]
        fn dropInteraction_previewForDroppingItem_withDefault(
            &self,
            _interaction: &UIDropInteraction,
            item: &UIDragItem,
            default_preview: &UITargetedDragPreview,
        ) -> Option<Id<UITargetedDragPreview>> {
            self.ivars().with_state(
                |state| {
                    state
                        .preview_for_dropping_item(item, default_preview)
                        .ok_log()
                        .unwrap_or(None)
                },
                || None,
            )
        }
    }
);

impl SNEDropContext {
    fn new(context: Weak<PlatformDropContext>, mtm: MainThreadMarker) -> Id<Self> {
        let this = mtm.alloc::<Self>();
        let this = this.set_ivars(Inner { context });
        unsafe { msg_send_id![super(this), init] }
    }
}
