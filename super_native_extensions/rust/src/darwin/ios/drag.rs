use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
    time::Duration,
};

use block2::RcBlock;
use irondash_engine_context::EngineContext;
use irondash_message_channel::{Late, Value};
use irondash_run_loop::{platform::PollSession, RunLoop};
use objc2::{
    declare_class, msg_send_id, mutability,
    rc::{Id, Retained},
    runtime::{NSObject, NSObjectProtocol, ProtocolObject},
    ClassType, DeclaredClass,
};
use objc2_foundation::{
    ns_string, CGPoint, CGRect, MainThreadMarker, NSArray, NSDictionary, NSNumber,
};
use objc2_ui_kit::{
    UIColor, UIDragDropSession, UIDragInteraction, UIDragInteractionDelegate, UIDragItem,
    UIDragPreview, UIDragPreviewParameters, UIDragSession, UIDropOperation, UIImageView,
    UIPreviewTarget, UITargetedDragPreview, UIView, UIViewAnimationOptions,
};

use crate::{
    api_model::{DataProviderId, DragConfiguration, DragRequest, DropOperation, Point},
    data_provider_manager::DataProviderHandle,
    drag_manager::{
        DataProviderEntry, DragSessionId, GetAdditionalItemsResult, GetDragConfigurationResult,
        PlatformDragContextDelegate, PlatformDragContextId,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    platform_impl::platform::os::util::IgnoreInteractionEvents,
    util::DropNotifier,
    value_promise::PromiseResult,
};

use super::{
    alpha_to_path::bezier_path_for_alpha,
    drag_common::DropOperationExt,
    util::{image_view_from_data, IntoObjc},
    DataProviderSessionDelegate, PlatformDataProvider,
};

pub struct PlatformDragContext {
    id: PlatformDragContextId,
    weak_self: Late<Weak<Self>>,
    view: Id<UIView>,
    delegate: Weak<dyn PlatformDragContextDelegate>,
    interaction: Late<Id<UIDragInteraction>>,
    interaction_delegate: Late<Id<SNEDragContext>>,
    sessions: RefCell<HashMap<DragSessionId, Rc<Session>>>,
    mtm: MainThreadMarker,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
enum ImageType {
    Lift,
    Drag,
}

struct Session {
    context_id: PlatformDragContextId,
    context_delegate: Weak<dyn PlatformDragContextDelegate>,
    view_container: Id<UIView>,
    session_id: DragSessionId,
    weak_self: Late<Weak<Self>>,
    in_progress: Cell<bool>,
    sent_did_end: Cell<bool>,
    configuration: RefCell<DragConfiguration>,
    data_providers: RefCell<Vec<Arc<DataProviderHandle>>>,
    views: RefCell<HashMap<(usize, ImageType), Id<UIImageView>>>, // index -> view
    mtm: MainThreadMarker,
}

impl Session {
    fn new(
        context_delegate: Weak<dyn PlatformDragContextDelegate>,
        context_view: Id<UIView>,
        platform_drag_context_id: PlatformDragContextId,
        session_id: DragSessionId,
        configuration: DragConfiguration,
        mtm: MainThreadMarker,
    ) -> Self {
        let view_container = unsafe {
            let bounds = context_view.bounds();
            let container = UIView::initWithFrame(mtm.alloc::<UIView>(), bounds);
            container.setUserInteractionEnabled(false);
            context_view.addSubview(&container);
            container
        };
        Self {
            context_delegate,
            view_container,
            context_id: platform_drag_context_id,
            weak_self: Late::new(),
            in_progress: Cell::new(false),
            sent_did_end: Cell::new(false),
            session_id,
            configuration: RefCell::new(configuration),
            data_providers: RefCell::new(Vec::new()),
            views: RefCell::new(HashMap::new()),
            mtm,
        }
    }

    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn create_item(
        &self,
        provider: Rc<PlatformDataProvider>,
        handle: Arc<DataProviderHandle>,
        index: usize,
    ) -> Id<UIDragItem> {
        // Keep the data provider alive for the duration of session
        self.data_providers.borrow_mut().push(handle);

        let local_object = HashMap::from([
            ("sessionId", self.session_id.into_objc()), // needed for cancel preview
            ("index", (index as i64).into_objc()),      // index in items
        ]);

        // We manage the data source notifier ourselves. Unfortunately the
        // NSItemProvider leaks and never gets released on iOS.
        // So after dragging is finished we manually drop the notifier releasing
        // everything data-source related. The DataProviderSession will be kept
        // alive but it only has weak references to PlatformDataProvider and
        // PlatformDataProviderState.
        let item_provider = provider.create_ns_item_provider(None, Some(self.weak_self.clone()));
        unsafe {
            let drag_item =
                UIDragItem::initWithItemProvider(self.mtm.alloc::<UIDragItem>(), &item_provider);
            drag_item.setLocalObject(Some(&local_object.into_objc()));

            // Setting preview provider here leaks entire session if drag is cancelled before
            // lift is complete. So instead we set it later in `will_begin`. After `will_begin`
            // it is safe to set preview provider as it will be picked immediately and won't leak.
            if self.in_progress.get() {
                self.set_preview_provider(&drag_item);
            }

            drag_item
        }
    }

    fn create_items(
        &self,
        from_index: usize,
        mut providers: HashMap<DataProviderId, DataProviderEntry>,
    ) -> Id<NSArray<UIDragItem>> {
        let mut dragging_items = Vec::<Id<UIDragItem>>::new();

        for (index, item) in self
            .configuration
            .borrow()
            .items
            .iter()
            .enumerate()
            .skip(from_index)
        {
            let provider_entry = providers
                .remove(&item.data_provider_id)
                .expect("Missing provider");
            dragging_items.push(self.create_item(
                provider_entry.provider,
                provider_entry.handle,
                index,
            ));
        }
        NSArray::from_vec(dragging_items)
    }

    fn process_additional_items(
        &self,
        mut items: GetAdditionalItemsResult,
    ) -> Id<NSArray<UIDragItem>> {
        let from_index = {
            let mut configuration = self.configuration.borrow_mut();
            let index = configuration.items.len();
            configuration.items.append(&mut items.items);
            index
        };
        self.create_items(from_index, items.providers)
    }

    fn get_additional_items_for_location(&self, location: Point) -> Id<NSArray<UIDragItem>> {
        if let Some(delegate) = self.context_delegate.upgrade() {
            let items_promise = delegate.get_additional_items_for_location(
                self.context_id,
                self.session_id,
                location,
            );
            let mut poll_session = PollSession::new();
            let _ignore_events = IgnoreInteractionEvents::new();
            loop {
                if let Some(items) = items_promise.try_take() {
                    match items {
                        PromiseResult::Ok { value } => return self.process_additional_items(value),
                        PromiseResult::Cancelled => return unsafe { NSArray::array() },
                    }
                }
                RunLoop::current()
                    .platform_run_loop
                    .poll_once(&mut poll_session);
            }
        } else {
            unsafe { NSArray::array() }
        }
    }

    unsafe fn set_preview_provider(&self, item: &UIDragItem) {
        let preview_provider = item.previewProvider();
        // If lift image is specified now create preview provider for dragging.
        // If this is done when creating items the whole session leaks...
        if preview_provider.is_null() {
            let Some((index, _)) = PlatformDragContext::item_info(item) else {
                return;
            };
            let configuration = self.configuration.borrow();
            let drag_item = &configuration.items[index];
            if drag_item.lift_image.is_none() {
                return;
            }
            let image = self.image_view_for_item(index, ImageType::Drag);
            let shadow_path = bezier_path_for_alpha(&drag_item.image.image_data);
            let mtm = self.mtm;
            let provider = RcBlock::new(move || {
                let parameters =
                    UIDragPreviewParameters::init(mtm.alloc::<UIDragPreviewParameters>());
                parameters.setBackgroundColor(Some(&UIColor::clearColor()));
                parameters.setShadowPath(Some(&shadow_path));
                let preview = UIDragPreview::initWithView_parameters(
                    mtm.alloc::<UIDragPreview>(),
                    &image,
                    &parameters,
                );
                Id::autorelease_return(preview)
            });
            item.setPreviewProvider(Some(&provider));
        }
    }

    fn drag_will_begin(&self, session: &ProtocolObject<dyn UIDragSession>) {
        self.in_progress.replace(true);
        // Only set preview providers when not transitioning from menu.
        // when transitioning for menu the items created during menu being
        // have already drag items images set for preview (instead of lift).
        if !self.menu_active() {
            unsafe {
                // workaround for memory leak, see [create_item].
                let items = session.items();
                for item in items.iter() {
                    self.set_preview_provider(item);
                }
            }
        }
    }

    fn did_move(&self, _session: &ProtocolObject<dyn UIDragSession>, location: Point) {
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.drag_session_did_move_to_location(self.context_id, self.session_id, location);
        }
    }

    fn did_end_with_operation(&self, operation: UIDropOperation) {
        if self.sent_did_end.replace(true) {
            // already cancelled
            return;
        }
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.drag_session_did_end_with_operation(
                self.context_id,
                self.session_id,
                DropOperation::from_platform(operation),
            );
        }
    }

    fn image_view_for_item(&self, index: usize, ty: ImageType) -> Id<UIImageView> {
        self.views
            .borrow_mut()
            .entry((index, ty))
            .or_insert_with(|| unsafe {
                let configuration = self.configuration.borrow();
                let item = &configuration.items[index];
                let drag_image = if ty == ImageType::Drag {
                    &item.image
                } else {
                    item.lift_image.as_ref().unwrap_or(&item.image)
                };

                let image_view = image_view_from_data(drag_image.image_data.clone(), self.mtm);

                let frame: CGRect = drag_image
                    .rect
                    .clone()
                    .translated(-100000.0, -100000.0)
                    .into();

                image_view.setFrame(frame);
                self.view_container.addSubview(&image_view);

                image_view
            })
            .clone()
    }

    fn cancelling(&self) {
        if self.sent_did_end.replace(true) {
            return; // already cancelled
        }
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.drag_session_did_end_with_operation(
                self.context_id,
                self.session_id,
                DropOperation::None,
            );
        }
    }

    fn preview_for_item_type(&self, index: usize, ty: ImageType) -> Id<UITargetedDragPreview> {
        let configuration = self.configuration.borrow();
        let item = &configuration.items[index];
        let drag_image = match ty {
            ImageType::Lift => item.lift_image.as_ref().unwrap_or(&item.image),
            ImageType::Drag => &item.image,
        };
        let image_view = self.image_view_for_item(index, ty);
        unsafe {
            let parameters =
                UIDragPreviewParameters::init(self.mtm.alloc::<UIDragPreviewParameters>());
            parameters.setBackgroundColor(Some(&UIColor::clearColor()));
            let shadow_path = bezier_path_for_alpha(&drag_image.image_data);
            parameters.setShadowPath(Some(&shadow_path));

            let center: CGPoint = drag_image.rect.center().into();
            let target = UIPreviewTarget::initWithContainer_center(
                self.mtm.alloc::<UIPreviewTarget>(),
                &self.view_container,
                center,
            );

            UITargetedDragPreview::initWithView_parameters_target(
                self.mtm.alloc::<UITargetedDragPreview>(),
                &image_view,
                &parameters,
                &target,
            )
        }
    }

    fn menu_active(&self) -> bool {
        if let Some(menu_contexts) = self
            .context_delegate
            .upgrade()
            .map(|d| d.get_platform_menu_contexts())
        {
            return menu_contexts.iter().any(|c| c.menu_active());
        }
        false
    }

    fn preview_for_item(&self, index: usize) -> Id<UITargetedDragPreview> {
        // while menu is active (and we're not dragging yet) create items
        // immediately with drag image instead of lift. This will alleviate
        // the issue of showing lift image for a moment after menu transitions
        // to drag.
        if self.menu_active() && !self.in_progress.get() {
            self.preview_for_item_type(index, ImageType::Drag)
        } else {
            self.preview_for_item_type(index, ImageType::Lift)
        }
    }

    fn preview_for_canceling(&self, index: usize) -> Id<UITargetedDragPreview> {
        let view_container = self.view_container.clone();
        // Fade the container view out. UIKit seems to keep the view
        // visible for way too long after cancellation, which is obvious
        // during scrolling. Ideally we would want updated position here
        // but for now it seems like a bit of an overkill.
        let animation_block = RcBlock::new(move || {
            unsafe { view_container.setAlpha(0.0) };
        });

        unsafe {
            UIView::animateWithDuration_delay_options_animations_completion(
                0.3,
                0.2,
                UIViewAnimationOptions::empty(),
                &animation_block,
                None,
                self.mtm,
            );
        };

        // It takes eternity to get the UIKit cancelled notification;
        // So we do it manually slightly after the animation is done.
        let weak_self = self.weak_self.clone();
        RunLoop::current()
            .schedule(Duration::from_millis(600), move || {
                if let Some(this) = weak_self.upgrade() {
                    this.cancelling();
                }
            })
            .detach();

        self.preview_for_item_type(index, ImageType::Lift)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.in_progress.get() {
            // Session is done without even having started. We still need to inform
            // dart code so that the session state gets properly cleaned-up.
            if let Some(delegate) = self.context_delegate.upgrade() {
                delegate.drag_session_did_end_with_operation(
                    self.context_id,
                    self.session_id,
                    DropOperation::UserCancelled,
                );
            }
        }

        let view_container = self.view_container.clone();
        let animation_block = RcBlock::new(move || {
            unsafe { view_container.setAlpha(0.0) };
        });

        let view_container = self.view_container.clone();
        let completion_block: RcBlock<dyn Fn(_)> = RcBlock::new(move |_| {
            unsafe { view_container.removeFromSuperview() };
        });

        unsafe {
            UIView::animateWithDuration_delay_options_animations_completion(
                0.5,
                0.0,
                UIViewAnimationOptions::empty(),
                &animation_block,
                Some(&completion_block),
                self.mtm,
            );
        };
    }
}

