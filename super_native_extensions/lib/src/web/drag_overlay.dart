import 'package:flutter/widgets.dart';

import '../widget_snapshot/display_widget_snapshot.dart';
import '../widget_snapshot/widget_snapshot.dart';
import '../drag_interaction/shadow_image.dart';
import '../drag_interaction/util.dart';

abstract class DragOverlayState<T extends StatefulWidget> extends State<T> {
  void updatePosition(Offset position);
  void animateHome(Duration duration, VoidCallback onCompleted);
}

class DragOverlayDesktop extends StatefulWidget {
  final List<TargetedWidgetSnapshot> snapshots;
  final Offset initialPosition;

  const DragOverlayDesktop({
    super.key,
    required this.snapshots,
    required this.initialPosition,
  });

  @override
  State<StatefulWidget> createState() => _DragOverlayDesktopState();
}

class _DragOverlayDesktopState extends DragOverlayState<DragOverlayDesktop> {
  Offset _position = Offset.zero;

  @override
  void initState() {
    super.initState();
    _position = widget.initialPosition;
  }

  @override
  void animateHome(Duration duration, VoidCallback onCompleted) {
    _homeAnimation = SimpleAnimation.animate(duration, (value) {
      setState(() {
        _homeTransition = value;
      });
    }, onEnd: onCompleted);
  }

  SimpleAnimation? _homeAnimation;

  double? _homeTransition;

  @override
  void dispose() {
    super.dispose();
    _homeAnimation?.cancel();
  }

  @override
  void updatePosition(Offset position) {
    setState(() {
      _position = position;
    });
  }

  @override
  Widget build(BuildContext context) {
    var delta = _position - widget.initialPosition;
    if (_homeTransition != null) {
      delta = Offset.lerp(
        delta,
        Offset.zero,
        Curves.easeOutCubic.transform(_homeTransition!),
      )!;
    }

    final renderObject = context.findAncestorRenderObjectOfType<RenderBox>()!;

    double opacity = 0.7 * (1.0 - (_homeTransition ?? 0.0));

    return IgnorePointer(
      ignoring: true,
      child: Stack(
        fit: StackFit.expand,
        children: [
          for (var snapshot in widget.snapshots)
            () {
              final local = renderObject.globalToLocal(snapshot.rect.topLeft);
              return Positioned(
                left: local.dx + delta.dx,
                top: local.dy + delta.dy,
                width: snapshot.snapshot.pointWidth,
                height: snapshot.snapshot.pointHeight,
                child: Opacity(
                  opacity: opacity,
                  child: ShadowImage(
                    image: snapshot.snapshot,
                    shadowRadius: kShadowRadius,
                    shadowOpacity: 1.0,
                  ),
                ),
              );
            }()
        ],
      ),
    );
  }
}

class DragOverlayMobile extends StatefulWidget {
  const DragOverlayMobile({
    required this.snapshot,
    required this.initialPosition,
    Key? key,
  }) : super(key: key);

  final TargetedWidgetSnapshot snapshot;
  final Offset initialPosition;

  @override
  State<StatefulWidget> createState() => _DragOverlayMobileState();
}

class _DragOverlayMobileState extends DragOverlayState<DragOverlayMobile> {
  Offset _position = Offset.zero;

  @override
  void initState() {
    super.initState();
    _position = widget.initialPosition;
  }

  @override
  void updatePosition(Offset position) {
    setState(() {
      _position = position;
    });
  }

  double? _homeTransition;

  @override
  Widget build(BuildContext context) {
    var offset = Offset(
        _position.dx - widget.snapshot.snapshot.pointWidth / 2.0,
        _position.dy - widget.snapshot.snapshot.pointHeight / 2.0);

    if (_homeTransition != null) {
      offset = Offset.lerp(
        offset,
        widget.snapshot.rect.topLeft,
        Curves.easeOutCubic.transform(_homeTransition!),
      )!;
    }

    final renderObject = context.findAncestorRenderObjectOfType<RenderBox>()!;
    offset = renderObject.globalToLocal(offset);

    double opacity = 0.7 * (1.0 - (_homeTransition ?? 0.0));

    return IgnorePointer(
      ignoring: true,
      child: Stack(
        children: [
          Positioned(
            left: offset.dx,
            top: offset.dy,
            width: widget.snapshot.snapshot.pointWidth,
            height: widget.snapshot.snapshot.pointHeight,
            child: Opacity(
              opacity: opacity,
              child: DisplayWidgetSnapshot(widget.snapshot.snapshot),
            ),
          ),
        ],
      ),
    );
  }

  SimpleAnimation? _homeAnimation;

  @override
  void dispose() {
    super.dispose();
    _homeAnimation?.cancel();
  }

  @override
  void animateHome(Duration duration, VoidCallback onCompleted) {
    _homeAnimation = SimpleAnimation.animate(duration, (value) {
      setState(() {
        _homeTransition = value;
      });
    }, onEnd: onCompleted);
  }
}
