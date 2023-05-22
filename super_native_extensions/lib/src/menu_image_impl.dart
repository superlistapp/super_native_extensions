import 'dart:ui' as ui;
import 'dart:async';
import 'package:flutter/widgets.dart';

import 'menu_model.dart';
import 'widget_snapshot/widget_snapshot.dart';

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
      return _ImageWidget(
        imageProvider: () => asImage(
          theme,
          MediaQuery.of(context).devicePixelRatio,
        ),
      );
    });
  }
}

class _ImageWidget extends StatefulWidget {
  final FutureOr<ui.Image?> Function() imageProvider;

  const _ImageWidget({
    required this.imageProvider,
  });

  @override
  State<StatefulWidget> createState() => _ImageWidgetState();
}

class _ImageWidgetState extends State<_ImageWidget> {
  ui.Image? _image;

  @override
  void initState() {
    super.initState();
    final image = widget.imageProvider();
    if (image is Future<ui.Image?>) {
      image.then((value) {
        if (mounted) {
          setState(() {
            _image = value;
          });
        } else {
          value?.dispose();
        }
      });
    } else {
      _image = image;
    }
  }

  @override
  void dispose() {
    super.dispose();
    _image?.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_image != null) {
      return RawImage(
        image: _image!,
      );
    }
    return const SizedBox.shrink();
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
