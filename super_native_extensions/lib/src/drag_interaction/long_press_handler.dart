import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'package:device_info_plus/device_info_plus.dart';

import '../drag.dart';
import '../drop.dart';
import '../menu.dart';
import '../widget_snapshot/widget_snapshot.dart';
import '../gesture/single_drag.dart';

import 'long_press_session.dart';
import 'interaction_session.dart';
import 'delayed_drag.dart';

int _nextMenuConfigurationId = 1;

class MenuPreviewWidget {
  final Widget widget;
  final Size size;

  MenuPreviewWidget({
    required this.widget,
    required this.size,
  });
}

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
    if (LongPressSession.active) {
      return null;
    }
    return LongPressSession.run(() {
      final previewUpdateNotifier = ValueNotifier<WidgetSnapshot?>(null);
      final session = _FakeDragSession();
      LongPressSession.onCleanup(session._dispose);

      final dragConfiguration =
          _dragContext.delegate?.getConfigurationForDragRequest(
        location: position,
        session: session,
      );
      final menuConfiguration = _menuContext.delegate?.getMenuConfiguration(
        MobileMenuConfigurationRequest(
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
      final dragFuture = _dragForConfigurationFutures(
        dragConfiguration: dragConfiguration,
        menuConfiguration: menuConfiguration,
        dragSession: session,
        pointer: pointer,
        context: context,
        position: position,
        previewUpdateListenable: previewUpdateNotifier,
      );
      return DelayedDrag(LongPressSession.extend(() => dragFuture));
    });
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
    required Future<MobileMenuConfiguration?>? menuConfiguration,
    required _FakeDragSession dragSession,
    required int pointer,
    required BuildContext context,
    required Offset position,
    required ValueListenable<WidgetSnapshot?> previewUpdateListenable,
  }) async {
    final drag = await dragConfiguration;
    final menu = await menuConfiguration;
    if (drag == null && menu == null) {
      return null;
    }
    return LongPressSession.extend(() async {
      LongPressSession.onCleanup(() {
        drag?.disposeImages();
        menu?.dispose();
      });
      if (!context.mounted) {
        return null;
      }
      return await _dragForConfiguration(
        dragConfiguration: drag,
        menuConfiguration: menu,
        dragSession: dragSession,
        pointer: pointer,
        context: context,
        position: position,
        previewUpdateListenable: previewUpdateListenable,
      );
    });
  }

  Future<SingleDrag?> _dragForConfiguration({
    required DragConfiguration? dragConfiguration,
    required MobileMenuConfiguration? menuConfiguration,
    required _FakeDragSession dragSession,
    required int pointer,
    required BuildContext context,
    required Offset position,
    required ValueListenable<WidgetSnapshot?> previewUpdateListenable,
  }) async {
    final ItemConfiguration primaryItem;
    final List<ItemConfiguration> secondaryItems;
    final DragStartCallback? onDragStart;

    bool successfullyTransitionedToDrag = false;

    DeviceInfoPlugin deviceInfoPlugin = DeviceInfoPlugin();
    final deviceInfo = await deviceInfoPlugin.deviceInfo;

    // ignore: use_build_context_synchronously
    if (!context.mounted) {
      return null;
    }

    LongPressSession.onCleanup(() {
      if (!successfullyTransitionedToDrag && dragConfiguration != null) {
        for (final item in dragConfiguration.items) {
          item.dataProvider.dispose();
        }
      }
    });

    if (dragConfiguration != null && dragConfiguration.items.isNotEmpty) {
      final primary = _findPrimaryItem(dragConfiguration, position);
      primaryItem = ItemConfiguration(
          liftImage: menuConfiguration?.liftImage ??
              primary.liftImage ??
              primary.image.retain(),
          dragImage: primary.image.snapshot);
      secondaryItems = dragConfiguration.items
          .where((e) => e != primary)
          .map((e) => ItemConfiguration(
                liftImage: e.liftImage ?? e.image.retain(),
                dragImage: e.image.snapshot,
              ))
          .toList(growable: false);

      onDragStart = (
        offset,
        pointer,
        snapshot,
        draggingStarted,
      ) {
        final realSession = _dragContext.newSession(pointer: pointer);
        dragSession.startDrag(realSession);

        final dragCompleter = Completer();

        void maybeStarted() {
          if (!successfullyTransitionedToDrag) {
            successfullyTransitionedToDrag = true;
            draggingStarted();
            if (realSession.dragCompleted.value == null) {
              realSession.dragCompleted.addListener(() {
                dragCompleter.complete();
              });
              LongPressSession.extend(() => dragCompleter.future);
            }
          }
        }

        // How many location changed events to ignore before hiding our placeholder.
        // On some Android version (30) the drag avatar is not displayed until the second
        // location change event.
        var ignoreEventCount = 0;

        if (deviceInfo is AndroidDeviceInfo) {
          if (deviceInfo.version.sdkInt <= 30) {
            ignoreEventCount = 1;
          }
        }

        final targetedImage = TargetedWidgetSnapshot(
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
        if (context.mounted) {
          LongPressSession.extend(
            () => _dragContext.startDrag(
                buildContext: context,
                session: realSession,
                configuration: dragConfiguration,
                position: offset,
                combinedDragImage: targetedImage),
          );
        }
      };
    } else if (menuConfiguration != null) {
      primaryItem = ItemConfiguration(
        liftImage: menuConfiguration.liftImage,
        dragImage: menuConfiguration.liftImage.snapshot.retain(),
      );
      secondaryItems = [];
      onDragStart = null;
    } else {
      return null;
    }

    final finisherCompleter = Completer();
    LongPressSession.extend(() => finisherCompleter.future);

    final DragInteractionMenuConfiguration? interactionMenuConfiguration;
    if (menuConfiguration != null) {
      MenuPreviewWidget createMenuPreview(Size size, WidgetSnapshot? snapshot) {
        return MenuPreviewWidget(
          size: size,
          widget: menuConfiguration.previewBuilder(size, snapshot),
        );
      }

      final MenuPreviewWidget menuPreviewWidget;
      final bool hasCustomMenuPreview;

      if (menuConfiguration.previewSize != null) {
        // deferred preview
        menuPreviewWidget =
            createMenuPreview(menuConfiguration.previewSize!, null);
        hasCustomMenuPreview = true;
      } else if (menuConfiguration.previewImage != null) {
        menuPreviewWidget = createMenuPreview(
            menuConfiguration.previewImage!.pointSize,
            menuConfiguration.previewImage);
        hasCustomMenuPreview = true;
      } else {
        menuPreviewWidget = createMenuPreview(
            menuConfiguration.liftImage.snapshot.pointSize,
            menuConfiguration.liftImage.snapshot);
        hasCustomMenuPreview = false;
      }

      final menuPreview = ValueNotifier(menuPreviewWidget);

      void previewUpdateListener() {
        final image = previewUpdateListenable.value;
        if (image != null) {
          LongPressSession.onCleanup(() {
            image.dispose();
          });
          menuPreview.value = createMenuPreview(
            image.pointSize,
            image,
          );
        }
      }

      previewUpdateListenable.addListener(previewUpdateListener);
      LongPressSession.onCleanup(() {
        previewUpdateListenable.removeListener(previewUpdateListener);
      });

      interactionMenuConfiguration = DragInteractionMenuConfiguration(
        hasCustomMenuPreview: hasCustomMenuPreview,
        menuHandle: menuConfiguration.handle,
        menuWidgetBuilder: menuConfiguration.menuWidgetBuilder,
        menuPreview: menuPreview,
        backgroundBuilder: menuConfiguration.backgroundBuilder,
        iconTheme: menuConfiguration.iconTheme,
        onMenuShown: () {
          _menuContext.delegate?.onShowMenu(menuConfiguration.configurationId);
        },
        onMenuHidden: (response) {
          _menuContext.delegate
              ?.onHideMenu(menuConfiguration.configurationId, response);
        },
        onMenuPreviewTapped: () {
          _menuContext.delegate?.onPreviewAction(
            menuConfiguration.configurationId,
          );
        },
      );
    } else {
      interactionMenuConfiguration = null;
    }

    final interactionConfiguration = DragInteractionConfiguration(
      menuConfiguration: interactionMenuConfiguration,
      primaryItem: primaryItem,
      secondaryItems: secondaryItems,
      onBeginTransitionToDrag: () {
        dragSession.beginDragging();
      },
      onDidNotFinishTransitionToDrag: () {
        dragSession.cancelDragging();
      },
      onDragStart: onDragStart,
      onFinished: () {
        finisherCompleter.complete();
      },
    );
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

  final _dragCompleted = ValueNotifier<DropOperation?>(null);
  final _dragging = ValueNotifier<bool>(false);
  final _lastScreenLocation = ValueNotifier<Offset?>(null);

  @override
  ValueListenable<DropOperation?> get dragCompleted => _dragCompleted;

  @override
  ValueListenable<bool> get dragging => _dragging;

  @override
  ValueListenable<Offset?> get lastScreenLocation => _lastScreenLocation;

  void startDrag(DragSession original) {
    this.original = original;
    original.dragCompleted.addListener(_originalDragCompleted);
    original.lastScreenLocation.addListener(() {
      _lastScreenLocation.value = original.lastScreenLocation.value;
    });
  }

  void _originalDragCompleted() {
    _dragging.value = false;
    _dragCompleted.value = original!.dragCompleted.value;
  }

  void _dispose() {
    original?.dragCompleted.removeListener(_originalDragCompleted);
    _dragCompleted.dispose();
    _dragging.dispose();
    _lastScreenLocation.dispose();
  }

  void beginDragging() {
    _dragging.value = true;
  }

  void cancelDragging() {
    if (_dragging.value) {
      _dragging.value = false;
      _dragCompleted.value = DropOperation.none;
    }
  }

  @override
  Future<List<Object?>?> getLocalData() {
    return original?.getLocalData() ?? Future.value(null);
  }
}
