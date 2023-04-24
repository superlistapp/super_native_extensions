import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import '../../image.dart';
import '../../menu.dart';
import '../../util.dart';
import '../gesture/single_drag.dart';
import '../menu_widget/menu_stack.dart';
import '../menu_widget/menu_widget_builder.dart';
import 'controller.dart';
import 'drag_state_machine.dart';
import 'menu_preview_widget.dart';
import 'overlay_layout.dart';
import 'shadow_image.dart';
import 'util.dart';

class OverlayWidget extends StatefulWidget {
  const OverlayWidget({
    required this.primaryItem,
    required this.secondaryItems,
    required this.menuPreview,
    required this.menuDragProvider,
    required this.menuBuilder,
    required this.onCancel,
    required this.menuHandle,
    required this.menuWidgetBuilder,
    super.key,
  });

  final MenuHandle? menuHandle;

  final ItemConfiguration primaryItem;
  final List<ItemConfiguration> secondaryItems;

  final ValueListenable<MenuPreviewWidget?> menuPreview;
  final MenuBuilder? menuBuilder;

  final SingleDrag? Function(Offset, int pointer) menuDragProvider;
  final VoidCallback onCancel;

  final MenuWidgetBuilder? menuWidgetBuilder;

  @override
  State<StatefulWidget> createState() => OverlayWidgetState();
}

double _easeOut(double value) {
  return 1.0 - math.pow(1 - value, 3);
}

const kShadowRadius = 12;

class OverlayWidgetState extends State<OverlayWidget> implements MenuDelegate {
  @override
  void initState() {
    super.initState();
    for (final _ in widget.secondaryItems) {
      _secondaryDragKeys.add(GlobalKey());
      _secondaryLiftKeys.add(GlobalKey());
      _secondaryRenderKeys.add(GlobalKey());
    }
  }

  @override
  void dispose() {
    _menuCanScrollNotifier.dispose();
    _resetMenuOffsetAnimation?.cancel();
    _menuDragOffsetAnimation?.cancel();
    _hidingAnimation?.cancel();
    _onDisposed.notify();
    super.dispose();
  }

  double _menuDragExtent = 0;

  double get menuDragExtent => _menuDragExtent;

  void setMenuDragExtent(double value) {
    _menuDragExtent = value;
  }

  double get menuDragOffset => _currentState.menuDragOffset;

  final _repaintBoundary = GlobalKey();

  ui.Image getSnapshot() {
    final boundary = _repaintBoundary.currentContext!.findRenderObject()
        as RenderBetterRepaintBoundary;
    final pixelRatio = MediaQuery.of(context).devicePixelRatio;
    var size = Size(
      widget.primaryItem.dragImage.width / pixelRatio + kShadowRadius * 2,
      widget.primaryItem.dragImage.height / pixelRatio + kShadowRadius * 2,
    );
    for (final key in _secondaryRenderKeys) {
      final renderObject = key.currentContext!.findRenderObject();
      if (renderObject != null) {
        final box = renderObject as RenderBox;
        final transform = box.getTransformTo(boundary);
        final rect =
            MatrixUtils.transformRect(transform, box.paintBounds).inflate(14.0);
        size = Size(
          math.max(size.width, rect.width),
          math.max(size.height, rect.height),
        );
      }
    }
    return boundary.toImageSync(
      Rect.fromCenter(
          center: _currentState.globalPosition,
          width: size.width,
          height: size.height),
      pixelRatio: pixelRatio,
    )..devicePixelRatio = pixelRatio;
  }

  double _angleForSecondaryItem(int index) {
    final step = 0.2 / (widget.secondaryItems.length / 2);
    final flip = index % 2 == 0 ? 1 : -1;
    return step * flip * (widget.secondaryItems.length - index) / 2;
  }

  final _menuCanScrollNotifier = ValueNotifier(true);

  AlignmentGeometry _menuAlignment = Alignment.center;

