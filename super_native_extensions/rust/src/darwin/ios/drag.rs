use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem::ManuallyDrop,
    os::raw::c_void,
    rc::{Rc, Weak},
    sync::Arc,
};

use block::ConcreteBlock;
use cocoa::{
    base::{id, nil, BOOL, NO, YES},
    foundation::{NSArray, NSUInteger},
};
use core_graphics::{
    base::CGFloat,
    geometry::{CGPoint, CGRect},
};

use nativeshell_core::{util::Late, Context, Value};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::{autoreleasepool, StrongPtr},
    runtime::{Class, Object, Protocol, Sel},
    sel, sel_impl, Encode, Encoding,
};
use once_cell::sync::Lazy;

use crate::{
    api_model::{DataProviderId, DragConfiguration, DragRequest, DropOperation, Point},
    data_provider_manager::DataProviderHandle,
    drag_manager::{
        DataProviderEntry, DragSessionId, GetAdditionalItemsResult, GetDragConfigurationResult,
        PlatformDragContextDelegate,
    },
    error::{NativeExtensionsError, NativeExtensionsResult},
    platform_impl::platform::common::{superclass, to_nsstring},
    util::DropNotifier,
    value_promise::PromiseResult,
};

use super::{
    drag_common::{DropOperationExt, UIDropOperation},
    util::{image_view_from_data, IntoObjc},
    DataProviderSessionDelegate, PlatformDataProvider,
};

pub struct PlatformDragContext {
    id: i64,
    weak_self: Late<Weak<Self>>,
    view: StrongPtr,
    delegate: Weak<dyn PlatformDragContextDelegate>,
    interaction: Late<StrongPtr>,
    interaction_delegate: Late<StrongPtr>,
    sessions: RefCell<HashMap<DragSessionId, Rc<Session>>>,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
enum ImageType {
    Lift,
    Drag,
}

struct Session {
    context_id: i64,
    context_delegate: Weak<dyn PlatformDragContextDelegate>,
    view_container: StrongPtr,
    session_id: DragSessionId,
    weak_self: Late<Weak<Self>>,
    in_progress: Cell<bool>,
    configuration: RefCell<DragConfiguration>,
    data_providers: RefCell<Vec<Arc<DataProviderHandle>>>,
    views: RefCell<HashMap<(usize, ImageType), StrongPtr>>, // index -> view
}

impl Session {
    fn new(
        context_delegate: Weak<dyn PlatformDragContextDelegate>,
        context_view: StrongPtr,
        platform_drag_context_id: i64,
        session_id: DragSessionId,
        configuration: DragConfiguration,
    ) -> Self {
        let view_container = unsafe {
            let bounds: CGRect = msg_send![*context_view, bounds];
            let container: id = msg_send![class!(UIView), alloc];
            let container = StrongPtr::new(msg_send![container, initWithFrame: bounds]);
            let () = msg_send![*container, setUserInteractionEnabled: NO];
            let () = msg_send![*context_view, addSubview: *container];
            container
        };
        Self {
            context_delegate,
            view_container,
            context_id: platform_drag_context_id,
            weak_self: Late::new(),
            in_progress: Cell::new(false),
            session_id,
            configuration: RefCell::new(configuration),
            data_providers: RefCell::new(Vec::new()),
            views: RefCell::new(HashMap::new()),
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
    ) -> id {
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
            let drag_item: id = msg_send![class!(UIDragItem), alloc];
            let drag_item: id = msg_send![drag_item, initWithItemProvider: item_provider];
            let drag_item: id = msg_send![drag_item, autorelease];
            let () = msg_send![drag_item, setLocalObject: local_object.into_objc().autorelease()];
            drag_item
        }
    }

    fn create_items(
        &self,
        from_index: usize,
        mut providers: HashMap<DataProviderId, DataProviderEntry>,
    ) -> id {
        let mut dragging_items = Vec::<id>::new();

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
        unsafe { NSArray::arrayWithObjects(nil, &dragging_items) }
    }

    fn process_additional_items(&self, mut items: GetAdditionalItemsResult) -> id {
        let from_index = {
            let mut configuration = self.configuration.borrow_mut();
            let index = configuration.items.len();
            configuration.items.append(&mut items.items);
            index
        };
        self.create_items(from_index, items.providers)
    }

