package com.superlist.super_native_extensions;

import static android.content.ContentResolver.SCHEME_ANDROID_RESOURCE;
import static android.content.ContentResolver.SCHEME_CONTENT;
import static android.content.ContentResolver.SCHEME_FILE;

import android.content.ClipData;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.content.res.AssetFileDescriptor;
import android.net.Uri;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import java.io.ByteArrayOutputStream;
import java.io.Closeable;
import java.io.FileInputStream;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;

// used from JNI
@SuppressWarnings("UnusedDeclaration")
public final class ClipDataHelper {

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
        Object res = _getData(data, index, type, context);
        onData(handle, res);
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
        return coerceToPlainText(item, context);
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

    // Similar to item.coerceToText but prefers text/plain
    private CharSequence coerceToPlainText(ClipData.Item item, Context context) {
        // If this Item has an explicit textual value, simply return that.
        CharSequence text = item.getText();
        if (text != null) {
            return text;
        }

        // If this Item has a URI value, try using that.
        Uri uri = item.getUri();
        if (uri != null) {
            // First see if the URI can be opened as a plain text stream
            // (of any sub-type).  If so, this is the best textual
            // representation for it.
            final ContentResolver resolver = context.getContentResolver();
            AssetFileDescriptor descr = null;
            FileInputStream stream = null;
            InputStreamReader reader = null;
            try {
                try {
                    // Ask for a stream of the desired type.
                    descr = resolver.openTypedAssetFileDescriptor(uri, "text/plain", null);
                } catch (SecurityException e) {
                    Log.w("ClipData", "Failure opening stream", e);
                } catch (FileNotFoundException | RuntimeException e) {
                    // Unable to open content URI as text...  not really an
                    // error, just something to ignore.
                    try {
                        // Retry for other text types
                        descr = resolver.openTypedAssetFileDescriptor(uri, "text/*", null);
                    } catch (SecurityException e_) {
                        Log.w("ClipData", "Failure opening stream", e);
                    } catch (FileNotFoundException | RuntimeException e_) {
                    }
                }
                if (descr != null) {
                    try {
                        stream = descr.createInputStream();
                        reader = new InputStreamReader(stream, "UTF-8");

                        // Got it...  copy the stream into a local string and return it.
                        final StringBuilder builder = new StringBuilder(128);
                        char[] buffer = new char[8192];
                        int len;
                        while ((len = reader.read(buffer)) > 0) {
                            builder.append(buffer, 0, len);
                        }
                        return builder.toString();
                    } catch (IOException e) {
                        // Something bad has happened.
                        Log.w("ClipData", "Failure loading text", e);
                        return e.toString();
                    }
                }
            } finally {
                closeQuietly(descr);
                closeQuietly(stream);
                closeQuietly(reader);
            }

            // If we couldn't open the URI as a stream, use the URI itself as a textual
            // representation (but not for "content", "android.resource" or "file" schemes).
            final String scheme = uri.getScheme();
            if (SCHEME_CONTENT.equals(scheme)
                    || SCHEME_ANDROID_RESOURCE.equals(scheme)
                    || SCHEME_FILE.equals(scheme)) {
                return "";
            }
            return uri.toString();
        }

        // Shouldn't get here, but just in case...
        return "";
    }

    static void closeQuietly(Closeable closeable) {
        try {
            closeable.close();
        } catch (IOException e) {
        }
    }
}
