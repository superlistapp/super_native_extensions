package com.superlist.super_native_extensions;

import android.content.Context;
import android.util.Log;

import androidx.annotation.NonNull;

import io.flutter.embedding.engine.plugins.FlutterPlugin;

/**
 * SuperNativeExtensionsPlugin
 */
public class SuperNativeExtensionsPlugin implements FlutterPlugin {

    static final ClipDataHelper ClipDataHelper = new ClipDataHelper();
    static final DragDropHelper DragDropHelper = new DragDropHelper();

    private static boolean nativeInitialized = false;

    @Override
    public void onAttachedToEngine(@NonNull FlutterPluginBinding flutterPluginBinding) {
        try {
            if (!nativeInitialized) {
                init(flutterPluginBinding.getApplicationContext(),
                        getClass().getClassLoader(), ClipDataHelper, DragDropHelper);
                nativeInitialized = true;
            }
        } catch (Throwable e) {
            Log.e("flutter", e.toString());
        }
    }

    @Override
    public void onDetachedFromEngine(@NonNull FlutterPluginBinding binding) {
    }

    public static native void init(Context context, ClassLoader pluginClassLoader,
                                   ClipDataHelper ClipDataHelper, DragDropHelper DragDropHelper);

    static {
        System.loadLibrary("super_native_extensions");
    }
}
