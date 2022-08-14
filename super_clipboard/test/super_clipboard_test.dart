import 'dart:convert';
import 'package:flutter_test/flutter_test.dart';
import 'package:super_clipboard/src/format_conversions.dart';

void main() {
  test('test windows html', () async {
    // ignore: prefer_function_declarations_over_variables
    final in1 = (format) async => base64.decode(
        'VmVyc2lvbjowLjkNClN0YXJ0SFRNTDowMDAwMDE1NQ0KRW5kSFRNTDowMDAwMDIzMA0KU3RhcnRGcmFnbWVudDowMDAwMDE4OQ0KRW5kRnJhZ21lbnQ6MDAwMDAxOTQNClNvdXJjZVVSTDpodHRwczovL3d3dy53M3NjaG9vbHMuY29tL2h0bWwvaHRtbF90YWJsZXMuYXNwDQo8aHRtbD48Ym9keT4NCjwhLS1TdGFydEZyYWdtZW50LS0+SFRNTCA8IS0tRW5kRnJhZ21lbnQtLT4NCjwvYm9keT4NCjwvaHRtbD4A');
    final decoded = await windowsHtmlFromSystem(in1, cfHtml);
    expect(decoded, 'HTML ');

    final encoded1 = windowsHtmlToSystem('Another\nTest', cfHtml);

    expect(
        encoded1,
        base64.decode(
            'VmVyc2lvbjowLjkNClN0YXJ0SFRNTDowMDAwMDA5Nw0KRW5kSFRNTDowMDAwMDE4MQ0KU3RhcnRGcmFnbWVudDowMDAwMDEzMg0KRW5kRnJhZ21lbnQ6MDAwMDAxNDUNCjxodG1sPjxib2R5Pg0KPCEtLVN0YXJ0RnJhZ21lbnQgLS0+QW5vdGhlcg0KVGVzdDwhLS1FbmRGcmFnbWVudC0tPg0KPC9ib2R5Pg0KPC9odG1sPgA='));

    final decoded2 = await windowsHtmlFromSystem((_) async => encoded1, cfHtml);
    expect(decoded2, 'Another\r\nTest');
  });
}
