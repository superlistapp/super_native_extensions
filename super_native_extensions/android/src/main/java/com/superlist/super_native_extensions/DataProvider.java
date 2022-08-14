package com.superlist.super_native_extensions;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.res.AssetFileDescriptor;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.os.CancellationSignal;
import android.os.Handler;
import android.os.Looper;
import android.os.ParcelFileDescriptor;
import android.util.Log;

import java.io.FileNotFoundException;
import java.io.FileOutputStream;
import java.io.OutputStream;

public class DataProvider extends ContentProvider {
    @Override
    public boolean onCreate() {
        return true;
    }

    private class PipeDataWriter implements ContentProvider.PipeDataWriter<byte[]> {
        PipeDataWriter(Uri uri, String mimeType) {
            this.uri = uri;
            this.mimeType = mimeType;
        }

        private Uri uri;
        private String mimeType;

        @Override
        public void writeDataToPipe(ParcelFileDescriptor output, Uri uri, String mimeType,
                                    Bundle opts, byte[] data) {
            if (this.uri != null && this.mimeType != null && data == null) {
                data = getDataForURI(this.uri.toString(), this.mimeType);
            }
            try (OutputStream out = new FileOutputStream(output.getFileDescriptor())) {
                out.write(data);
                out.flush();
            } catch (Exception e) {
                Log.w("flutter", "Failing to write data", e);
            }
        }
    }

    @Override
    public AssetFileDescriptor openTypedAssetFile(Uri uri, String mimeTypeFilter, Bundle opts, CancellationSignal signal) throws FileNotFoundException {
        String uriString = uri.toString();
        String mimeType = getMimeTypeForURI(uriString, mimeTypeFilter);
        // when requesting to open the file on main thread we assume that the stream
        // will also be read on main thread (blocking) so we need the data upfront.
        // this happens for example when ClipData.coerceToText is called by flutter
        ParcelFileDescriptor f;
        if (Looper.getMainLooper().isCurrentThread()) {
            byte[] data = getDataForURI(uriString, mimeType);
            f = openPipeHelper(uri, getType(uri), opts, data, new PipeDataWriter(uri, mimeType));
        } else {
            // resolve data in background thread
            f = openPipeHelper(uri, getType(uri), opts, null, new PipeDataWriter(uri, mimeType));
        }
        return new AssetFileDescriptor(f, 0, -1);
    }

    @Override
    public String[] getStreamTypes(Uri uri, String mimeTypeFilter) {
        return getAllMimeTypesForURI(uri.toString(), mimeTypeFilter);
    }

    private String getMimeTypeForURI(String uriString, String mimeTypeFilter) {
        String[] types = getAllMimeTypesForURI(uriString, mimeTypeFilter);
        if (types != null && types.length > 0) {
            return types[0];
        } else {
            return null;
        }
    }

    private native String[] getAllMimeTypesForURI(String uriString, String mimeTypeFilter);

    private native byte[] getDataForURI(String uriString, String mimeType);

    private final Handler handler = new Handler(Looper.getMainLooper());

    @Override
    public Cursor query(Uri uri, String[] projection, String selection, String[] selectionArgs, String sortOrder) {
        return null;
    }

    @Override
    public String getType(Uri uri) {
        String[] types = getStreamTypes(uri, "*/*");
        return types.length > 0 ? types[0] : null;
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        throw new UnsupportedOperationException();
    }

    @Override
    public int delete(Uri uri, String selection, String[] selectionArgs) {
        throw new UnsupportedOperationException();
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] selectionArgs) {
        throw new UnsupportedOperationException();
    }
}