    fn get_additional_items_for_location(&self, location: Point) -> id {
        if let Some(delegate) = self.context_delegate.upgrade() {
            let items_promise = delegate.get_additional_items_for_location(
                self.context_id,
                self.session_id,
                location,
            );
            loop {
                if let Some(items) = items_promise.try_take() {
                    match items {
                        PromiseResult::Ok { value } => return self.process_additional_items(value),
                        PromiseResult::Cancelled => return nil,
                    }
                }
                Context::get().run_loop().platform_run_loop.poll_once();
            }
        } else {
            nil
        }
    }

    fn drag_will_begin(&self) {
        self.in_progress.replace(true);
    }

    fn did_move(&self, session: id, location: Point) {
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.drag_session_did_move_to_location(self.context_id, self.session_id, location);
        }
        unsafe {
            let items: id = msg_send![session, items];
            for i in 0..NSArray::count(items) {
                let item = NSArray::objectAtIndex(items, i);
                let preview_provider: id = msg_send![item, previewProvider];
                // If lift image is specified now create preview provider for dragging.
                // If this is done when creating items the whole session leaks...
                if preview_provider.is_null()
                    && self.configuration.borrow().items[i as usize]
                        .lift_image
                        .is_some()
                {
                    let (index, _) = PlatformDragContext::item_info(item);
                    let image = self.image_view_for_item(index, ImageType::Drag);
                    let provider = ConcreteBlock::new(move || {
                        let image = image.clone().autorelease();
                        let preview: id = msg_send![class!(UIDragPreview), alloc];
                        let () = msg_send![preview, initWithView: image];
                        let () = msg_send![preview, autorelease];
                        preview
                    });
                    let provider = provider.copy();

                    let () = msg_send![item, setPreviewProvider: &*provider];
                }
            }
        }
    }

    fn did_end_with_operation(&self, operation: UIDropOperation) {
        if let Some(delegate) = self.context_delegate.upgrade() {
            delegate.drag_session_did_end_with_operation(
                self.context_id,
                self.session_id,
                DropOperation::from_platform(operation),
            );
        }
    }

    fn image_view_for_item(&self, index: usize, ty: ImageType) -> StrongPtr {
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
                let image_view = image_view_from_data(drag_image.image_data.clone());

                let () = msg_send![*self.view_container, addSubview:*image_view];

                let frame: CGRect = drag_image
                    .source_rect
                    .clone()
                    .translated(-100000.0, -100000.0)
                    .into();
                let () = msg_send![*image_view, setFrame: frame];

                image_view
            })
            .clone()
    }

    fn preview_for_item(&self, index: usize) -> id {
        let configuration = self.configuration.borrow();
        let drag_image = &configuration.items[index].image;
        let image_view = self.image_view_for_item(index, ImageType::Lift);
        unsafe {
            let parameters: id = msg_send![class!(UIDragPreviewParameters), new];
            let () = msg_send![parameters, autorelease];

            let target: id = msg_send![class!(UIPreviewTarget), alloc];
            let center: CGPoint = drag_image.source_rect.center().into();
            let () = msg_send![target, initWithContainer:*self.view_container center:center];
            let () = msg_send![target, autorelease];

            let preview: id = msg_send![class!(UITargetedDragPreview), alloc];
            let () =
                msg_send![preview, initWithView:*image_view parameters:parameters target:target];
            let () = msg_send![preview, autorelease];
            preview
        }
    }

    fn preview_for_canceling(&self, index: usize) -> id {
        let view_container = self.view_container.clone();
        // Fade the container view out. UIKit seems to keep the view
        // visible for way too long after cancelation, which is obvious
        // during scrolling. Ideally we would want updated position here
        // but for now it seems like a bit of an overkill.
        let animation_block = ConcreteBlock::new(move || {
            let () = unsafe { msg_send![*view_container, setAlpha: 0.0] };
        });
        let animation_block = animation_block.copy();
        unsafe {
            let () = msg_send![class!(UIView),
                         animateWithDuration: 0.3f64
                         delay: 0.2f64
                         options: 0 as NSUInteger
                         animations:&*animation_block
                         completion:nil];
        };
        self.preview_for_item(index)
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
        let animation_block = ConcreteBlock::new(move || {
            let () = unsafe { msg_send![*view_container, setAlpha: 0.0] };
        });
        let animation_block = animation_block.copy();

        let view_container = self.view_container.clone();
        let completion_block = ConcreteBlock::new(move || {
            let () = unsafe { msg_send![*view_container, removeFromSuperview] };
        });
        let completion_block = completion_block.copy();

        unsafe {
            let () = msg_send![class!(UIView),
                         animateWithDuration: 0.5f64
                         delay: 0.0f64
                         options: 0 as NSUInteger
                         animations:&*animation_block
                         completion:&*completion_block];
        };
    }
}

