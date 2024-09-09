@TestOn('chrome')

import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import 'package:flutter_test/flutter_test.dart';
import 'package:super_native_extensions/src/web/drop.dart';
import 'package:web/web.dart' as web;

void main() {
  test('should have no listeners by default', () async {
    expect(
      web.document.getProperty(DropContextImpl.listenersProperty.toJS),
      isNull,
    );
  });
}
