import 'dart:async';

extension FutureOrThen<T> on FutureOr<T> {
  FutureOr<R> then<R>(FutureOr<R> Function(T value) callback) {
    if (this is Future<T>) {
      return (this as Future<T>).then(callback);
    } else {
      return callback(this as T);
    }
  }
}
