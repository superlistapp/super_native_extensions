import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';

import '../shadow.dart';
import '../widget_snapshot/display_widget_snapshot.dart';
import '../widget_snapshot/widget_snapshot.dart';
import '../image_data.dart';

class ShadowImage extends StatelessWidget {
  const ShadowImage({
    super.key,
    required this.image,
    required this.shadowRadius,
    required this.shadowOpacity,
  });

  final WidgetSnapshot image;
  final int shadowRadius;
  final double shadowOpacity;

  @override
  Widget build(BuildContext context) {
    if (image.isImage) {
      return _ShadowImage(
        image: image.image,
        shadowRadius: shadowRadius,
        shadowOpacity: shadowOpacity,
      );
    } else {
      return DisplayWidgetSnapshot(image);
    }
  }
}

class _ShadowImage extends StatefulWidget {
  const _ShadowImage({
    required this.image,
    required this.shadowRadius,
    required this.shadowOpacity,
  });

  final ui.Image image;
  final int shadowRadius;
  final double shadowOpacity;

  @override
  State<StatefulWidget> createState() => _ShadowImageState();
}

const kShadowRadius = 14;

class _ShadowImageState extends State<_ShadowImage> {
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!_didGenerateShadow) {
      _didGenerateShadow = true;
      _generateShadow();
    }
  }

  void _generateShadow() async {
    final devicePixelRatio = widget.image.devicePixelRatio ?? 1.0;

    final rect = Rect.fromLTWH(
      0,
      0,
      widget.image.width / devicePixelRatio,
      widget.image.height / devicePixelRatio,
    );
    final targetedImageData =
        await (TargetedWidgetSnapshot(WidgetSnapshot.image(widget.image), rect))
            .intoRaw();
    final shadow = targetedImageData.withShadowOnly(kShadowRadius);
    final shadowImage = await shadow.imageData.toImage();
    if (!mounted) {
      return;
    }
    setState(() {
      _shadowImage = shadowImage;
    });
  }

  bool _didGenerateShadow = false;

  ui.Image? _shadowImage;

  @override
  void dispose() {
    super.dispose();
    _shadowImage?.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(builder: (context, constrains) {
      if (_shadowImage == null) {
        return RawImage(
          image: widget.image,
          fit: BoxFit.fill,
        );
      }
      final devicePixelRatio = MediaQuery.of(context).devicePixelRatio;
      final imageWidth = widget.image.width / devicePixelRatio;
      final imageHeight = widget.image.height / devicePixelRatio;
      final ratioX = constrains.maxWidth / imageWidth;
      final ratioY = constrains.maxHeight / imageHeight;
      return Stack(
        clipBehavior: Clip.none,
        children: [
          Positioned(
            left: -widget.shadowRadius * ratioX,
            right: -widget.shadowRadius * ratioX,
            top: -widget.shadowRadius * ratioY,
            bottom: -widget.shadowRadius * ratioY,
            child: Opacity(
              opacity: widget.shadowOpacity,
              child: RawImage(
                image: _shadowImage!,
                fit: BoxFit.fill,
              ),
            ),
          ),
          Positioned.fill(
            child: RawImage(
              image: widget.image,
              fit: BoxFit.fill,
            ),
          ),
        ],
      );
    });
  }
}
