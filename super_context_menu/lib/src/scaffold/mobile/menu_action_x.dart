import 'package:flutter/cupertino.dart';
import 'package:super_native_extensions/raw_menu.dart';

class MenuActionX extends MenuAction {
  MenuActionX({
    required super.callback,
    super.title,
    super.image,
    this.suffixIcon,
  });

  final Widget? suffixIcon;
}
