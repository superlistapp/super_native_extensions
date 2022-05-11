package com.superlist.super_native_extensions;

import android.content.Context;
import android.util.Log;
import androidx.annotation.NonNull;
import io.flutter.embedding.engine.plugins.FlutterPlugin;

/** SuperNativeExtensionsPlugin */
public class SuperNativeExtensionsPlugin implements FlutterPlugin {

  final ClipDataUtil util = new ClipDataUtil();

  @Override
  public void onAttachedToEngine(@NonNull FlutterPluginBinding flutterPluginBinding) {
    try {
      init(flutterPluginBinding.getApplicationContext(), util);
    } catch (Throwable e) {
      Log.e("flutter", e.toString());
    }
  }

  @Override
  public void onDetachedFromEngine(@NonNull FlutterPluginBinding binding) {
  }

  public static native void init(Context context, ClipDataUtil clipDataUtil);

  static {
    System.loadLibrary("super_native_extensions");
  }
}
