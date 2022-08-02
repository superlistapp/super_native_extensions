import 'package:js/js.dart';

@JS()
class Promise<T> {
  external Promise(
      void Function(void Function(T result) resolve, Function reject) executor);
  external Promise then(void Function(T result) onFulfilled,
      [Function onRejected]);
}

Promise<T> futureToPromise<T>(Future<T> future) {
  return Promise<T>(allowInterop((resolve, reject) {
    future.then(resolve, onError: reject);
  }));
}
