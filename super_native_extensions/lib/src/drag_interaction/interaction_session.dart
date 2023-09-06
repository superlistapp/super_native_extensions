import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import '../gesture/multi_touch_detector.dart';
import '../menu.dart';
import '../menu_model.dart';
import '../widget_snapshot/widget_snapshot.dart';
import '../gesture/single_drag.dart';
import 'long_press_handler.dart';
import 'drag_state_machine.dart';
import 'overlay_widget.dart';

typedef DragStartCallback = void Function(
  Offset globalPosition,
  int? pointer,
  WidgetSnapshot compositeSnapshot,
  VoidCallback draggingStarted,
);

class ItemConfiguration {
  ItemConfiguration({
    required this.liftImage,
    required this.dragImage,
  });

  final TargetedWidgetSnapshot liftImage;
  final WidgetSnapshot dragImage;
}

class DragInteractionMenuConfiguration {
  DragInteractionMenuConfiguration({
    required this.hasCustomMenuPreview,
    required this.iconTheme,
    required this.backgroundBuilder,
    required this.menuPreview,
    required this.menuHandle,
    required this.menuWidgetBuilder,
    required this.onMenuShown,
    required this.onMenuHidden,
    required this.onMenuPreviewTapped,
  });

  final MenuHandle menuHandle;
  final IconThemeData iconTheme;
  final MobileMenuWidgetFactory menuWidgetBuilder;
  final Widget Function(double opacity) backgroundBuilder;
  final ValueListenable<MenuPreviewWidget?> menuPreview;
  final bool hasCustomMenuPreview;
  final VoidCallback onMenuShown;
  final ValueSetter<MenuResult> onMenuHidden;
  final VoidCallback onMenuPreviewTapped;
}

class DragInteractionConfiguration {
  DragInteractionConfiguration({
    required this.primaryItem,
    required this.secondaryItems,
    required this.onBeginTransitionToDrag,
    required this.onDidNotFinishTransitionToDrag,
    required this.onDragStart,
    required this.onFinished,
    required this.menuConfiguration,
  });

  final ItemConfiguration primaryItem;
  final List<ItemConfiguration> secondaryItems;
  final VoidCallback onBeginTransitionToDrag;
  final VoidCallback onDidNotFinishTransitionToDrag;
  final DragStartCallback? onDragStart;
  final VoidCallback onFinished;
  final DragInteractionMenuConfiguration? menuConfiguration;
}

class DragInteractionSession implements DragDelegate {
  bool _menuItemSelected = false;

  DragInteractionSession({
    required BuildContext buildContext,
    required this.configuration,
  }) {
    final overlay = Overlay.of(buildContext, rootOverlay: true);
    _entry = OverlayEntry(
      builder: (context) {
        return OverlayWidget(
          key: _key,
          configuration: configuration,
          menuDragProvider: _createDragForMenu,
          onMenuItemSelected: () {
            _menuItemSelected = true;
          },
          onCancel: cancel,
        );
      },
      opaque: false,
    );
    overlay.insert(_entry);
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

  bool _menuHidden = false;
  bool _menuShown = false;

  bool _isDone = false;

  void _done() {
    if (_isDone) {
      return;
    }
    _isDone = true;

    if (!_menuHidden) {
      configuration.menuConfiguration?.onMenuHidden(MenuResult(
        itemSelected: _menuItemSelected,
      ));
      _menuHidden = true;
    }
    _entry.remove();
    configuration.onFinished();
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
    configuration.menuConfiguration?.onMenuShown();
    _menuShown = true;
  }

  @override
  void beginTransitionToDrag() {
    if (_menuShown) {
      configuration.menuConfiguration?.onMenuHidden(MenuResult(
        itemSelected: false,
      ));
      _menuHidden = true;
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
      configuration.menuConfiguration?.onMenuPreviewTapped();
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
    return configuration.menuConfiguration != null;
  }

  @override
  bool canTransitionToDrag() {
    // Transition to drag in Android with multi touch active messes
    // up touch events and potentially locks up the application.
    if (MultiTouchDetector.isMultiTouchActive()) {
      return false;
    }
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
