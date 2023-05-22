import 'package:flutter/material.dart' show CircularProgressIndicator, Colors;
import 'package:flutter/widgets.dart';
import 'package:super_native_extensions/widget_snapshot.dart';

import 'menu_widget_builder.dart';

class MenuPreviewWidget extends StatefulWidget {
  const MenuPreviewWidget({
    super.key,
    required this.size,
    required this.menuWidgetBuilder,
    this.image,
  });

  final Size size;
  final WidgetSnapshot? image;
  final MobileMenuWidgetBuilder menuWidgetBuilder;

  @override
  State<StatefulWidget> createState() => _MenuPreviewState();
}

class _MenuPreviewState extends State<MenuPreviewWidget> {
  @override
  Widget build(BuildContext context) {
    final Widget child;
    if (widget.image != null) {
      child = DisplayWidgetSnapshot(widget.image!);
    } else {
      child = const Center(
        child: CircularProgressIndicator(
          strokeWidth: 2.0,
          color: Colors.white,
        ),
      );
    }
    return _ScaledWidget(
      size: widget.size,
      child: RepaintBoundary(
        child: widget.menuWidgetBuilder.buildMenuPreviewContainer(
          context,
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 250),
            child: child,
          ),
        ),
      ),
    );
  }
}

class _ScaledWidget extends StatelessWidget {
  const _ScaledWidget({
    required this.size,
    required this.child,
  });

  final Size size;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(builder: (context, constraints) {
      return Stack(
        clipBehavior: Clip.none,
        fit: StackFit.expand,
        children: [
          Positioned(
            left: 0,
            top: 0,
            width: size.width,
            height: size.height,
            child: Transform.scale(
              alignment: Alignment.topLeft,
              scaleX: constraints.maxWidth / size.width,
              scaleY: constraints.maxHeight / size.height,
              child: child,
            ),
          ),
        ],
      );
    });
  }
}
