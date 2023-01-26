import 'package:flutter/foundation.dart';
import 'package:js/js.dart';
import 'dart:js_util' as js_util;
import 'dart:html' as html;

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

extension DataTransferItemExt on html.DataTransferItem {
  bool get isString => kind == 'string';
  bool get isFile => kind == 'file';

  String get format {
    final type = this.type ?? '';
    if (type.isNotEmpty) {
      return type;
    } else if (isString) {
      return 'text/plain';
    } else {
      return 'application/octet-stream';
    }
  }

  void getAsString(ValueChanged<String> callback) {
    js_util.callMethod(this, 'getAsString', [allowInterop(callback)]);
  }
}

extension BlobExt on html.Blob {
  ReadableStream stream() => js_util.callMethod(this, 'stream', []);
}

@JS()
@staticInterop
class ReadableStream {
  external factory ReadableStream();
}

extension ReadableStreamMethods on ReadableStream {
  ReadableStreamDefaultReader getReader() =>
      js_util.callMethod(this, 'getReader', []);
}

@JS()
@staticInterop
class ReadableStreamDefaultReader implements ReadableStreamGenericReader {
  external factory ReadableStreamDefaultReader(ReadableStream stream);
}

extension ReadableStreamDefaultReaderExt on ReadableStreamDefaultReader {
  Future<ReadableStreamReadResult> read() =>
      js_util.promiseToFuture(js_util.callMethod(this, 'read', []));

  Future<void> cancel([dynamic reason]) =>
      js_util.promiseToFuture(js_util.callMethod(this, 'cancel', [reason]));
}

@anonymous
@JS()
@staticInterop
class ReadableStreamReadResult {
  external factory ReadableStreamReadResult(
      {dynamic value, required bool done});
}

extension PropsReadableStreamReadResult on ReadableStreamReadResult {
  dynamic get value => js_util.getProperty(this, 'value');
  set value(dynamic newValue) {
    js_util.setProperty(this, 'value', newValue);
  }

  bool get done => js_util.getProperty(this, 'done');
  set done(bool newValue) {
    js_util.setProperty(this, 'done', newValue);
  }
}

@JS()
@staticInterop
class ReadableStreamGenericReader {
  external factory ReadableStreamGenericReader();
}
