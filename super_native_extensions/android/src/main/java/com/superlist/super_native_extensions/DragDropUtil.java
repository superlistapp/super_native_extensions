package com.superlist.super_native_extensions;

import android.content.ClipData;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Point;
import android.util.Log;
import android.view.DragEvent;
import android.view.View;

import java.util.HashMap;
import java.util.Map;

import io.flutter.embedding.android.FlutterView;

// Wrap drag sessionId in typed object so that we can safely ignore drag possible local data
// from session not created by super_native_extensions.
class SessionId {
    SessionId(long sessionId) {
        this.sessionId = sessionId;
    }

    final long sessionId;
}

// used from JNI
@SuppressWarnings("UnusedDeclaration")
public class DragDropUtil {
    public static native boolean onDrag(DragEvent event, long dropHandlerId);

    private long _nextId = 1;
    private final Map<Long, FlutterView> flutterViewMap = new HashMap<>();

    long registerFlutterView(FlutterView view) {
        long id = _nextId++;
        flutterViewMap.put(id, view);
        return id;
    }

    void unregisterFlutterView(long id) {
        flutterViewMap.remove(id);
    }

    static class DragShadowBuilder extends View.DragShadowBuilder {
        DragShadowBuilder(Bitmap bitmap, Point touchPoint) {
            this.bitmap = bitmap;
            this.touchPoint = touchPoint;
        }

        private final Bitmap bitmap;
        private final Point touchPoint;

        @Override
        public void onProvideShadowMetrics(Point outShadowSize, Point outShadowTouchPoint) {
            outShadowSize.set(bitmap.getWidth(), bitmap.getHeight());
            outShadowTouchPoint.set(touchPoint.x, touchPoint.y);
        }

        @Override
        public void onDrawShadow(Canvas canvas) {
            canvas.drawBitmap(bitmap, 0, 0, null);
        }
    }

    void startDrag(long viewId, long dragSessionId, ClipData clipData, Bitmap bitmap, int touchPointX, int touchPointY) {
        FlutterView view = flutterViewMap.get(viewId);
        final int DRAG_FLAG_GLOBAL = 1 << 8;
        final int DRAG_FLAG_GLOBAL_URI_READ = Intent.FLAG_GRANT_READ_URI_PERMISSION;
        if (view != null) {
            view.startDrag(clipData,
                    new DragShadowBuilder(bitmap, new Point(touchPointX, touchPointY)), new SessionId(dragSessionId),
                    DRAG_FLAG_GLOBAL | DRAG_FLAG_GLOBAL_URI_READ
            );
        }
    }

    Long getSessionId(DragEvent event) {
        Object localState = event.getLocalState();
        if (localState instanceof SessionId) {
            return ((SessionId) localState).sessionId;
        } else {
            return null;
        }
    }

    void registerDropHandler(long viewId, long handlerId) {
        FlutterView view = flutterViewMap.get(viewId);
        if (view != null) {
            view.setOnDragListener((v, event) -> {
                Log.i("flutter", "DragEvent " + event);
                return onDrag(event, handlerId);
            });
        }
    }
}
