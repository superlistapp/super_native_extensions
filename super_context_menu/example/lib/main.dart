import 'package:flutter/material.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

void main() async {
  runApp(const MainApp());
}

class Item extends StatelessWidget {
  const Item({
    super.key,
    this.color = Colors.blue,
    required this.child,
    this.padding = const EdgeInsets.all(14),
  });

  final EdgeInsets padding;
  final Color color;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: padding,
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(8),
      ),
      child: DefaultTextStyle(
        style: const TextStyle(color: Colors.white),
        child: child,
      ),
    );
  }
}

class _PageLayout extends StatelessWidget {
  const _PageLayout({
    required this.itemZone,
    required this.dropZone,
  });

  final Widget itemZone;
  final Widget dropZone;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(builder: (context, constraints) {
      return Padding(
        padding: const EdgeInsets.all(16.0),
        child: Flex(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          mainAxisSize: MainAxisSize.max,
          // textDirection: TextDirection.rtl,
          direction: constraints.maxWidth > constraints.maxHeight
              ? Axis.horizontal
              : Axis.vertical,
          children: [
            Expanded(
              flex: 5,
              child: itemZone,
            ),
            const SizedBox(width: 16, height: 16),
            Expanded(
              flex: 2,
              child: dropZone,
            ),
          ],
        ),
      );
    });
  }
}

class Section extends StatelessWidget {
  const Section({
    super.key,
    required this.description,
    required this.child,
  });

  final Widget description;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: Colors.blueGrey.shade50,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.all(16.0),
            child: child,
          ),
          Padding(
            padding: const EdgeInsets.all(16.0).copyWith(top: 0),
            child: description,
          ),
        ],
      ),
    );
  }
}

class _BaseContextMenu extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return ContextMenuWidget(
      child: const Item(
        child: Text('Base Context Menu'),
      ),
      menuProvider: (_) {
        return Menu(
          children: [
            MenuAction(title: 'Menu Item 1', callback: () {}),
            MenuAction(title: 'Menu Item 2', callback: () {}),
            MenuAction(title: 'Menu Item 3', callback: () {}),
            MenuSeparator(),
            Menu(title: 'Submenu', children: [
              MenuAction(title: 'Submenu Item 1', callback: () {}),
              MenuAction(title: 'Submenu Item 2', callback: () {}),
              Menu(title: 'Nested Submenu', children: [
                MenuAction(title: 'Submenu Item 1', callback: () {}),
                MenuAction(title: 'Submenu Item 2', callback: () {}),
              ]),
            ]),
          ],
        );
      },
    );
  }
}

