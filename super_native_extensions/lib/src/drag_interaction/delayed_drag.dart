import '../gesture/single_drag.dart';

class DelayedDrag implements SingleDrag {
  DelayedDrag(Future<SingleDrag?> dragFuture) {
    _start(dragFuture);
  }

  void _start(Future<SingleDrag?> drag) async {
    _drag = await drag;
    if (_cancelled) {
      _drag?.cancel();
    } else {
      if (_longPressRecognized) {
        _drag?.longPressRecognized();
      }
      if (_endDetails != null) {
        _drag?.end(_endDetails!);
      }
    }
  }

  SingleDrag? _drag;
  bool _cancelled = false;
  SingleDragEndDetails? _endDetails;
  bool _longPressRecognized = false;

  @override
  void longPressRecognized() {
    _longPressRecognized = true;
    _drag?.longPressRecognized();
  }

  @override
  void cancel() {
    _cancelled = true;
    _drag?.cancel();
  }

  @override
  void end(SingleDragEndDetails details) {
    _drag?.end(details);
    _endDetails = details;
  }

  @override
  void update(SingleDragUpdateDetails details) {
    _drag?.update(details);
  }
}
