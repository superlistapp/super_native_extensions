import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

class ButtonState {
  ButtonState({
    required this.enabled,
    required this.focused,
    required this.hovered,
    required this.pressed,
  });

  final bool enabled;
  final bool focused;
  final bool hovered;
  final bool pressed;

  bool get idle => !pressed && !hovered && !focused;

  static ButtonState of(BuildContext context) {
    final state = context.dependOnInheritedWidgetOfExactType<_ButtonState>();
    assert(state != null,
        'ButtonState.of() called with a context that does not contain a Button.');
    return state!.state;
  }
}

class _ButtonState extends InheritedWidget {
  const _ButtonState({
    required super.child,
    required this.state,
  });

  final ButtonState state;

  @override
  bool updateShouldNotify(covariant _ButtonState oldWidget) {
    return oldWidget.state != state;
  }
}

typedef CustomButtonBuilder = Widget Function(
  BuildContext context,
  ButtonState state,
  Widget? child,
);

/// Minimal but reasonably complete button implementation without the material
/// overhead.
class CustomButton extends StatefulWidget {
  const CustomButton({
    super.key,
    required this.onPressed,
    this.onKeyEvent,
    this.child,
    required this.builder,
    this.focusNode,
    this.tapToFocus = false,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
  });

  final VoidCallback? onPressed;
  final KeyEventResult? Function(KeyEvent)? onKeyEvent;
  final FocusNode? focusNode;
  final Widget? child;
  final CustomButtonBuilder builder;
  final bool tapToFocus;
  final HitTestBehavior hitTestBehavior;

  @override
  State<StatefulWidget> createState() => _CustomButtonState();
}

class _CustomButtonState extends State<CustomButton> {
  late FocusNode focusNode;

  @override
  void initState() {
    super.initState();

    focusNode = widget.focusNode ?? FocusNode(debugLabel: '$CustomButton');
    focusNode.onKeyEvent = _onKeyEvent;
    focusNode.addListener(_maybeShowOnScreen);
  }

  void _maybeShowOnScreen() {
    if (focusNode.hasFocus) {
      final ro = context.findRenderObject();
      if (ro != null) {
        ro.showOnScreen();
      }
    }
  }

  final _detector = GlobalKey();

  bool _hovered = false;
  bool _mousePressed = false;
  bool _keyPressed = false;

  void _onPressed() {
    if (widget.onPressed != null) {
      widget.onPressed?.call();

      HapticFeedback.lightImpact();
    }
  }

  KeyEventResult _onKeyEvent(FocusNode node, KeyEvent event) {
    assert(node == focusNode);
    if (widget.onPressed == null) {
      return KeyEventResult.ignored;
    }
    final widgetResult = widget.onKeyEvent?.call(event);
    if (widgetResult != null) {
      return widgetResult;
    }
    if (event is KeyDownEvent) {
      if (event.logicalKey == LogicalKeyboardKey.enter ||
          event.logicalKey == LogicalKeyboardKey.numpadEnter) {
        _onPressed();
        return KeyEventResult.handled;
      }
      if (event.logicalKey == LogicalKeyboardKey.space) {
        setState(() {
          _keyPressed = true;
        });
        return KeyEventResult.handled;
      }
    } else if (event is KeyUpEvent) {
      if (event.logicalKey == LogicalKeyboardKey.space) {
        final wasPressed = _keyPressed;
        setState(() {
          _keyPressed = false;
        });
        if (wasPressed) {
          _onPressed();
        }
        return KeyEventResult.handled;
      }
    }

    return KeyEventResult.ignored;
  }

  @override
  void didUpdateWidget(covariant CustomButton oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.focusNode != null && widget.focusNode != focusNode) {
      focusNode.onKeyEvent = null;
      focusNode.removeListener(_maybeShowOnScreen);
      if (oldWidget.focusNode != null) {
        focusNode.dispose();
      }
      focusNode = widget.focusNode!;
      focusNode.onKeyEvent = _onKeyEvent;
    }
    focusNode.canRequestFocus = widget.onPressed != null;
  }

  @override
  void dispose() {
    super.dispose();
    focusNode.removeListener(_maybeShowOnScreen);
    if (widget.focusNode != focusNode) {
      focusNode.dispose();
    }
  }

  bool get _isMobile =>
      defaultTargetPlatform == TargetPlatform.android ||
      defaultTargetPlatform == TargetPlatform.iOS;

  void _onTapUp(TapUpDetails details) {
    setState(() {
      _mousePressed = false;
      if (_isMobile) _hovered = false;
    });

    _onPressed();
  }

  void _onPanDown(DragDownDetails details) {
    setState(() {
      _mousePressed = true;
      if (_isMobile) _hovered = true;
    });
    if (widget.tapToFocus) {
      // This is an oversight in how traversal is implemented in Flutter
      // currently. Manually changing focus doesn't reset traversal history,
      // which can result in unexpected directional movement after.
      FocusTraversalGroup.of(context)
          // ignore: invalid_use_of_protected_member
          .invalidateScopeData(focusNode.nearestScope!);
      FocusScope.of(context).requestFocus(focusNode);
    }
  }

  void _onPanEnd(DragEndDetails details) {
    setState(() {
      _mousePressed = false;
    });
    if (_hovered) {
      _onPressed();
    }
  }

  void _onTapCancel() {
    if (_isMobile) {
      setState(() {
        _hovered = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final enabled = widget.onPressed != null;
    final state = ButtonState(
      enabled: enabled,
      focused: enabled && focusNode.hasFocus,
      hovered: enabled && _hovered,
      pressed: enabled && ((_mousePressed && _hovered) || _keyPressed),
    );
    return Semantics(
      button: true,
      container: true,
      enabled: enabled,
      onTap: widget.onPressed,
      child: Focus.withExternalFocusNode(
        focusNode: focusNode,
        onFocusChange: (_) {
          setState(() {});
        },
        child: MouseRegion(
          cursor: enabled ? SystemMouseCursors.click : MouseCursor.defer,
          onEnter: (event) {
            setState(() {
              _hovered = true;
            });
          },
          onExit: (event) {
            setState(() {
              _hovered = false;
            });
          },
          child: RawGestureDetector(
            behavior: widget.hitTestBehavior,
            key: _detector,
            gestures: {
              TapGestureRecognizer:
                  GestureRecognizerFactoryWithHandlers<TapGestureRecognizer>(
                () => TapGestureRecognizer(),
                (instance) {
                  instance.onTapUp = _onTapUp;
                  instance.onTapCancel = _onTapCancel;
                },
              ),
              _PanGestureRecognizer:
                  GestureRecognizerFactoryWithHandlers<_PanGestureRecognizer>(
                      () => _PanGestureRecognizer(), (instance) {
                instance.onDown = _onPanDown;
                instance.onEnd = _onPanEnd;
              }),
            },
            child: _ButtonState(
              state: state,
              child: widget.builder(context, state, widget.child),
            ),
          ),
        ),
      ),
    );
  }
}

class _PanGestureRecognizer extends PanGestureRecognizer {
  @override
  bool isPointerPanZoomAllowed(PointerPanZoomStartEvent event) {
    return false;
  }

  @override
  bool isPointerAllowed(PointerEvent event) {
    if (event.kind == PointerDeviceKind.mouse) {
      return event.buttons == 1;
    }
    return super.isPointerAllowed(event);
  }
}
