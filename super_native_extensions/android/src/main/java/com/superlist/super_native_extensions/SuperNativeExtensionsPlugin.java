package com.superlist.super_native_extensions;

import android.content.Context;
import android.util.Log;
import androidx.annotation.NonNull;

import io.flutter.embedding.android.FlutterView;
import io.flutter.embedding.engine.plugins.FlutterPlugin;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import io.flutter.plugin.common.MethodChannel.MethodCallHandler;
import io.flutter.plugin.common.MethodChannel.Result;

/** SuperNativeExtensionsPlugin */
public class SuperNativeExtensionsPlugin implements FlutterPlugin, MethodCallHandler {

  static final ClipDataUtil clipDataUtil = new ClipDataUtil();
  static final DragDropUtil dragDropUtil = new DragDropUtil();

  private MethodChannel channel;
  private FlutterPluginBinding binding;

  private static boolean nativeInitialized = false;

  @Override
  public void onAttachedToEngine(@NonNull FlutterPluginBinding flutterPluginBinding) {
    try {
      if (!nativeInitialized) {
        init(flutterPluginBinding.getApplicationContext(), clipDataUtil, dragDropUtil);
        nativeInitialized = true;
      }
      channel = new MethodChannel(flutterPluginBinding.getBinaryMessenger(), "super_native_extensions");
      channel.setMethodCallHandler(this);
      binding = flutterPluginBinding;
    } catch (Throwable e) {
      Log.e("flutter", e.toString());
    }
  }

  @Override
  public void onDetachedFromEngine(@NonNull FlutterPluginBinding binding) {
    if (flutterViewId != null) {
      dragDropUtil.unregisterFlutterView(flutterViewId);
    }
  }

  Long flutterViewId = null;

  @Override
  public void onMethodCall(@NonNull MethodCall call, @NonNull Result result) {
    if (call.method.equals("getFlutterView")) {
      if (flutterViewId == null) {
        FlutterView view = DragDropUtil.getFlutterView(binding);
        flutterViewId = dragDropUtil.registerFlutterView(view);
      }
      result.success(flutterViewId);
    } else {
      result.notImplemented();
    }
  }

  public static native void init(Context context, ClipDataUtil clipDataUtil, DragDropUtil dragDropUtil);

  static {
    System.loadLibrary("super_native_extensions");
  }
}
