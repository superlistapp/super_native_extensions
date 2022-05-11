package com.superlist.super_native_extensions;

import android.content.ClipData;
import android.content.Context;
import android.content.res.AssetFileDescriptor;
import android.net.Uri;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import java.io.ByteArrayOutputStream;
import java.io.FileInputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;

// used from JNI
@SuppressWarnings("UnusedDeclaration")
public final class ClipDataUtil {

    static final String typeTextPlain = "text/plain";
    static final String typeTextHtml = "text/html";
    static final String typeUriList = "text/uri-list";

    public String[] getTypes(ClipData data, int index, Context context) {
        if (index < data.getItemCount()) {
            return getTypes(data.getItemAt(index), context);
        } else {
            return null;
        }
    }

    private final Handler handler = new Handler(Looper.getMainLooper());

    public void getData(ClipData data, int index, String type, Context context, int handle) {
        // It is likely that getData is invoked through JNI, which means current code is executed
        // through native looper polling. Getting the stream might require manually loop polling
        // inside source.rs, which doesn't work correctly from within native looper polling code.
        // To get around this we reschedule and get the data on next RunLoop turn.
        handler.post(() -> {
            Object res = _getData(data, index, type, context);
            onData(handle, res);
        });
    }

    native void onData(int handle, Object data);

    public Object _getData(ClipData data, int index, String type, Context context) {
        if (index < data.getItemCount()) {
            ClipData.Item item = data.getItemAt(index);
            switch (type) {
                case typeTextHtml:
                    return getHtml(item, context);
                case typeTextPlain:
                    return getText(item, context);
                case typeUriList:
                    return getUri(item, context);
                default:
                    return getData(item, type, context);
            }
        } else {
            return null;
        }
    }

    String[] getTypes(ClipData.Item item, Context context) {
        ArrayList<String> res = new ArrayList<>();
        if (item.getHtmlText() != null) {
            res.add(typeTextHtml);
        }
        if (item.getText() != null) {
            res.add(typeTextPlain);
        }
        if (item.getUri() != null) {
            String[] types = context.getContentResolver().getStreamTypes(item.getUri(), "*/*");
            if (types != null) {
                for (String type : types) {
                    if (!res.contains(type)) {
                        res.add(type);
                    }
                }
            } else {
                res.add(typeUriList);
            }
        }
        return res.toArray(new String[0]);
    }

    CharSequence getText(ClipData.Item item, Context context) {
        return item.coerceToText(context);
    }

    CharSequence getHtml(ClipData.Item item, Context context) {
        return item.coerceToHtmlText(context);
    }

    CharSequence getUri(ClipData.Item item, Context context) {
        // first try if we can get URI data in case the URI we have (if any) is
        // a content URI
        try {
            byte[] data = getData(item, typeUriList, context);
            if (data != null) {
                return new String(data, StandardCharsets.UTF_8);
            } else {
                Uri uri = item.getUri();
                if (uri != null) {
                    return uri.toString();
                } else {
                    return null;
                }
            }
        } catch (Exception e) {
            Log.w("flutter", "Failed to decode Uri", e);
            return null;
        }
    }

    byte[] getData(ClipData.Item item, String type, Context context) {
        Uri uri = item.getUri();
        if (uri == null) {
            return null;
        }
        AssetFileDescriptor descriptor;
        try {
            descriptor = context.getContentResolver().openTypedAssetFileDescriptor(uri, type, null);
            if (descriptor == null) {
                return null;
            }
        } catch (Exception e) {
            Log.w("flutter", "Failed to open resource stream", e);
            return null;
        }
        try {
            FileInputStream stream = descriptor.createInputStream();
            ByteArrayOutputStream output = new ByteArrayOutputStream();

            byte[] buffer = new byte[8192];
            int numRead;
            while ((numRead = stream.read(buffer)) > 0) {
                output.write(buffer, 0, numRead);
            }
            try {
                stream.close();
            } catch (IOException ignored) {
            }
            return output.toByteArray();
        } catch (IOException e) {
            Log.w("flutter", "Failed loading clip data", e);
            return null;
        } finally {
            try {
                descriptor.close();
            } catch (IOException e) {
                // Java is annoying
            }
        }
    }
}
