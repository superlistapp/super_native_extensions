import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:collection/collection.dart';
import 'package:flutter/widgets.dart';
import 'package:pixel_snap/pixel_snap.dart';

import '../gesture/multi_touch_detector.dart';
import '../menu.dart';
import '../repaint_boundary.dart';
import '../util.dart';
import '../widget_snapshot/widget_snapshot.dart';
import '../widget_snapshot/widget_snapshotter.dart';
import '../gesture/single_drag.dart';
import 'long_press_handler.dart';
import 'interaction_session.dart';
import 'drag_state_machine.dart';
import 'overlay_layout.dart';
import 'shadow_image.dart';
import 'util.dart';

class OverlayWidget extends StatefulWidget {
  const OverlayWidget({
    required this.configuration,
    required this.menuDragProvider,
    required this.onCancel,
    required this.onMenuItemSelected,
    super.key,
  });

  final DragInteractionConfiguration configuration;
  final SingleDrag? Function(Offset, int pointer) menuDragProvider;
  final VoidCallback onCancel;
  final VoidCallback onMenuItemSelected;

  @override
  State<StatefulWidget> createState() => OverlayWidgetState();
}

double _easeOut(double value) {
  const curve = Cubic(0.1, 0.48, 0.31, 0.57);
  return curve.transform(value);
}

const kShadowRadius = 12;