  BoxConstraints? _lastConstraints;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<MenuPreviewWidget?>(
      valueListenable: widget.menuPreview,
      builder: (context, menuPreview, _) {
        return LayoutBuilder(builder: (context, constraints) {
          // Hide menu on rotation. The underlying widgets have likely been rebuilt anyway.
          if (_lastConstraints != null && _lastConstraints != constraints) {
            hide();
          }
          _lastConstraints = constraints;
          return _build(context, menuPreview);
        });
      },
    );
  }

  final _onDisposed = SimpleNotifier();

  Widget _build(BuildContext context, MenuPreviewWidget? menuPreview) {
    final backgroundOpacity = _currentState.menuFactor *
        (1.0 - _currentState.dragFactor) *
        (1.0 - _hideFactor);

    final mediaQuery = MediaQuery.of(context);

    final liftOpacity =
        1.0 - math.max(_currentState.menuFactor, _currentState.dragFactor);

    final menuOpacity =
        _currentState.menuFactor * (1.0 - _currentState.dragFactor);

    final dragOpacity = _currentState.dragFactor * 1;

    double secondaryLiftAlpha = (_currentState.dragFactor * 10).clamp(0, 1) *
        (1 - _currentState.dragFactor);
    double secondaryDragAlpha = _currentState.dragFactor;

    final layoutDelegate = OverlayLayoutDelegate(
      padding: mediaQuery.padding,
      menuDragExtentSetter: setMenuDragExtent,
      canScrollMenuSetter: (value) {
        _menuCanScrollNotifier.value = value;
      },
      menuAlignmentSetter: (value) {
        _menuAlignment = value;
      },
      primaryItem: LayoutItemConfiguration(
        index: 0,
        liftChildId: _liftKey,
        dragChildId: _dragKey,
        liftRect: widget.primaryItem.liftImage.rect,
        dragSize: widget.primaryItem.dragImage.pointSize,
        liftImage: widget.primaryItem.liftImage.image,
        dragImage: widget.primaryItem.dragImage,
      ),
      menuPreviewSize: menuPreview?.size,
      menuPreviewId: _menuPreviewKey,
      menuId: _menuKey,
      secondaryItems: widget.secondaryItems.mapIndexed((int index, e) {
        return LayoutItemConfiguration(
          index: index,
          liftChildId: _secondaryLiftKeys[index],
          dragChildId: _secondaryDragKeys[index],
          liftRect: e.liftImage.rect,
          dragSize: e.dragImage.pointSize,
          liftImage: e.liftImage.image,
          dragImage: e.dragImage,
        );
      }).toList(growable: false),
      dragState: _currentState,
    );

    return RawGestureDetector(
      behavior: HitTestBehavior.translucent,
      gestures: {
        SingleDragGestureRecognizer:
            GestureRecognizerFactoryWithHandlers<SingleDragGestureRecognizer>(
          () => SingleDragGestureRecognizer(debugOwner: this),
          (SingleDragGestureRecognizer instance) {
            instance.onDragStart = (Offset position) {
              if (_currentState.menuFactor < 1 || _hidingAnimation != null) {
                return null;
              }
              final drag = widget.menuDragProvider(
                position,
                instance.lastPointer!,
              );
              // When recognizer disappers the drag is not cancelled, which prevents
              // subsequent drags from
              return drag;
            };
          },
        ),
      },
      child: _Background(
        opacity: backgroundOpacity,
        menuWidgetBuilder: widget.menuWidgetBuilder,
        child: Opacity(
          opacity: (1.0 - 0.3 * _currentState.dragFactor) * (1.0 - _hideFactor),
          child: BetterRepaintBoundary(
            key: _repaintBoundary,
            child: CustomMultiChildLayout(
              delegate: layoutDelegate,
              children: [
                for (final item in layoutDelegate.secondaryItems)
                  LayoutId(
                    key: _secondaryLiftKeys[item.index],
                    id: item.liftChildId,
                    child: Opacity(
                      opacity: secondaryLiftAlpha,
                      child: ShadowImage(
                          image: item.liftImage,
                          shadowRadius: kShadowRadius,
                          shadowOpacity: 1.0),
                    ),
                  ),
                for (final item in layoutDelegate.secondaryItems)
                  LayoutId(
                    id: item.dragChildId,
                    key: _secondaryDragKeys[item.index],
                    child: Opacity(
                      opacity: secondaryDragAlpha,
                      child: Transform.rotate(
                        angle: _angleForSecondaryItem(item.index) *
                            _currentState.dragFactor,
                        child: ShadowImage(
                            key: _secondaryRenderKeys[item.index],
                            image: item.dragImage,
                            shadowRadius: kShadowRadius,
                            shadowOpacity: 1.0),
                      ),
                    ),
                  ),
                LayoutId(
                  id: layoutDelegate.primaryItem.liftChildId,
                  key: _liftKey,
                  child: Opacity(
                    opacity: liftOpacity,
                    child: ShadowImage(
                      image: widget.primaryItem.liftImage.image,
                      shadowRadius: kShadowRadius,
                      shadowOpacity: _currentState.liftFactor,
                    ),
                  ),
                ),
                if (widget.menuBuilder != null && _currentState.menuFactor > 0)
                  LayoutId(
                    id: layoutDelegate.menuPreviewId,
                    key: _menuPreviewKey,
                    child: RepaintBoundary(
                      child: Opacity(
                        opacity: menuOpacity,
                        child: menuPreview,
                      ),
                    ),
                  ),
                if (widget.menuBuilder != null && _currentState.menuFactor > 0)
                  LayoutId(
                    key: _menuKey,
                    id: layoutDelegate.menuId,
                    child: Opacity(
                      opacity: menuOpacity,
                      child: Transform.scale(
                        alignment: _menuAlignment,
                        scale: menuOpacity * _easeOut(1.0 - _hideFactor),
                        child: widget.menuBuilder!(
                          context,
                          this,
                          _menuAlignment,
                          _menuCanScrollNotifier,
                        ),
                      ),
                    ),
                  ),
                LayoutId(
                  id: layoutDelegate.primaryItem.dragChildId,
                  key: _dragKey,
                  child: Opacity(
                    opacity: dragOpacity,
                    child: IgnorePointer(
                      ignoring: true,
                      child: ShadowImage(
                        shadowRadius: kShadowRadius,
                        shadowOpacity: 1.0,
                        image: layoutDelegate.primaryItem.dragImage,
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  final _menuKey = GlobalKey();
  final _menuPreviewKey = GlobalKey();
  final _liftKey = GlobalKey();
  final _dragKey = GlobalKey();

  final _secondaryDragKeys = <GlobalKey>[];
  final _secondaryLiftKeys = <GlobalKey>[];
  final _secondaryRenderKeys = <GlobalKey>[];

  double _hideFactor = 0;

  bool isMenuPreviewAtPosition(Offset globalPosition) {
    final renderBox =
        _menuPreviewKey.currentContext!.findRenderObject() as RenderBox;
    final localPosition = renderBox.globalToLocal(globalPosition);
    return renderBox.paintBounds.contains(localPosition);
  }

  void hide() {
    if (!mounted) {
      widget.onCancel;
      return;
    }
    final originalMenu = _currentState.menuFactor;
    final originalMenuOffset = _currentState.menuOverdrag;
    _hidingAnimation =
        SimpleAnimation.animate(const Duration(milliseconds: 300), (value) {
      if (!mounted) {
        return;
      }
      setState(() {
        _hideFactor = value;
        _currentState = _currentState.copyWith(
          menuFactor: ui.lerpDouble(originalMenu, 0, _easeOut(value))!,
          menuOverdrag:
              Offset.lerp(originalMenuOffset, Offset.zero, _easeOut(value))!,
        );
      });
    }, onEnd: () {
      widget.onCancel();
    });
  }

  SimpleAnimation? _hidingAnimation;
  SimpleAnimation? _resetMenuOffsetAnimation;
  SimpleAnimation? _menuDragOffsetAnimation;

  var _currentState = DragState(
    dragFactor: 0.0,
    globalPosition: Offset.zero,
    liftFactor: 0.0,
    menuFactor: 0.0,
    menuOverdrag: Offset.zero,
    menuDragOffset: 0,
  );

  void menuDragEnded(double velocity) {
    if (_currentState.menuOverdrag != Offset.zero ||
        (_currentState.menuDragOffset != 0 &&
            _currentState.menuDragOffset != 1)) {
      final originalOffset = _currentState.menuOverdrag;
      final originalDragOffset = _currentState.menuDragOffset;
      final double newDragOffset;
      if (velocity < -1000.0) {
        newDragOffset = 1.0;
      } else if (velocity > 1000.0) {
        newDragOffset = 0;
      } else {
        newDragOffset = _currentState.menuDragOffset.roundToDouble();
      }

      _resetMenuOffsetAnimation =
          SimpleAnimation.animate(const Duration(milliseconds: 200), (v) {
        final value = _easeOut(v);
        setState(() {
          _currentState = _currentState.copyWith(
            menuOverdrag: Offset.lerp(originalOffset, Offset.zero, value),
            menuDragOffset:
                ui.lerpDouble(originalDragOffset, newDragOffset, value),
          );
        });
      });
    }
  }

  bool isMenuOpened() {
    return _currentState.menuFactor > 0;
  }

  void update(DragState state) {
    _resetMenuOffsetAnimation?.cancel();
    _menuDragOffsetAnimation?.cancel;

    setState(() {
      _currentState = DragState(
        dragFactor: state.dragFactor,
        globalPosition: state.globalPosition,
        liftFactor: state.liftFactor,
        menuFactor: _easeOut(state.menuFactor),
        menuOverdrag: state.menuOverdrag,
        menuDragOffset: state.menuDragOffset,
      );
    });
  }

  @override
  void hideMenu() {
    hide();
  }

  @override
  void didPushSubmenu() {
    if (_currentState.menuDragOffset < 1.0) {
      final originalOffset = _currentState.menuDragOffset;
      _menuDragOffsetAnimation =
          SimpleAnimation.animate(const Duration(milliseconds: 200), (v) {
        final value = _easeOut(v);
        setState(() {
          _currentState = _currentState.copyWith(
            menuDragOffset: ui.lerpDouble(originalOffset, 1.0, value),
          );
        });
      });
    }
  }
}

class _Background extends StatefulWidget {
  final Widget child;
  final double opacity;
  final MenuWidgetBuilder? menuWidgetBuilder;

  const _Background({
    required this.opacity,
    required this.child,
    required this.menuWidgetBuilder,
  });

  @override
  State<_Background> createState() => _BackgroundState();
}

class _BackgroundState extends State<_Background> {
  final _childKey = GlobalKey();

  @override
  Widget build(BuildContext context) {
    if (widget.opacity == 0) {
      return KeyedSubtree(
        key: _childKey,
        child: widget.child,
      );
    }
    return Stack(
      children: [
        if (widget.menuWidgetBuilder != null)
          widget.menuWidgetBuilder!.buildOverlayBackground(
            context,
            widget.opacity,
          ),
        Positioned.fill(
          child: KeyedSubtree(
            key: _childKey,
            child: widget.child,
          ),
        ),
      ],
    );
  }
}
