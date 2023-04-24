import 'dart:ui' as ui;

import 'dart:async';

import 'package:flutter/widgets.dart';

import 'image.dart';
import 'menu.dart';

class SystemMenuImage extends MenuImage {
  SystemMenuImage(this.systemImageName);

  final String systemImageName;

  @override
  Widget? asWidget(IconThemeData theme) {
    return null;
  }

  @override
  FutureOr<ui.Image?> asImage(IconThemeData theme, double devicePixelRatio) {
    return null;
  }
}

class ImageProviderMenuImage extends MenuImage {
  ImageProviderMenuImage(this.imageProvider);

  final FutureOr<ui.Image?>? Function(IconThemeData theme, int devicePixelRatio)
      imageProvider;

  @override
  FutureOr<ui.Image?> asImage(IconThemeData theme, double devicePixelRatio) {
    return imageProvider(theme, devicePixelRatio.toInt());
  }

  @override
  Widget? asWidget(IconThemeData theme) {
    return Builder(builder: (context) {
      final image = asImage(theme, MediaQuery.of(context).devicePixelRatio);
      if (image is ui.Image) {
        return RawImage(
          image: image,
        );
      }
      return FutureBuilder<ui.Image?>(
        future: image as Future<ui.Image?>,
        builder: (context, image) {
          if (image.hasData) {
            return RawImage(
              image: image.data!,
            );
          }
          return const SizedBox.shrink();
        },
      );
    });
  }
}

class IconMenuImage extends MenuImage {
  final IconData _icon;

  IconMenuImage(this._icon);

  @override
  FutureOr<ui.Image?> asImage(
    IconThemeData theme,
    double devicePixelRatio,
  ) async {
    assert(theme.size != null, 'IconThemeData.size must not be null!');
    final size = theme.size!;

    final recorder = ui.PictureRecorder();
    final canvas = Canvas(recorder);
    final iconFill = theme.fill;
    final iconWeight = theme.weight;
    final iconGrade = theme.grade;
    final iconOpticalSize = theme.opticalSize;
    final textStyle = TextStyle(
      fontVariations: <ui.FontVariation>[
        if (iconFill != null) ui.FontVariation('FILL', iconFill),
        if (iconWeight != null) ui.FontVariation('wght', iconWeight),
        if (iconGrade != null) ui.FontVariation('GRAD', iconGrade),
        if (iconOpticalSize != null) ui.FontVariation('opsz', iconOpticalSize),
      ],
      inherit: false,
      color: theme.color,
      fontSize: theme.size,
      fontFamily: _icon.fontFamily,
      package: _icon.fontPackage,
      shadows: theme.shadows,
    );
    final paragraphBuilder = ui.ParagraphBuilder(textStyle.getParagraphStyle())
      ..pushStyle(textStyle.getTextStyle())
      ..addText(String.fromCharCode(_icon.codePoint));
    final paragraph = paragraphBuilder.build();
    paragraph.layout(const ui.ParagraphConstraints(width: double.infinity));

    canvas.scale(devicePixelRatio);

    final offset = Offset(size / 2.0 - paragraph.longestLine / 2.0,
        size / 2.0 - paragraph.height / 2.0);

    canvas.drawParagraph(paragraph, offset);
    final picture = recorder.endRecording();
    final image = picture.toImageSync(
      (size * devicePixelRatio).round(),
      (size * devicePixelRatio).round(),
    );
    image.devicePixelRatio = devicePixelRatio;
    return image;
  }

  @override
  Widget? asWidget(IconThemeData theme) {
    return Icon(
      _icon,
      size: theme.size,
      fill: theme.fill,
      weight: theme.weight,
      grade: theme.grade,
      opticalSize: theme.opticalSize,
      color: theme.color,
      shadows: theme.shadows,
    );
  }
}
