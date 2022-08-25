package com.superlist.super_native_extensions;

import android.app.Activity;
import android.content.Context;
import android.util.Log;

import androidx.annotation.NonNull;

import io.flutter.embedding.android.FlutterActivity;
import io.flutter.embedding.android.FlutterView;
import io.flutter.embedding.engine.plugins.FlutterPlugin;
import io.flutter.embedding.engine.plugins.activity.ActivityAware;
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import io.flutter.plugin.common.MethodChannel.MethodCallHandler;
import io.flutter.plugin.common.MethodChannel.Result;

/**
 * SuperNativeExtensionsPlugin
 */
public class SuperNativeExtensionsPlugin implements FlutterPlugin, MethodCallHandler, ActivityAware {

    static final ClipDataHelper ClipDataHelper = new ClipDataHelper();
    static final DragDropHelper DragDropHelper = new DragDropHelper();

    private MethodChannel channel;

    private static boolean nativeInitialized = false;

    @Override
    public void onAttachedToEngine(@NonNull FlutterPluginBinding flutterPluginBinding) {
        try {
            if (!nativeInitialized) {
                init(flutterPluginBinding.getApplicationContext(), ClipDataHelper, DragDropHelper);
                nativeInitialized = true;
            }
            channel = new MethodChannel(flutterPluginBinding.getBinaryMessenger(), "super_native_extensions");
            channel.setMethodCallHandler(this);
        } catch (Throwable e) {
            Log.e("flutter", e.toString());
        }
    }

    @Override
    public void onDetachedFromEngine(@NonNull FlutterPluginBinding binding) {
        channel.setMethodCallHandler(null);
    }

    Long flutterViewId = null;
    ActivityPluginBinding activityPluginBinding;

    @Override
    public void onMethodCall(@NonNull MethodCall call, @NonNull Result result) {
        if (call.method.equals("getFlutterView")) {
            if (flutterViewId == null) {
                Activity activity = activityPluginBinding.getActivity();
                FlutterView view = activity.findViewById(FlutterActivity.FLUTTER_VIEW_ID);
                flutterViewId = DragDropHelper.registerFlutterView(view, activity);
            }
            result.success(flutterViewId);
        } else {
            result.notImplemented();
        }
    }

    public static native void init(Context context, ClipDataHelper ClipDataHelper, DragDropHelper DragDropHelper);

    static {
        System.loadLibrary("super_native_extensions");
    }

    @Override
    public void onAttachedToActivity(@NonNull ActivityPluginBinding binding) {
        activityPluginBinding = binding;
    }

    @Override
    public void onDetachedFromActivityForConfigChanges() {
    }

    @Override
    public void onReattachedToActivityForConfigChanges(@NonNull ActivityPluginBinding binding) {
    }

    @Override
    public void onDetachedFromActivity() {
        if (flutterViewId != null) {
            DragDropHelper.unregisterFlutterView(flutterViewId);
            flutterViewId = null;
        }
    }
}