impl DataProviderSessionDelegate for Session {
    fn should_fetch_items(&self) -> bool {
        self.in_progress.get()
    }
}

impl PlatformDragContext {
    pub fn new(
        id: PlatformDragContextId,
        engine_handle: i64,
        delegate: Weak<dyn PlatformDragContextDelegate>,
    ) -> NativeExtensionsResult<Self> {
        let view = EngineContext::get()?.get_flutter_view(engine_handle)?;

        Ok(Self {
            id,
            weak_self: Late::new(),
            view: unsafe { Id::cast(view) },
            delegate,
            interaction: Late::new(),
            interaction_delegate: Late::new(),
            sessions: RefCell::new(HashMap::new()),
            mtm: MainThreadMarker::new().unwrap(),
        })
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());

        let delegate = SNEDragContext::new(weak_self, self.mtm);
        self.interaction_delegate.set(delegate.retain());
        let interaction = unsafe {
            UIDragInteraction::initWithDelegate(
                self.mtm.alloc::<UIDragInteraction>(),
                &Id::cast(delegate),
            )
        };
        unsafe { self.view.addInteraction(&Id::cast(interaction.clone())) };
        self.interaction.set(interaction);
    }

    pub fn needs_combined_drag_image() -> bool {
        false
    }

    pub async fn start_drag(
        &self,
        _request: DragRequest,
        _providers: HashMap<DataProviderId, DataProviderEntry>,
        _session_id: DragSessionId,
    ) -> NativeExtensionsResult<()> {
        Err(NativeExtensionsError::UnsupportedOperation)
    }

    fn _items_for_beginning(
        &self,
        _interaction: &UIDragInteraction,
        drag_session: &ProtocolObject<dyn UIDragSession>,
        data: GetDragConfigurationResult,
    ) -> Id<NSArray<UIDragItem>> {
        let session = Rc::new(Session::new(
            self.delegate.clone(),
            self.view.clone(),
            self.id,
            data.session_id,
            data.configuration,
            self.mtm,
        ));
        let session_id = data.session_id;
        session.assign_weak_self(Rc::downgrade(&session));
        self.sessions
            .borrow_mut()
            .insert(session_id, session.clone());

        // There doesn't seem to be a better way to determine when session is disposed.
        // didEndWithOperation: and didTransferItems: are only called when session began drag,
        // but it is possible for lift to end without user actually dragging, which will
        // cancel the session; In which case we still want to cleanup the session state.
        let weak_self = self.weak_self.clone();
        let drop_notifier = Arc::new(DropNotifier::new(move || {
            if let Some(this) = weak_self.upgrade() {
                this.sessions.borrow_mut().remove(&session_id);
            }
        }));

        let context = HashMap::from([
            ("sessionId", data.session_id.into_objc()),
            ("dropNotifier", drop_notifier.into_objc()),
        ])
        .into_objc();
        unsafe { drag_session.setLocalContext(Some(&context)) };

        session.create_items(0, data.providers)
    }

    fn items_for_beginning(
        &self,
        interaction: &UIDragInteraction,
        session: &ProtocolObject<dyn UIDragSession>,
    ) -> Id<NSArray<UIDragItem>> {
        if let Some(delegate) = self.delegate.upgrade() {
            let location = unsafe { session.locationInView(&self.view) };
            let configuration_promise =
                delegate.get_drag_configuration_for_location(self.id, location.into());
            let mut poll_session = PollSession::new();

            // Make sure that the recognizer doesn't get touch events while the
            // we're pumping inner event loop, otherwise it might fail with an
            // assertion inside _UIDragInteractionDriverStateMachineHandleEvent.
            // This can be reproduced by quickly dragging the gesture while it
            // is waiting for initial items
            let _ignore_events = IgnoreInteractionEvents::new();
            loop {
                if let Some(configuration) = configuration_promise.try_take() {
                    match configuration {
                        PromiseResult::Ok { value } => {
                            return self._items_for_beginning(interaction, session, value);
                        }
                        PromiseResult::Cancelled => return unsafe { NSArray::array() },
                    }
                }
                RunLoop::current()
                    .platform_run_loop
                    .poll_once(&mut poll_session);
            }
        } else {
            unsafe { NSArray::array() }
        }
    }

    fn items_for_adding(
        &self,
        _interaction: &UIDragInteraction,
        session: &ProtocolObject<dyn UIDragSession>,
        point: CGPoint,
    ) -> Id<NSArray<UIDragItem>> {
        if let Some(session) = self.get_session(session) {
            session.get_additional_items_for_location(point.into())
        } else {
            unsafe { NSArray::array() }
        }
    }

    fn get_session_id(session: &ProtocolObject<dyn UIDragSession>) -> Option<DragSessionId> {
        unsafe {
            let context = session.localContext()?;
            let context: Retained<NSObject> = Id::cast(context);
            if context.is_kind_of::<NSDictionary>() {
                let context = Id::cast::<NSDictionary<NSObject, NSObject>>(context);
                let session_id = context.objectForKey(ns_string!("sessionId"))?;
                if session_id.is_kind_of::<NSNumber>() {
                    let session_id = Id::cast::<NSNumber>(session_id);
                    let session_id = session_id.longLongValue();
                    return Some(session_id.into());
                }
            }
        }
        None
    }

    fn get_session(&self, session: &ProtocolObject<dyn UIDragSession>) -> Option<Rc<Session>> {
        Self::get_session_id(session).and_then(|id| self.sessions.borrow().get(&id).cloned())
    }

    fn drag_will_begin(
        &self,
        _interaction: &UIDragInteraction,
        platform_session: &ProtocolObject<dyn UIDragSession>,
    ) {
        if let Some(session) = self.get_session(platform_session) {
            session.drag_will_begin(platform_session);
        }
    }

    fn did_move(
        &self,
        _interaction: &UIDragInteraction,
        platform_session: &ProtocolObject<dyn UIDragSession>,
    ) {
        let location = unsafe { platform_session.locationInView(&self.view) };
        if let Some(session) = self.get_session(platform_session) {
            session.did_move(platform_session, location.into());
        }
    }

    fn did_end_with_operation(
        &self,
        _interaction: &UIDragInteraction,
        session: &ProtocolObject<dyn UIDragSession>,
        operation: UIDropOperation,
    ) {
        if let Some(session) = self.get_session(session) {
            session.did_end_with_operation(operation);
        }
    }

    fn allows_move_operation(
        &self,
        _interaction: &UIDragInteraction,
        session: &ProtocolObject<dyn UIDragSession>,
    ) -> bool {
        if let Some(session) = self.get_session(session) {
            session
                .configuration
                .borrow()
                .allowed_operations
                .contains(&DropOperation::Move)
        } else {
            false
        }
    }

    fn did_transfer_items(
        &self,
        _interaction: &UIDragInteraction,
        _session: &ProtocolObject<dyn UIDragSession>,
    ) {
    }

    fn preview_for_item(
        &self,
        _interaction: &UIDragInteraction,
        item: &UIDragItem,
    ) -> Option<Id<UITargetedDragPreview>> {
        let (index, session_id) = Self::item_info(item)?;
        self.sessions
            .borrow()
            .get(&session_id)
            .cloned()
            .map(|session| session.preview_for_item(index))
    }

    fn preview_for_canceling(
        &self,
        _interaction: &UIDragInteraction,
        item: &UIDragItem,
    ) -> Option<Id<UITargetedDragPreview>> {
        let (index, session_id) = Self::item_info(item)?;
        self.sessions
            .borrow()
            .get(&session_id)
            .cloned()
            .map(|session| session.preview_for_canceling(index))
    }

    fn prefers_full_size_previews(
        &self,
        _interaction: &UIDragInteraction,
        session: &ProtocolObject<dyn UIDragSession>,
    ) -> bool {
        if let Some(session) = self.get_session(session) {
            session.configuration.borrow().prefers_full_size_previews
        } else {
            false
        }
    }

    fn item_info(item: &UIDragItem) -> Option<(usize, DragSessionId)> {
        unsafe {
            let local_object = item.localObject()?;
            let local_object: Retained<NSObject> = Id::cast(local_object);
            if !local_object.is_kind_of::<NSDictionary>() {
                return None;
            }
            let local_object = Id::cast::<NSDictionary<NSObject, NSObject>>(local_object);
            let index = local_object.objectForKey(ns_string!("index"))?;
            let session_id = local_object.objectForKey(ns_string!("sessionId"))?;
            if !index.is_kind_of::<NSNumber>() || !session_id.is_kind_of::<NSNumber>() {
                return None;
            }
            let index = Id::cast::<NSNumber>(index);
            let index = index.longLongValue();
            let session_id = Id::cast::<NSNumber>(session_id);
            let session_id = session_id.longLongValue();
            Some((index as usize, session_id.into()))
        }
    }

    pub fn get_local_data(
        &self,
        session: &ProtocolObject<dyn UIDragSession>,
    ) -> Option<Vec<Value>> {
        self.get_session(session)
            .map(|s| s.configuration.borrow().get_local_data())
    }

    pub fn get_local_data_for_session_id(
        &self,
        id: DragSessionId,
    ) -> NativeExtensionsResult<Vec<Value>> {
        let session = self
            .sessions
            .borrow()
            .get(&id)
            .cloned()
            .ok_or(NativeExtensionsError::DragSessionNotFound)?;
        let data: Vec<_> = session.configuration.borrow().get_local_data();
        Ok(data)
    }
}