impl DataProviderSessionDelegate for Session {
    fn should_fetch_items(&self) -> bool {
        self.in_progress.get()
    }
}

impl PlatformDragContext {
    pub fn new(id: i64, view_handle: i64, delegate: Weak<dyn PlatformDragContextDelegate>) -> Self {
        Self {
            id,
            weak_self: Late::new(),
            view: unsafe { StrongPtr::retain(view_handle as *mut _) },
            delegate,
            interaction: Late::new(),
            interaction_delegate: Late::new(),
            sessions: RefCell::new(HashMap::new()),
        }
    }

    pub fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self.clone());
        autoreleasepool(|| unsafe {
            let delegate: id = msg_send![*DELEGATE_CLASS, new];
            (*delegate).set_ivar("context", Weak::into_raw(weak_self) as *mut c_void);
            self.interaction_delegate.set(StrongPtr::new(delegate));
            let interaction: id = msg_send![class!(UIDragInteraction), alloc];
            let interaction: id = msg_send![interaction, initWithDelegate: delegate];
            self.interaction.set(StrongPtr::new(interaction));
            let () = msg_send![*self.view, addInteraction: interaction];
        });
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
        _interaction: id,
        drag_session: id,
        data: GetDragConfigurationResult,
    ) -> id {
        let session = Rc::new(Session::new(
            self.delegate.clone(),
            self.view.clone(),
            self.id,
            data.session_id,
            data.configuration,
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
        // Also note that there is a memory leak - if items have previewProvider set at the
        // beginning the session will never get disposed :-/
        // Setting previewProviders during dragging seems to work.
        let weak_self = self.weak_self.clone();
        let drop_notifier = Arc::new(DropNotifier::new(move || {
            if let Some(this) = weak_self.upgrade() {
                this.sessions.borrow_mut().remove(&session_id);
            }
        }));
        unsafe {
            let context = HashMap::from([
                ("sessionId", data.session_id.into_objc()),
                ("dropNotifier", drop_notifier.into_objc()),
            ])
            .into_objc();
            let () = msg_send![drag_session, setLocalContext: *context];
        }

        session.create_items(0, data.providers)
    }

    fn items_for_beginning(&self, interaction: id, session: id) -> id {
        if let Some(delegate) = self.delegate.upgrade() {
            let location: CGPoint = unsafe { msg_send![session, locationInView:*self.view] };
            let configuration_promise =
                delegate.get_drag_configuration_for_location(self.id, location.into());
            loop {
                if let Some(configuration) = configuration_promise.try_take() {
                    match configuration {
                        PromiseResult::Ok { value } => {
                            return self._items_for_beginning(interaction, session, value);
                        }
                        PromiseResult::Cancelled => return nil,
                    }
                }
                Context::get().run_loop().platform_run_loop.poll_once();
            }
        } else {
            nil
        }
    }

    fn items_for_adding(&self, _interaction: id, session: id, point: CGPoint) -> id {
        if let Some(session) = self.get_session(session) {
            session.get_additional_items_for_location(point.into())
        } else {
            nil
        }
    }

    fn get_session_id(session: id) -> Option<DragSessionId> {
        unsafe {
            let context: id = msg_send![session, localContext];
            let is_dictionary: BOOL = msg_send![context, isKindOfClass: class!(NSDictionary)];
            if is_dictionary == YES {
                let session_id: id = msg_send![context, objectForKey: *to_nsstring("sessionId")];
                let is_number: BOOL = msg_send![session_id, isKindOfClass: class!(NSNumber)];
                if is_number == YES {
                    let session_id: i64 = msg_send![session_id, longLongValue];
                    return Some(session_id.into());
                }
            }
        }
        None
    }

    fn get_session(&self, session: id) -> Option<Rc<Session>> {
        Self::get_session_id(session).and_then(|id| self.sessions.borrow().get(&id).cloned())
    }

    fn drag_will_begin(&self, _interaction: id, session: id) {
        if let Some(session) = self.get_session(session) {
            session.drag_will_begin();
        }
    }

    fn did_move(&self, _interaction: id, platform_session: id) {
        let location: CGPoint = unsafe { msg_send![platform_session, locationInView:*self.view] };
        if let Some(session) = self.get_session(platform_session) {
            session.did_move(platform_session, location.into());
        }
    }

    fn did_end_with_operation(&self, _interaction: id, session: id, operation: UIDropOperation) {
        if let Some(session) = self.get_session(session) {
            session.did_end_with_operation(operation);
        }
    }

    fn allows_move_operation(&self, _interaction: id, session: id) -> bool {
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

    fn did_transfer_items(&self, _interaction: id, _session: id) {}

    fn preview_for_item(&self, _interaction: id, item: id) -> id {
        let info = Self::item_info(item);
        if let Some(session) = self.sessions.borrow().get(&info.1).cloned() {
            session.preview_for_item(info.0)
        } else {
            nil
        }
    }

    fn preview_for_canceling(&self, _interaction: id, item: id) -> id {
        let info = Self::item_info(item);
        if let Some(session) = self.sessions.borrow().get(&info.1).cloned() {
            session.preview_for_canceling(info.0)
        } else {
            nil
        }
    }

    fn prefers_full_size_previews(&self, _interaction: id, session: id) -> BOOL {
        if let Some(session) = self.get_session(session) {
            if session.configuration.borrow().prefers_full_size_previews {
                YES
            } else {
                NO
            }
        } else {
            NO
        }
    }

    fn item_info(item: id) -> (usize, DragSessionId) {
        unsafe {
            let local_object: id = msg_send![item, localObject];
            let index: id = msg_send![local_object, objectForKey: *to_nsstring("index")];
            let index: u64 = msg_send![index, longLongValue];
            let session_id: id = msg_send![local_object, objectForKey: *to_nsstring("sessionId")];
            let session_id: i64 = msg_send![session_id, longLongValue];
            (index as usize, session_id.into())
        }
    }

    pub fn get_local_data(&self, session: id) -> Vec<Value> {
        if let Some(session) = self.get_session(session) {
            session.configuration.borrow().get_local_data()
        } else {
            Vec::new()
        }
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
            let () = msg_send![*self.view, removeInteraction: **self.interaction];
        }
    }
}

fn with_state<F, FR, R>(this: id, callback: F, default: FR) -> R
where
    F: FnOnce(Rc<PlatformDragContext>) -> R,
    FR: FnOnce() -> R,
{
    unsafe {
        let context_ptr = {
            let context_ptr: *mut c_void = *(*this).get_ivar("context");
            context_ptr as *const PlatformDragContext
        };
        let this = ManuallyDrop::new(Weak::from_raw(context_ptr));
        let this = this.upgrade();
        match this {
            Some(this) => callback(this),
            None => default(),
        }
    }
}

extern "C" fn dealloc(this: &Object, _sel: Sel) {
    unsafe {
        let context_ptr = {
            let context_ptr: *mut c_void = *this.get_ivar("context");
            context_ptr as *const PlatformDragContext
        };
        Weak::from_raw(context_ptr);

        let superclass = superclass(this);
        let () = msg_send![super(this, superclass), dealloc];
    }
}

extern "C" fn items_for_beginning(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    session: id,
) -> id {
    with_state(
        this,
        |state| state.items_for_beginning(interaction, session),
        || nil,
    )
}

extern "C" fn items_for_adding(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    session: id,
    point: _CGPoint,
) -> id {
    with_state(
        this,
        |state| state.items_for_adding(interaction, session, point.into()),
        || nil,
    )
}

extern "C" fn drag_will_begin(this: &mut Object, _sel: Sel, interaction: id, session: id) {
    with_state(
        this,
        |state| state.drag_will_begin(interaction, session),
        || (),
    )
}

extern "C" fn did_move(this: &mut Object, _sel: Sel, interaction: id, session: id) {
    with_state(this, |state| state.did_move(interaction, session), || ())
}

extern "C" fn did_end_with_operation(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    session: id,
    operation: UIDropOperation,
) {
    with_state(
        this,
        |state| state.did_end_with_operation(interaction, session, operation),
        || {},
    );
}

extern "C" fn allows_move_operation(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    session: id,
) -> BOOL {
    with_state(
        this,
        |state| {
            if state.allows_move_operation(interaction, session) {
                YES
            } else {
                NO
            }
        },
        || NO,
    )
}

extern "C" fn did_transfer_items(this: &mut Object, _sel: Sel, interaction: id, session: id) {
    with_state(
        this,
        |state| state.did_transfer_items(interaction, session),
        || {},
    );
}

extern "C" fn preview_for_for_lifting_item(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    item: id,
    _session: id,
) -> id {
    with_state(
        this,
        |state| state.preview_for_item(interaction, item),
        || nil,
    )
}

extern "C" fn preview_for_cancelling_item(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    item: id,
    _default: id,
) -> id {
    with_state(
        this,
        |state| state.preview_for_canceling(interaction, item),
        || nil,
    )
}

extern "C" fn prefers_full_size_previews(
    this: &mut Object,
    _sel: Sel,
    interaction: id,
    session: id,
) -> BOOL {
    with_state(
        this,
        |state| state.prefers_full_size_previews(interaction, session),
        || NO,
    )
}

// CGPoint doesn't seem to have encoding defined so we do it ourselves
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct _CGPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}

