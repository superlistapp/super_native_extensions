import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import 'widget_snapshot.dart';

class DisplayWidgetSnapshot extends StatelessWidget {
  final WidgetSnapshot snapshot;

  const DisplayWidgetSnapshot(
    this.snapshot, {
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    if (snapshot.isImage) {
      return RawImage(
        image: snapshot.image,
        fit: BoxFit.fill,
      );
    } else {
      return LayoutBuilder(builder: (context, constraints) {
        return Transform.scale(
          alignment: Alignment.topLeft,
          scaleX: constraints.biggest.width / snapshot.pointSize.width,
          scaleY: constraints.biggest.height / snapshot.pointSize.height,
          child: _WidgetSnapshotWidget(snapshot),
        );
      });
    }
  }
}

class _WidgetSnapshotWidget extends SingleChildRenderObjectWidget {
  _WidgetSnapshotWidget(
    this.snapshot,
  ) : assert(snapshot.isRenderObject);

  final WidgetSnapshot snapshot;

  @override
  RenderObject createRenderObject(BuildContext context) {
    assert(!snapshot.debugRenderObjectRequested);
    return _RenderWidgetSnapshot()..snapshot = snapshot;
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant RenderObject renderObject) {
    final renderObject_ = renderObject as _RenderWidgetSnapshot;
    if (renderObject_.snapshot != snapshot) {
      assert(!snapshot.debugRenderObjectRequested);
      renderObject_.snapshot = snapshot;
      renderObject_.didPaint = false;
    }
  }
}

class _RenderWidgetSnapshot extends RenderProxyBox {
  WidgetSnapshot? snapshot;

  @override
  bool get isRepaintBoundary => true;

  @override
  void performLayout() {
    size = constraints.biggest;
  }

  @override
  void markNeedsPaint() {
    if (!didPaint) {
      super.markNeedsPaint();
    }
  }

  bool didPaint = false;

  @override
  void paint(PaintingContext context, Offset offset) {
    final snapshot = this.snapshot;
    if (snapshot != null) {
      final renderObject = snapshot.getRenderObject();
      renderObject?.paint(
        context,
        offset - snapshot.renderObjectBounds.topLeft,
      );
    }
    didPaint = true;
  }
}