class _BaseContextMenuWithDrag extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return DragItemWidget(
      allowedOperations: () => [DropOperation.copy],
      dragItemProvider: (_) => DragItem(localData: 'LocalDragData'),
      child: DraggableWidget(
        child: ContextMenuWidget(
          child: const Item(
            child: Text('Base Context Menu with Drag'),
          ),
          menuProvider: (_) {
            return Menu(
              children: [
                MenuAction(title: 'Menu Item 1', callback: () {}),
                MenuAction(title: 'Menu Item 2', callback: () {}),
                MenuAction(title: 'Menu Item 3', callback: () {}),
                MenuSeparator(),
                Menu(title: 'Submenu', children: [
                  MenuAction(title: 'Submenu Item 1', callback: () {}),
                  MenuAction(title: 'Submenu Item 2', callback: () {}),
                  Menu(title: 'Submenu', children: [
                    MenuAction(title: 'Submenu Item 1', callback: () {}),
                    MenuAction(title: 'Submenu Item 2', callback: () {}),
                  ]),
                ]),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _DeferredMenuPreview extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return DragItemWidget(
      allowedOperations: () => [DropOperation.copy],
      dragItemProvider: (_) => DragItem(localData: 'LocalDragData'),
      child: DraggableWidget(
        child: ContextMenuWidget(
          child: const Item(
            child: Text('Deferred context menu preview'),
          ),
          deferredPreviewBuilder: (context, child, cancellationToken) {
            return DeferredMenuPreview(
              const Size(200, 150),
              Future.delayed(
                const Duration(seconds: 2),
                () {
                  return const Item(
                    color: Colors.blue,
                    child: Text('Deferred menu preview'),
                  );
                },
              ),
            );
          },
          menuProvider: (_) {
            return Menu(
              children: [
                MenuAction(title: 'Menu Item 1', callback: () {}),
                MenuAction(title: 'Menu Item 2', callback: () {}),
                MenuAction(title: 'Menu Item 3', callback: () {}),
                MenuSeparator(),
                Menu(title: 'Submenu', children: [
                  MenuAction(title: 'Submenu Item 1', callback: () {}),
                  MenuAction(title: 'Submenu Item 2', callback: () {}),
                  Menu(title: 'Submenu', children: [
                    MenuAction(title: 'Submenu Item 1', callback: () {}),
                    MenuAction(title: 'Submenu Item 2', callback: () {}),
                  ]),
                ]),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _DropZone extends StatefulWidget {
  const _DropZone();

  @override
  State<StatefulWidget> createState() => _DropZoneState();
}

class _DropZoneState extends State<_DropZone> {
  bool _inside = false;

  @override
  Widget build(BuildContext context) {
    return DropRegion(
      formats: const [], // local data only
      onDropOver: (event) {
        return DropOperation.copy;
      },
      onDropEnter: (_) {
        setState(() {
          _inside = true;
        });
      },
      onDropLeave: (_) {
        setState(() {
          _inside = false;
        });
      },
      onPerformDrop: (event) async {},
      child: Container(
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(16),
          border: Border.all(width: 1, color: Colors.blueGrey.shade300),
          color: _inside ? Colors.blue.shade200 : Colors.transparent,
        ),
        alignment: Alignment.center,
        child: const Text('Drop Zone'),
      ),
    );
  }
}

class _ComplexContextMenu extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return DragItemWidget(
      allowedOperations: () => [DropOperation.copy],
      dragItemProvider: (_) => DragItem(localData: 'LocalDragData'),
      dragBuilder: (context, child) => const Item(
        color: Colors.teal,
        child: Text('Custom Drag preview'),
      ),
      child: DraggableWidget(
        child: ContextMenuWidget(
          liftBuilder: (context, child) {
            return Item(
              color: Colors.red,
              child: (child as Item).child,
            );
          },
          previewBuilder: (context, child) {
            return Item(
              color: Colors.amber.shade600,
              padding: const EdgeInsets.all(24),
              child: const Text('Custom menu preview'),
            );
          },
          child: const Item(
            child: Text('Complex Context Menu'),
          ),
          menuProvider: (_) {
            return Menu(
              children: [
                MenuAction(
                  image: MenuImage.icon(Icons.access_time),
                  title: 'Menu Item 1',
                  callback: () {},
                ),
                MenuAction(
                  title: 'Disabled Menu Item',
                  image: MenuImage.icon(Icons.replay_outlined),
                  attributes: const MenuActionAttributes(disabled: true),
                  callback: () {},
                ),
                MenuAction(
                  title: 'Destructive Menu Item',
                  image: MenuImage.icon(Icons.delete),
                  attributes: const MenuActionAttributes(destructive: true),
                  callback: () {},
                ),
                MenuSeparator(),
                Menu(title: 'Submenu', children: [
                  MenuAction(title: 'Submenu Item 1', callback: () {}),
                  MenuAction(title: 'Submenu Item 2', callback: () {}),
                ]),
                Menu(title: 'Deferred Item Example', children: [
                  MenuAction(title: 'Leading Item', callback: () {}),
                  DeferredMenuElement((_) async {
                    await Future.delayed(const Duration(seconds: 2));
                    return [
                      MenuSeparator(),
                      MenuAction(title: 'Lazily Loaded Item', callback: () {}),
                      Menu(title: 'Lazily Loaded Submenu', children: [
                        MenuAction(title: 'Submenu Item 1', callback: () {}),
                        MenuAction(title: 'Submenu Item 2', callback: () {}),
                      ]),
                      MenuSeparator(),
                    ];
                  }),
                  MenuAction(title: 'Trailing Item', callback: () {}),
                ]),
                MenuSeparator(),
                MenuAction(
                  title: 'Checked Menu Item',
                  state: MenuActionState.checkOn,
                  callback: () {},
                ),
                MenuAction(
                  title: 'Menu Item in Mixed State',
                  state: MenuActionState.checkMixed,
                  callback: () {},
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _Separator extends StatelessWidget {
  const _Separator();

  @override
  Widget build(BuildContext context) {
    return const SizedBox(height: 16);
  }
}

class MainApp extends StatelessWidget {
  const MainApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        body: SafeArea(
          child: _PageLayout(
            itemZone: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  Section(
                    description:
                        const Text('Base context menu, without drag & drop.'),
                    child: _BaseContextMenu(),
                  ),
                  Section(
                    description:
                        const Text('Base context menu, with drag & drop.'),
                    child: _BaseContextMenuWithDrag(),
                  ),
                  Section(
                    description: const Text(
                        'Complex context menu, with custom lift, preview and drag builders (mobile only).'),
                    child: _ComplexContextMenu(),
                  ),
                  Section(
                    description:
                        const Text('Deferred menu preview (mobile only).'),
                    child: _DeferredMenuPreview(),
                  ),
                ].intersperse(const _Separator()).toList(growable: false),
              ),
            ),
            dropZone: const _DropZone(),
          ),
        ),
      ),
    );
  }
}

extension IntersperseExtensions<T> on Iterable<T> {
  Iterable<T> intersperse(T element) sync* {
    final iterator = this.iterator;
    if (iterator.moveNext()) {
      yield iterator.current;
      while (iterator.moveNext()) {
        yield element;
        yield iterator.current;
      }
    }
  }
}