impl Drop for PlatformDragContext {
    fn drop(&mut self) {
        unsafe {
            self.view
                .removeInteraction(&Id::cast(self.interaction.clone()));
        }
    }
}

pub struct Inner {
    context: Weak<PlatformDragContext>,
}

impl Inner {
    fn with_state<F, FR, R>(&self, callback: F, default: FR) -> R
    where
        F: FnOnce(Rc<PlatformDragContext>) -> R,
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
    struct SNEDragContext;

    unsafe impl ClassType for SNEDragContext {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "SNEDragContext";
    }

    impl DeclaredClass for SNEDragContext {
        type Ivars = Inner;
    }

    unsafe impl NSObjectProtocol for SNEDragContext {}

    #[allow(non_snake_case)]
    unsafe impl UIDragInteractionDelegate for SNEDragContext {
        #[method_id(dragInteraction:itemsForBeginningSession:)]
        fn dragInteraction_itemsForBeginningSession(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> Id<NSArray<UIDragItem>> {
            self.ivars().with_state(
                |state| state.items_for_beginning(interaction, session),
                || unsafe { NSArray::array() },
            )
        }

        #[method_id(dragInteraction:itemsForAddingToSession:withTouchAtPoint:)]
        fn dragInteraction_itemsForAddingToSession_withTouchAtPoint(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
            point: CGPoint,
        ) -> Id<NSArray<UIDragItem>> {
            self.ivars().with_state(
                |state| state.items_for_adding(interaction, session, point),
                || unsafe { NSArray::array() },
            )
        }

        #[method(dragInteraction:sessionWillBegin:)]
        fn dragInteraction_sessionWillBegin(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) {
            self.ivars()
                .with_state(|state| state.drag_will_begin(interaction, session), || ())
        }

        #[method(dragInteraction:sessionDidMove:)]
        fn dragInteraction_sessionDidMove(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) {
            self.ivars()
                .with_state(|state| state.did_move(interaction, session), || ())
        }

        #[method(dragInteraction:session:didEndWithOperation:)]
        fn dragInteraction_session_didEndWithOperation(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
            operation: UIDropOperation,
        ) {
            self.ivars().with_state(
                |state| state.did_end_with_operation(interaction, session, operation),
                || (),
            )
        }

        #[method(dragInteraction:sessionDidTransferItems:)]
        fn dragInteraction_sessionDidTransferItems(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) {
            self.ivars().with_state(
                |state| state.did_transfer_items(interaction, session),
                || (),
            )
        }

        #[method_id(dragInteraction:previewForLiftingItem:session:)]
        fn dragInteraction_previewForLiftingItem_session(
            &self,
            interaction: &UIDragInteraction,
            item: &UIDragItem,
            _session: &ProtocolObject<dyn UIDragSession>,
        ) -> Option<Id<UITargetedDragPreview>> {
            self.ivars()
                .with_state(|state| state.preview_for_item(interaction, item), || None)
        }

        #[method_id(dragInteraction:previewForCancellingItem:withDefault:)]
        fn dragInteraction_previewForCancellingItem_withDefault(
            &self,
            interaction: &UIDragInteraction,
            item: &UIDragItem,
            _default_preview: &UITargetedDragPreview,
        ) -> Option<Id<UITargetedDragPreview>> {
            self.ivars().with_state(
                |state| state.preview_for_canceling(interaction, item),
                || None,
            )
        }

        #[method(dragInteraction:prefersFullSizePreviewsForSession:)]
        fn dragInteraction_prefersFullSizePreviewsForSession(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> bool {
            self.ivars().with_state(
                |state| state.prefers_full_size_previews(interaction, session),
                || false,
            )
        }

        #[method(dragInteraction:sessionAllowsMoveOperation:)]
        fn dragInteraction_sessionAllowsMoveOperation(
            &self,
            interaction: &UIDragInteraction,
            session: &ProtocolObject<dyn UIDragSession>,
        ) -> bool {
            self.ivars().with_state(
                |state| state.allows_move_operation(interaction, session),
                || false,
            )
        }
    }
);

impl SNEDragContext {
    fn new(context: Weak<PlatformDragContext>, mtm: MainThreadMarker) -> Id<Self> {
        let this = mtm.alloc::<Self>();
        let this = this.set_ivars(Inner { context });
        unsafe { msg_send_id![super(this), init] }
    }
}