class OverlayWidgetState extends State<OverlayWidget>
    implements MobileMenuDelegate {
  @override
  void initState() {
    super.initState();
    for (final _ in widget.configuration.secondaryItems) {
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
    _onDisposed.dispose();
    super.dispose();
  }

  double _menuDragExtent = 0;

  double get menuDragExtent => _menuDragExtent;

  void setMenuDragExtent(double value) {
    _menuDragExtent = value;
  }

  double get menuDragOffset => _currentState.menuDragOffset;

  final _repaintBoundary = GlobalKey();

  WidgetSnapshot getSnapshot() {
    final boundary = _repaintBoundary.currentContext!.findRenderObject()
        as RenderBetterRepaintBoundary;
    final pixelRatio = MediaQuery.of(context).devicePixelRatio;
    var size = Size(
      widget.configuration.primaryItem.dragImage.pointWidth + kShadowRadius * 2,
      widget.configuration.primaryItem.dragImage.pointHeight +
          kShadowRadius * 2,
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

    final rect = Rect.fromCenter(
        center: boundary.globalToLocal(_currentState.globalPosition),
        width: size.width,
        height: size.height);

    if (WidgetSnapshotter.snapshotToImageSupported()) {
      final image = boundary.toImageSync(bounds: rect, pixelRatio: pixelRatio)
        ..devicePixelRatio = pixelRatio;
      return WidgetSnapshot.image(image);
    } else {
      return WidgetSnapshot.renderObject(
        boundary,
        rect,
      );
    }
  }

  double _angleForSecondaryItem(int index) {
    final step = 0.2 / (widget.configuration.secondaryItems.length / 2);
    final flip = index % 2 == 0 ? 1 : -1;
    return step *
        flip *
        (widget.configuration.secondaryItems.length - index) /
        2;
  }

  final _menuCanScrollNotifier = ValueNotifier(true);

  AlignmentGeometry _menuAlignment = Alignment.center;

  BoxConstraints? _lastConstraints;

  @override
  Widget build(BuildContext context) {
    if (widget.configuration.menuConfiguration != null) {
      return ValueListenableBuilder<MenuPreviewWidget?>(
        valueListenable: widget.configuration.menuConfiguration!.menuPreview,
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
    } else {
      return _build(context, null);
    }
  }

  final _onDisposed = SimpleNotifier();

  final _layoutState = OverlayLayoutState();

  Widget _build(BuildContext context, MenuPreviewWidget? menuPreview) {
    final menuConfiguration = widget.configuration.menuConfiguration;

    final backgroundOpacity = _currentState.menuFactor *
        (1.0 - _currentState.dragFactor) *
        (1.0 - _hideFactor);

    final mediaQuery = MediaQuery.of(context);

    final liftOpacity =
        1.0 - math.max(_currentState.menuFactor, _currentState.dragFactor);

    // Bump the opacity a bit in the middle of transition to avoid background
    // showing through.
    const menuOpacityCurve = Cubic(0.16, 0.7, 0.41, 0.88);
    final menuOpacity = menuOpacityCurve.transform(_currentState.menuFactor) *
        (1.0 - _currentState.dragFactor);

    final dragOpacity = _currentState.dragFactor * 1;

    double secondaryLiftAlpha = (_currentState.dragFactor * 10).clamp(0, 1) *
        (1 - _currentState.dragFactor);
    double secondaryDragAlpha = _currentState.dragFactor;

    final renderBox = context.findAncestorRenderObjectOfType<RenderBox>()!;
    final transform = renderBox.getTransformTo(null)..invert();

    Rect rectToLocal(Rect rect) => MatrixUtils.transformRect(transform, rect);
    Offset pointToLocal(Offset offset) =>
        MatrixUtils.transformPoint(transform, offset);

    final layoutDelegate = OverlayLayoutDelegate(
      layoutState: _layoutState,
      pixelSnap: PixelSnap.of(context),
      padding: mediaQuery.padding,
      menuDragExtentSetter: setMenuDragExtent,
      canScrollMenuSetter: (value) {
        _menuCanScrollNotifier.value = value;
      },
      hasCustomMenuPreview: menuConfiguration?.hasCustomMenuPreview ?? false,
      menuAlignmentSetter: (value) {
        _menuAlignment = value;
      },
      primaryItem: LayoutItemConfiguration(
        index: 0,
        liftChildId: _liftKey,
        dragChildId: _dragKey,
        liftRect: rectToLocal(widget.configuration.primaryItem.liftImage.rect),
        dragSize: widget.configuration.primaryItem.dragImage.pointSize,
        liftImage: widget.configuration.primaryItem.liftImage.snapshot,
        dragImage: widget.configuration.primaryItem.dragImage,
      ),
      menuPreviewSize: menuPreview?.size,
      menuPreviewId: _menuPreviewKey,
      menuId: _menuKey,
      secondaryItems:
          widget.configuration.secondaryItems.mapIndexed((int index, e) {
        return LayoutItemConfiguration(
          index: index,
          liftChildId: _secondaryLiftKeys[index],
          dragChildId: _secondaryDragKeys[index],
          liftRect: rectToLocal(e.liftImage.rect),
          dragSize: e.dragImage.pointSize,
          liftImage: e.liftImage.snapshot,
          dragImage: e.dragImage,
        );
      }).toList(growable: false),
      dragState: _currentState.copyWith(
          globalPosition: pointToLocal(
        _currentState.globalPosition,
      )),
    );

    return MultiTouchDetector(
      child: RawGestureDetector(
        behavior: HitTestBehavior.translucent,
        gestures: {
          SingleDragGestureRecognizer:
              GestureRecognizerFactoryWithHandlers<SingleDragGestureRecognizer>(
            () => SingleDragGestureRecognizer(debugOwner: this),
            (SingleDragGestureRecognizer instance) {
              instance.onDragStart = (Offset position) {
                if (_currentState.menuFactor == 0 || _hidingAnimation != null) {
                  return null;
                }
                final drag = widget.menuDragProvider(
                  position,
                  instance.lastPointer!,
                );
                // When recognizer disappears the drag is not cancelled, which prevents
                // subsequent drags from
                return drag;
              };
            },
          ),
        },
        child: _Background(
          opacity: backgroundOpacity,
          backgroundBuilder:
              widget.configuration.menuConfiguration?.backgroundBuilder,
          child: Opacity(
            opacity:
                (1.0 - 0.3 * _currentState.dragFactor) * (1.0 - _hideFactor),
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
                        image:
                            widget.configuration.primaryItem.liftImage.snapshot,
                        shadowRadius: kShadowRadius,
                        shadowOpacity: _currentState.liftFactor,
                      ),
                    ),
                  ),
                  if (menuConfiguration != null && _currentState.menuFactor > 0)
                    LayoutId(
                      id: layoutDelegate.menuPreviewId,
                      key: _menuPreviewKey,
                      child: RepaintBoundary(
                        child: Opacity(
                          opacity: menuOpacity,
                          child: menuPreview?.widget,
                        ),
                      ),
                    ),
                  if (menuConfiguration != null && _currentState.menuFactor > 0)
                    LayoutId(
                      key: _menuKey,
                      id: layoutDelegate.menuId,
                      child: Opacity(
                        opacity: menuOpacity,
                        child: Transform.scale(
                          alignment: _menuAlignment,
                          scale: menuOpacity * _easeOut(1.0 - _hideFactor),
                          child: menuConfiguration.menuWidgetBuilder(
                            context,
                            menuConfiguration.menuHandle.menu,
                            this,
                            _menuAlignment,
                            _menuCanScrollNotifier,
                            menuConfiguration.iconTheme,
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
          SimpleAnimation.animate(const Duration(milliseconds: 150), (v) {
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
        liftFactor: _easeOut(state.liftFactor),
        menuFactor: _easeOut(state.menuFactor),
        menuOverdrag: state.menuOverdrag,
        menuDragOffset: state.menuDragOffset,
      );
    });
  }

  @override
  void hideMenu({required bool itemSelected}) {
    if (itemSelected) {
      widget.onMenuItemSelected();
    }
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
  final Widget Function(double opacity)? backgroundBuilder;

  const _Background({
    required this.opacity,
    required this.child,
    required this.backgroundBuilder,
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
        if (widget.backgroundBuilder != null)
          widget.backgroundBuilder!(
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
