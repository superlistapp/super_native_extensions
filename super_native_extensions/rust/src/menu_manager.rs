use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

use async_trait::async_trait;
use irondash_message_channel::{
    AsyncMethodHandler, AsyncMethodInvoker, IntoValue, IsolateId, Late, MethodCall, PlatformResult,
    RegisteredAsyncMethodHandler, TryFromValue, Value,
};
use irondash_run_loop::spawn;
use log::warn;

use crate::{
    api_model::{
        DeferredMenuResponse, ImageData, MenuConfiguration, MenuElement, Point,
        ShowContextMenuRequest, ShowContextMenuResponse,
    },
    context::Context,
    drag_manager::GetDragManager,
    error::{NativeExtensionsError, NativeExtensionsResult},
    log::{OkLog, OkLogUnexpected},
    platform_impl::platform::{PlatformDragContext, PlatformMenu, PlatformMenuContext},
    util::NextId,
    value_promise::{Promise, PromiseResult},
};

// Each isolate has its own DragContext.
pub type PlatformMenuContextId = IsolateId;

pub trait PlatformMenuContextDelegate {
    fn on_show_menu(&self, context_id: PlatformMenuContextId, menu_configuration_id: i64);

    fn on_hide_menu(
        &self,
        context_id: PlatformMenuContextId,
        menu_configuration_id: i64,
        item_selected: bool,
    );

    fn get_platform_drag_contexts(&self) -> Vec<Rc<PlatformDragContext>>;

    fn on_preview_action(&self, context_id: PlatformMenuContextId, menu_configuration_id: i64);

    fn get_menu_configuration_for_location(
        &self,
        context_id: PlatformMenuContextId,
        location: Point,
    ) -> Arc<Promise<PromiseResult<MenuConfiguration>>>;
}

#[async_trait(?Send)]
pub trait PlatformMenuDelegate {
    fn on_action(&self, isolate_id: IsolateId, action: i64);

    async fn get_deferred_menu(
        &self,
        isolate_id: IsolateId,
        id: i64,
    ) -> NativeExtensionsResult<Vec<MenuElement>>;
}

pub struct MenuManager {
    weak_self: Late<Weak<Self>>,
    invoker: Late<AsyncMethodInvoker>,
    contexts: RefCell<HashMap<PlatformMenuContextId, Rc<PlatformMenuContext>>>,
    next_id: Cell<i64>,
    menus: RefCell<HashMap<i64, Rc<PlatformMenu>>>,
}

pub trait GetMenuManager {
    fn menu_manager(&self) -> Rc<MenuManager>;
}

impl GetMenuManager for Context {
    fn menu_manager(&self) -> Rc<MenuManager> {
        self.get_attachment(MenuManager::new).handler()
    }
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct MenuContextInitRequest {
    engine_handle: i64,
}

#[derive(TryFromValue)]
#[irondash(rename_all = "camelCase")]
struct UpdatePreviewImageRequest {
    configuration_id: i64,
    image: ImageData,
}

impl MenuManager {
    pub fn new() -> RegisteredAsyncMethodHandler<Self> {
        Self {
            weak_self: Late::new(),
            invoker: Late::new(),
            contexts: RefCell::new(HashMap::new()),
            next_id: Cell::new(0),
            menus: RefCell::new(HashMap::new()),
        }
        .register("MenuManager")
    }

    pub fn get_platform_menu_contexts(&self) -> Vec<Rc<PlatformMenuContext>> {
        self.contexts.borrow().values().cloned().collect()
    }

    fn new_context(
        &self,
        isolate: IsolateId,
        request: MenuContextInitRequest,
    ) -> NativeExtensionsResult<()> {
        if self.contexts.borrow().get(&isolate).is_some() {
            // Can happen during hot reload
            warn!("MenuContext already exists for isolate {:?}", isolate);
            return Ok(());
        }
        let context = Rc::new(PlatformMenuContext::new(
            isolate,
            request.engine_handle,
            self.weak_self.clone(),
        )?);
        context.assign_weak_self(Rc::downgrade(&context));
        self.contexts.borrow_mut().insert(isolate, context);
        Ok(())
    }

    async fn register_menu(
        &self,
        menu: MenuElement,
        isolate: IsolateId,
    ) -> NativeExtensionsResult<i64> {
        if let MenuElement::Menu(menu) = menu {
            let platform_menu = PlatformMenu::new(isolate, self.weak_self.clone(), menu)?;
            let id = self.next_id.next_id();
            self.menus.borrow_mut().insert(id, platform_menu);
            Ok(id)
        } else {
            Err(NativeExtensionsError::InvalidMenuElement)
        }
    }

    async fn dispose_menu(&self, id: i64) -> NativeExtensionsResult<()> {
        self.menus.borrow_mut().remove(&id);
        Ok(())
    }

    fn update_preview_image(
        &self,
        request: UpdatePreviewImageRequest,
        isolate_id: IsolateId,
    ) -> NativeExtensionsResult<()> {
        let contexts = self.contexts.borrow();
        let context = contexts.get(&isolate_id);
        if let Some(context) = context {
            context.update_preview_image(request.configuration_id, request.image)?;
        }
        Ok(())
    }

    async fn show_context_menu(
        &self,
        mut menu_request: ShowContextMenuRequest,
        isolate_id: IsolateId,
    ) -> NativeExtensionsResult<ShowContextMenuResponse> {
        let context = self
            .contexts
            .borrow()
            .get(&isolate_id)
            .cloned()
            .ok_or(NativeExtensionsError::PlatformContextNotFound)?;
        let menu = self
            .menus
            .borrow()
            .get(&menu_request.menu_handle)
            .cloned()
            .ok_or(NativeExtensionsError::PlatformMenuNotFound)?;
        menu_request.menu = Some(menu);
        context.show_context_menu(menu_request).await
    }

