import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'package:device_info_plus/device_info_plus.dart';

import 'api_model.dart';
import 'drag.dart';
import 'menu.dart';
import 'util.dart';
import 'image.dart';
import 'widgets/gesture/single_drag.dart';
import 'widgets/menu_widget/menu_stack.dart';
import 'widgets/drag_interaction/controller.dart';
import 'widgets/drag_interaction/delayed_drag.dart';
import 'widgets/drag_interaction/menu_preview_widget.dart';

int _nextMenuConfigurationId = 1;

class LongPressHandler {
  static Future<LongPressHandler> create() async {
    return LongPressHandler._(
      dragContext: await DragContext.instance(),
      menuContext: await MenuContext.instance(),
    );
  }

  SingleDrag? dragGestureForPosition({
    required BuildContext context,
    required Offset position,
    required int pointer,
  }) {
    if (DragInteractionSession.active) {
      return null;
    }

    final previewUpdateNotifier = ValueNotifier<ui.Image?>(null);
    final session = _FakeDragSession();
    final dragConfiguration =
        _dragContext.delegate?.getConfigurationForDragRequest(
      location: position,
      session: session,
    );
    final menuConfiguration = _menuContext.delegate?.getMenuConfiguration(
      MenuConfigurationRequest(
        configurationId: _nextMenuConfigurationId++,
        location: position,
        previewImageSetter: (image) {
          previewUpdateNotifier.value = image;
        },
      ),
    );
    if (dragConfiguration == null && menuConfiguration == null) {
      return null;
    }
    return DelayedDrag(_dragForConfigurationFutures(
      dragConfiguration: dragConfiguration,
      menuConfiguration: menuConfiguration,
      dragSession: session,
      pointer: pointer,
      context: context,
      position: position,
      previewUpdateListenable: previewUpdateNotifier,
    ));
  }

  DragItem _findPrimaryItem(DragConfiguration configuration, Offset position) {
    for (final item in configuration.items) {
      final image = item.liftImage ?? item.image;
      if (image.rect.contains(position)) {
        return item;
      }
    }
    return configuration.items.reduce((v1, v2) {
      final image1 = v1.liftImage ?? v1.image;
      final image2 = v2.liftImage ?? v2.image;
      final distance1 = (image1.rect.center - position).distanceSquared;
      final distance2 = (image2.rect.center - position).distanceSquared;
      return distance1 < distance2 ? v1 : v2;
    });
  }

  Future<SingleDrag?> _dragForConfigurationFutures({
    required Future<DragConfiguration?>? dragConfiguration,
    required Future<MenuConfiguration?>? menuConfiguration,
    required _FakeDragSession dragSession,
    required int pointer,
    required BuildContext context,
    required Offset position,
    required ValueListenable<ui.Image?> previewUpdateListenable,
  }) async {
    final drag = await dragConfiguration;
    final menu = await menuConfiguration;
    if (drag == null && menu == null) {
      return null;
    }
    // ignore: use_build_context_synchronously
    if (!context.mounted) {
      return null;
    }
    return _dragForConfiguration(
      dragConfiguration: drag,
      menuConfiguration: menu,
      dragSession: dragSession,
      pointer: pointer,
      context: context,
      position: position,
      previewUpdateListenable: previewUpdateListenable,
    );
  }

