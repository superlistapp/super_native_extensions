import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import '../../menu.dart';
import '../gesture/single_drag.dart';
import '../menu_widget/menu_stack.dart';
import '../menu_widget/menu_widget_builder.dart';
import 'drag_state_machine.dart';
import 'menu_preview_widget.dart';
import 'overlay_widget.dart';
import '../../api_model.dart';

typedef DragStartCallback = void Function(
  Offset globalPosition,
  int? pointer,
  ui.Image compositeSnapshot,
  VoidCallback draggingStarted,
);

typedef MenuBuilder = Widget Function(
  BuildContext context,
  MenuDelegate menuDelegate,
  AlignmentGeometry menuAlignment,
  ValueListenable<bool> canScrollListenable,
);

class ItemConfiguration {
  ItemConfiguration({
    required this.liftImage,
    ui.Image? dragImage,
  }) : _dragImage = dragImage;

  final TargetedImage liftImage;
  ui.Image get dragImage => _dragImage ?? liftImage.image;
  final ui.Image? _dragImage;
}

class DragInteractionConfiguration {
  DragInteractionConfiguration({
    required this.primaryItem,
    required this.secondaryItems,
    required this.menuHandle,
    required this.menuBuilder,
    required this.menuWidgetBuilder,
    required this.menuPreview,
    required this.onBeginTransitionToDrag,
    required this.onDidNotFinishTransitionToDrag,
    required this.onDragStart,
    required this.onMenuShown,
    required this.onMenuHidden,
    required this.onMenuPreviewTapped,
  });

  final ItemConfiguration primaryItem;
  final List<ItemConfiguration> secondaryItems;

  MenuHandle? menuHandle;
  MenuBuilder? menuBuilder;
  MenuWidgetBuilder? menuWidgetBuilder;
  final ValueListenable<MenuPreviewWidget?> menuPreview;

  final VoidCallback onBeginTransitionToDrag;
  final VoidCallback onDidNotFinishTransitionToDrag;
  final VoidCallback onMenuShown;
  final VoidCallback onMenuHidden;
  final VoidCallback onMenuPreviewTapped;
  final DragStartCallback? onDragStart;
}

class DragInteractionSession implements DragDelegate {
  static bool _active = false;

  static bool get active => _active;

  DragInteractionSession({
    required BuildContext buildContext,
    required this.configuration,
  }) {
    final overlay = Overlay.of(buildContext, rootOverlay: true);
    _entry = OverlayEntry(
      builder: (context) {
        return OverlayWidget(
          key: _key,
          primaryItem: configuration.primaryItem,
          secondaryItems: configuration.secondaryItems,
          menuPreview: configuration.menuPreview,
          menuBuilder: configuration.menuBuilder,
          menuDragProvider: _createDragForMenu,
          menuHandle: configuration.menuHandle,
          menuWidgetBuilder: configuration.menuWidgetBuilder,
          onCancel: cancel,
        );
      },
      opaque: false,
    );
    overlay.insert(_entry);
    _active = true;
  }

  double _menuDragExtent() => _key.currentState?.menuDragExtent ?? 0;

  SingleDrag intoDrag(Offset offset, int pointer) {
    return DragInteractionDrag(
        delegate: this,
        pointer: pointer,
        initialOffset: offset,
        menuDragExtent: _menuDragExtent,
        initialMenuDragOffset: 0);
  }

  DragInteractionConfiguration configuration;

  SingleDrag _createDragForMenu(Offset offset, int pointer) {
    return DragInteractionDrag(
      delegate: this,
      pointer: pointer,
      initialOffset: offset,
      menuDragExtent: _menuDragExtent,
      initialMenuDragOffset: _key.currentState?.menuDragOffset ?? 0,
    );
  }

  final GlobalKey<OverlayWidgetState> _key = GlobalKey();
  late OverlayEntry _entry;

  bool _menuShown = false;

  void _done() {
    if (_menuShown) {
      configuration.onMenuHidden();
      _menuShown = false;
    }
    if (_active) {
      _entry.remove();
      _active = false;
    }
  }

  @override
  @protected
  void beginDrag(Offset globalPosition, int? pointer) {
    WidgetsBinding.instance.addPostFrameCallback((timeStamp) {
      final snapshot = _key.currentState?.getSnapshot();
      if (snapshot == null) {
        _done();
      } else {
        configuration.onDragStart?.call(globalPosition, pointer, snapshot, () {
          _done();
        });
      }
    });
  }

  @override
  void beginTransitonToMenu() {
    configuration.onMenuShown();
    _menuShown = true;
  }

  @override
  void beginTransitionToDrag() {
    if (_menuShown) {
      configuration.onMenuHidden();
      _menuShown = false;
    }
    configuration.onBeginTransitionToDrag();
  }

  @override
  void didNotFinishTransitionToDrag() {
    configuration.onDidNotFinishTransitionToDrag();
  }

  @override
  void onTapUp(Offset globalPosition) {
    final inPreview =
        _key.currentState?.isMenuPreviewAtPosition(globalPosition) ?? false;
    if (inPreview) {
      configuration.onMenuPreviewTapped();
    }
    _key.currentState?.hide();
  }

  @override
  @protected
  void cancel() {
    _done();
  }

  @override
  @protected
  bool hasMenu() {
    return configuration.menuBuilder != null;
  }

  @override
  bool canTransitionToDrag() {
    return configuration.onDragStart != null;
  }

  @override
  @protected
  bool isMenuOpened() {
    return _key.currentState?.isMenuOpened() ?? false;
  }

  @override
  @protected
  void menuDragEnded(double velocity) {
    _key.currentState?.menuDragEnded(velocity);
  }

  @override
  @protected
  void updateState(DragState state) {
    _key.currentState?.update(state);
  }
}