    async fn get_menu_configuration_for_location(
        &self,
        context_id: PlatformMenuContextId,
        location: Point,
    ) -> NativeExtensionsResult<Option<MenuConfiguration>> {
        #[derive(IntoValue)]
        #[irondash(rename_all = "camelCase")]
        struct MenuConfigurationRequest {
            location: Point,
            configuration_id: i64,
        }
        #[derive(TryFromValue, Debug)]
        #[irondash(rename_all = "camelCase")]
        struct MenuConfigurationResponse {
            configuration: Option<MenuConfiguration>,
        }
        let configuration_id = self.next_id.next_id();

        let configuration: MenuConfigurationResponse = self
            .invoker
            .call_method_cv(
                context_id,
                "getConfigurationForLocation",
                MenuConfigurationRequest {
                    location,
                    configuration_id,
                },
            )
            .await?;
        let configuration = configuration.configuration;
        match configuration {
            Some(mut configuration) => {
                if configuration.configuration_id != configuration_id {
                    return Err(NativeExtensionsError::InvalidMenuConfigurationId);
                }
                let menu = self.menus.borrow().get(&configuration.menu_handle).cloned();
                if let Some(menu) = menu {
                    configuration.menu = Some(menu);
                    Ok(Some(configuration))
                } else {
                    Err(NativeExtensionsError::PlatformMenuNotFound)
                }
            }
            None => Ok(None),
        }
    }
}

#[async_trait(?Send)]
impl PlatformMenuDelegate for MenuManager {
    fn on_action(&self, isolate_id: IsolateId, action: i64) {
        self.invoker
            .call_method_sync(isolate_id, "onAction", action, |r| {
                r.ok_log();
            });
    }

    async fn get_deferred_menu(
        &self,
        isolate_id: IsolateId,
        id: i64,
    ) -> NativeExtensionsResult<Vec<MenuElement>> {
        let response: DeferredMenuResponse = self
            .invoker
            .call_method_cv(isolate_id, "getDeferredMenu", id)
            .await?;
        Ok(response.elements)
    }
}

impl PlatformMenuContextDelegate for MenuManager {
    fn on_show_menu(&self, context_id: PlatformMenuContextId, menu_configuration_id: i64) {
        self.invoker
            .call_method_sync(context_id, "onShowMenu", menu_configuration_id, |r| {
                r.ok_log();
            });
    }

    fn on_hide_menu(
        &self,
        context_id: PlatformMenuContextId,
        menu_configuration_id: i64,
        item_selected: bool,
    ) {
        #[derive(IntoValue)]
        #[irondash(rename_all = "camelCase")]
        struct HideMenuRequest {
            menu_configuration_id: i64,
            item_selected: bool,
        }
        self.invoker.call_method_sync(
            context_id,
            "onHideMenu",
            HideMenuRequest {
                menu_configuration_id,
                item_selected,
            },
            |r| {
                r.ok_log();
            },
        );
    }

    fn on_preview_action(&self, context_id: PlatformMenuContextId, menu_configuration_id: i64) {
        self.invoker
            .call_method_sync(context_id, "onPreviewAction", menu_configuration_id, |r| {
                r.ok_log();
            });
    }

    fn get_platform_drag_contexts(&self) -> Vec<Rc<PlatformDragContext>> {
        Context::get().drag_manager().get_platform_drag_contexts()
    }

    fn get_menu_configuration_for_location(
        &self,
        context_id: PlatformMenuContextId,
        location: Point,
    ) -> Arc<Promise<PromiseResult<MenuConfiguration>>> {
        let res = Arc::new(Promise::new());
        let res_clone = res.clone();
        let weak_self = self.weak_self.clone();
        spawn(async move {
            let this = weak_self.upgrade();
            if let Some(this) = this {
                match this
                    .get_menu_configuration_for_location(context_id, location)
                    .await
                    .ok_log_unexpected()
                    .flatten()
                {
                    Some(data) => {
                        res_clone.set(PromiseResult::Ok { value: data });
                    }
                    None => {
                        res_clone.set(PromiseResult::Cancelled);
                    }
                }
            } else {
                res_clone.set(PromiseResult::Cancelled);
            }
        });
        res
    }
}

#[async_trait(?Send)]
impl AsyncMethodHandler for MenuManager {
    fn assign_weak_self(&self, weak_self: Weak<Self>) {
        self.weak_self.set(weak_self);
    }

    fn assign_invoker(&self, invoker: AsyncMethodInvoker) {
        self.invoker.set(invoker);
    }

    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "newContext" => {
                self.new_context(call.isolate, call.args.try_into()?)?;
                Ok(Value::Null)
            }
            "registerMenu" => {
                let id = self
                    .register_menu(call.args.try_into()?, call.isolate)
                    .await?;
                Ok(id.into())
            }
            "disposeMenu" => {
                self.dispose_menu(call.args.try_into()?).await?;
                Ok(Value::Null)
            }
            "updatePreviewImage" => {
                self.update_preview_image(call.args.try_into()?, call.isolate)?;
                Ok(Value::Null)
            }
            "showContextMenu" => {
                let res = self
                    .show_context_menu(call.args.try_into()?, call.isolate)
                    .await?;
                Ok(res.into())
            }
            _ => Ok(Value::Null),
        }
    }
}