  Future<SingleDrag?> _dragForConfiguration({
    required DragConfiguration? dragConfiguration,
    required MenuConfiguration? menuConfiguration,
    required _FakeDragSession dragSession,
    required int pointer,
    required BuildContext context,
    required Offset position,
    required ValueListenable<ui.Image?> previewUpdateListenable,
  }) async {
    final ItemConfiguration primaryItem;
    final List<ItemConfiguration> secondaryItems;
    final DragStartCallback? onDragStart;

    if (dragConfiguration != null && dragConfiguration.items.isNotEmpty) {
      final primary = _findPrimaryItem(dragConfiguration, position);
      primaryItem = ItemConfiguration(
          liftImage: menuConfiguration?.liftImage ??
              primary.liftImage ??
              primary.image,
          dragImage: primary.image.image);
      secondaryItems = dragConfiguration.items
          .where((e) => e != primary)
          .map((e) => ItemConfiguration(
              liftImage: e.liftImage ?? e.image, dragImage: e.image.image))
          .toList(growable: false);

      onDragStart = (
        offset,
        pointer,
        snapshot,
        draggingStarted,
      ) async {
        final realSession = _dragContext.newSession(pointer: pointer);
        dragSession.startDrag(realSession);

        bool started = false;
        void maybeStarted() {
          if (!started) {
            started = true;
            draggingStarted();
          }
        }

        // How many location changed events to ignore before hiding our placeholder.
        // On some Android version (30) the drag avatar is not displayed until the second
        // location change event.
        var ignoreEventCount = 0;

        DeviceInfoPlugin deviceInfoPlugin = DeviceInfoPlugin();
        final deviceInfo = await deviceInfoPlugin.deviceInfo;
        if (deviceInfo is AndroidDeviceInfo) {
          if (deviceInfo.version.sdkInt <= 30) {
            ignoreEventCount = 1;
          }
        }

        final targetedImage = TargetedImage(
            snapshot,
            Rect.fromCenter(
                center: primary.image.rect.center,
                width: snapshot.pointWidth,
                height: snapshot.pointHeight));

        void lastScreenLocationChanged() {
          if (ignoreEventCount == 0) {
            maybeStarted();
            realSession.lastScreenLocation
                .removeListener(lastScreenLocationChanged);
          }
          --ignoreEventCount;
        }

        realSession.lastScreenLocation.addListener(lastScreenLocationChanged);
        realSession.dragCompleted.addListener(maybeStarted);
        _dragContext.startDrag(
            session: realSession,
            configuration: dragConfiguration,
            position: offset,
            combinedDragImage: await targetedImage.intoRaw());
      };
    } else if (menuConfiguration != null) {
      primaryItem = ItemConfiguration(
        liftImage: menuConfiguration.liftImage,
      );
      secondaryItems = [];
      onDragStart = null;
    } else {
      return null;
    }

    final MenuPreviewWidget? menuPreviewWidget;

    if (menuConfiguration != null) {
      if (menuConfiguration.previewSize != null) {
        // deferred size
        menuPreviewWidget = MenuPreviewWidget(
          size: menuConfiguration.previewSize!,
          menuWidgetBuilder: menuConfiguration.menuWidgetBuilder,
        );
      } else if (menuConfiguration.previewImage != null) {
        menuPreviewWidget = MenuPreviewWidget(
          image: menuConfiguration.previewImage,
          size: menuConfiguration.previewImage!.pointSize,
          menuWidgetBuilder: menuConfiguration.menuWidgetBuilder,
        );
      } else {
        menuPreviewWidget = MenuPreviewWidget(
          image: menuConfiguration.liftImage.image,
          size: menuConfiguration.liftImage.image.pointSize,
          menuWidgetBuilder: menuConfiguration.menuWidgetBuilder,
        );
      }
    } else {
      menuPreviewWidget = null;
    }

    final menuPreview = ValueNotifier(menuPreviewWidget);

    previewUpdateListenable.addListener(() {
      final image = previewUpdateListenable.value;
      if (image != null) {
        menuPreview.value = MenuPreviewWidget(
          image: image,
          size: image.pointSize,
          menuWidgetBuilder: menuConfiguration!.menuWidgetBuilder,
        );
      }
    });

    final interactionConfiguration = DragInteractionConfiguration(
      primaryItem: primaryItem,
      secondaryItems: secondaryItems,
      menuHandle: menuConfiguration?.handle,
      menuBuilder: menuConfiguration != null
          ? (context, menuDelegate, alignment, canScrollListenable) {
              return MenuStack(
                rootMenu: menuConfiguration.handle.menu,
                menuAlignment: alignment,
                delegate: menuDelegate,
                canScrollListenable: canScrollListenable,
                builder: menuConfiguration.menuWidgetBuilder,
                iconTheme: menuConfiguration.iconTheme,
              );
            }
          : null,
      menuPreview: menuPreview,
      menuWidgetBuilder: menuConfiguration?.menuWidgetBuilder,
      onBeginTransitionToDrag: () {
        dragSession.beginDragging();
      },
      onDidNotFinishTransitionToDrag: () {
        dragSession.cancelDragging();
      },
      onDragStart: onDragStart,
      onMenuShown: () {
        if (menuConfiguration != null) {
          _menuContext.delegate?.onShowMenu(menuConfiguration.configurationId);
        }
      },
      onMenuHidden: () {
        if (menuConfiguration != null) {
          _menuContext.delegate?.onHideMenu(menuConfiguration.configurationId);
        }
      },
      onMenuPreviewTapped: () {
        if (menuConfiguration != null) {
          _menuContext.delegate?.onPreviewAction(
            menuConfiguration.configurationId,
          );
        }
      },
    );
    // Last chace to bail. There might be a drag session already active started
    // by a different gesture recognizer.
    if (DragInteractionSession.active) {
      return null;
    }
    return DragInteractionSession(
      buildContext: context,
      configuration: interactionConfiguration,
    ).intoDrag(position, pointer);
  }

  final DragContext _dragContext;
  final MenuContext _menuContext;

  LongPressHandler._({
    required DragContext dragContext,
    required MenuContext menuContext,
  })  : _dragContext = dragContext,
        _menuContext = menuContext;
}

class _FakeDragSession extends DragSession {
  _FakeDragSession();

  DragSession? original;

  @override
  final dragCompleted = ValueNotifier<DropOperation?>(null);

  @override
  final dragStarted = SimpleNotifier();

  @override
  final lastScreenLocation = ValueNotifier<Offset?>(null);

  void startDrag(DragSession original) {
    this.original = original;
    original.dragCompleted.addListener(() {
      (dragCompleted as ValueNotifier).value = original.dragCompleted.value;
    });
    original.lastScreenLocation.addListener(() {
      (lastScreenLocation as ValueNotifier).value =
          original.lastScreenLocation.value;
    });
  }

  void beginDragging() {
    _dragging = true;
    (dragStarted as SimpleNotifier).notify();
  }

  void cancelDragging() {
    _dragging = false;
    (dragCompleted as ValueNotifier).value = DropOperation.none;
  }

  bool _dragging = false;

  @override
  bool get dragging => _dragging;

  @override
  Future<List<Object?>?> getLocalData() {
    return original?.getLocalData() ?? Future.value(null);
  }
}
