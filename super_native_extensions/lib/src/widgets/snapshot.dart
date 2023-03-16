import 'dart:async';

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import '../api_model.dart';

enum SnapshotType {
  lift,
  drag,
}

typedef SnapshotBuilder = Widget Function(
  BuildContext context,
  SnapshotType? type,
);

class CustomSnapshotWidget extends StatefulWidget {
  const CustomSnapshotWidget({
    super.key,
    required this.builder,
    this.supportedTypes = const {},
  });

  final Set<SnapshotType> supportedTypes;
  final SnapshotBuilder builder;

  @override
  State<CustomSnapshotWidget> createState() => _CustomSnapshotWidgetState();
}

abstract class Snapshotter {
  static Snapshotter? of(BuildContext context) {
    final real = context.findAncestorStateOfType<_CustomSnapshotWidgetState>();
    if (real != null) {
      return real;
    } else {
      return context.findAncestorStateOfType<_FallbackSnapshotWidgetState>();
    }
  }

  void arm();
  void disarm();

  Future<TargettedImage?> getSnapshot(SnapshotType? type);
}

class _PendingSnapshot {
  _PendingSnapshot(this.type, this.completer);

  final SnapshotType? type;
  final Completer<TargettedImage?> completer;
}

TargettedImage _getSnapshot(
    BuildContext context, RenderRepaintBoundary renderObject) {
  final image = renderObject.toImageSync(
      pixelRatio: MediaQuery.of(context).devicePixelRatio);
  final transform = renderObject.getTransformTo(null);
  final r =
      Rect.fromLTWH(0, 0, renderObject.size.width, renderObject.size.height);
  final rect = MatrixUtils.transformRect(transform, r);
  return TargettedImage(image, rect);
}

class _ZeroClipper extends CustomClipper<Rect> {
  const _ZeroClipper();

  @override
  Rect getClip(Size size) {
    return Rect.zero;
  }

  @override
  bool shouldReclip(covariant CustomClipper<Rect> oldClipper) {
    return false;
  }
}

class _CustomSnapshotWidgetState extends State<CustomSnapshotWidget>
    implements Snapshotter {
  @override
  Widget build(BuildContext context) {
    return Builder(builder: (context) {
      if (!_armed && _pendingSnapshots.isEmpty) {
        return KeyedSubtree(
          key: _contentKey,
          child: widget.builder(context, null),
        );
      } else {
        return Stack(
          fit: StackFit.passthrough,
          children: [
            RepaintBoundary(
              key: _defaultKey,
              child: KeyedSubtree(
                key: _contentKey,
                child: widget.builder(context, null),
              ),
            ),
            for (final type in widget.supportedTypes)
              IgnorePointer(
                ignoring: true,
                child: ClipRect(
                  clipper: const _ZeroClipper(),
                  child: RepaintBoundary(
                    key: _keys[type],
                    child: widget.builder(context, type),
                  ),
                ),
              ),
          ],
        );
      }
    });
  }

  bool _armed = false;
  final _pendingSnapshots = <_PendingSnapshot>[];

  final _contentKey = GlobalKey();

  final _defaultKey = GlobalKey();

  final _keys = {
    SnapshotType.lift: GlobalKey(),
    SnapshotType.drag: GlobalKey(),
  };

  RenderRepaintBoundary? _getRenderObject(SnapshotType? type) {
    final object = type != null
        ? _keys[type]?.currentContext?.findRenderObject()
        : _defaultKey.currentContext?.findRenderObject();
    return object is RenderRepaintBoundary ? object : null;
  }

  @override
  void arm() {
    if (!_armed) {
      setState(() {
        _armed = true;
      });
    }
  }

  @override
  void disarm() {
    if (_armed) {
      setState(() {
        _armed = false;
      });
    }
  }

  void _checkSnapshots() {
    if (_pendingSnapshots.isEmpty) {
      return;
    }
    if (!mounted) {
      for (final snapshot in _pendingSnapshots) {
        snapshot.completer.complete(null);
      }
      _pendingSnapshots.clear();
      return;
    }
    if (_getRenderObject(null) == null) {
      setState(() {});
      WidgetsBinding.instance.addPostFrameCallback((timeStamp) {
        _checkSnapshots();
      });
      return;
    }

    for (final s in _pendingSnapshots) {
      final renderObject = _getRenderObject(s.type);
      if (renderObject != null) {
        s.completer.complete(_getSnapshot(context, renderObject));
      } else {
        s.completer.complete(null);
      }
    }
    _pendingSnapshots.clear();
    setState(() {});
  }

  @override
  Future<TargettedImage?> getSnapshot(SnapshotType? type) {
    final completer = Completer<TargettedImage?>();
    _pendingSnapshots.add(_PendingSnapshot(type, completer));
    _checkSnapshots();
    return completer.future;
  }
}

class FallbackSnapshotWidget extends StatefulWidget {
  const FallbackSnapshotWidget({
    super.key,
    required this.child,
  });

  final Widget child;

  @override
  State<FallbackSnapshotWidget> createState() => _FallbackSnapshotWidgetState();
}

class _FallbackSnapshotWidgetState extends State<FallbackSnapshotWidget>
    implements Snapshotter {
  final _contentKey = GlobalKey();
  final _repaintBoundaryKey = GlobalKey();

  final _pendingSnapshots = <Completer<TargettedImage?>>[];

  bool _armed = false;

  @override
  Widget build(BuildContext context) {
    if (!_armed && _pendingSnapshots.isEmpty) {
      return KeyedSubtree(key: _contentKey, child: widget.child);
    }
    if (_armed || _pendingSnapshots.isNotEmpty) {
      return RepaintBoundary(
        key: _repaintBoundaryKey,
        child: KeyedSubtree(key: _contentKey, child: widget.child),
      );
    } else {
      return KeyedSubtree(key: _contentKey, child: widget.child);
    }
  }

  @override
  Future<TargettedImage?> getSnapshot(SnapshotType? type) {
    if (type != null) {
      return Future.value(null);
    }

    final completer = Completer<TargettedImage>();
    _pendingSnapshots.add(completer);
    _checkSnapshot();
    return completer.future;
  }

  void _checkSnapshot() {
    if (!mounted) {
      for (final completer in _pendingSnapshots) {
        completer.complete(null);
      }
      _pendingSnapshots.clear();
      return;
    }
    final object = _repaintBoundaryKey.currentContext?.findRenderObject();
    if (object is RenderRepaintBoundary) {
      final snapshot = _getSnapshot(context, object);
      for (final completer in _pendingSnapshots) {
        completer.complete(snapshot);
      }
      _pendingSnapshots.clear();
      setState(() {});
    } else {
      setState(() {});
      WidgetsBinding.instance.addPostFrameCallback((timeStamp) {
        _checkSnapshot();
      });
    }
  }

  @override
  void arm() {
    if (!_armed) {
      setState(() {
        _armed = true;
      });
    }
  }

  @override
  void disarm() {
    if (_armed) {
      setState(() {
        _armed = false;
      });
    }
  }
}