impl From<_CGPoint> for CGPoint {
    fn from(p: _CGPoint) -> Self {
        CGPoint { x: p.x, y: p.y }
    }
}

unsafe impl Encode for _CGPoint {
    fn encode() -> Encoding {
        unsafe { Encoding::from_str("{CGPoint=dd}") }
    }
}

static DELEGATE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SNEDragInteractionDelegate", superclass).unwrap();
    decl.add_protocol(Protocol::get("UIDragInteractionDelegate").unwrap());
    decl.add_ivar::<*mut c_void>("context");
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    decl.add_method(
        sel!(dragInteraction:itemsForBeginningSession:),
        items_for_beginning as extern "C" fn(&mut Object, Sel, id, id) -> id,
    );

    decl.add_method(
        sel!(dragInteraction:itemsForAddingToSession:withTouchAtPoint:),
        items_for_adding as extern "C" fn(&mut Object, Sel, id, id, _CGPoint) -> id,
    );
    decl.add_method(
        sel!(dragInteraction:sessionWillBegin:),
        drag_will_begin as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.add_method(
        sel!(dragInteraction:sessionDidMove:),
        did_move as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.add_method(
        sel!(dragInteraction:session:didEndWithOperation:),
        did_end_with_operation as extern "C" fn(&mut Object, Sel, id, id, UIDropOperation),
    );
    decl.add_method(
        sel!(dragInteraction:sessionAllowsMoveOperation:),
        allows_move_operation as extern "C" fn(&mut Object, Sel, id, id) -> BOOL,
    );
    decl.add_method(
        sel!(dragInteraction:sessionDidTransferItems:),
        did_transfer_items as extern "C" fn(&mut Object, Sel, id, id),
    );
    decl.add_method(
        sel!(dragInteraction:previewForLiftingItem:session:),
        preview_for_for_lifting_item as extern "C" fn(&mut Object, Sel, id, id, id) -> id,
    );
    decl.add_method(
        sel!(dragInteraction:previewForCancellingItem:withDefault:),
        preview_for_cancelling_item as extern "C" fn(&mut Object, Sel, id, id, id) -> id,
    );
    decl.add_method(
        sel!(dragInteraction:prefersFullSizePreviewsForSession:),
        prefers_full_size_previews as extern "C" fn(&mut Object, Sel, id, id) -> BOOL,
    );
    decl.register()
});
