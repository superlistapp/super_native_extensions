package com.superlist.super_native_extensions;

import androidx.annotation.Keep;

import android.content.ClipData;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.Point;
import android.os.SystemClock;
import android.util.Log;
import android.view.DragEvent;
import android.view.InputDevice;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewParent;

// Wrap drag sessionId in typed object so that we can safely ignore possible local data
// from sessions not created by super_native_extensions.
class SessionId {
    SessionId(long sessionId) {
        this.sessionId = sessionId;
    }

    final long sessionId;
}

// used from JNI
@Keep
@SuppressWarnings("UnusedDeclaration")
public class DragDropHelper {
    public static native boolean onDrag(DragEvent event, long dropHandlerId);

    static class DragShadowBuilder extends View.DragShadowBuilder {
        DragShadowBuilder(Bitmap bitmap, Point touchPoint) {
            this.bitmap = bitmap;
            this.touchPoint = touchPoint;
        }

        private final Bitmap bitmap;
        private final Point touchPoint;

        @Override
        public void onProvideShadowMetrics(Point outShadowSize, Point outShadowTouchPoint) {
            outShadowSize.set(bitmap.getWidth() + 20, bitmap.getHeight() + 20);
            outShadowTouchPoint.set(touchPoint.x, touchPoint.y);
        }

        @Override
        public void onDrawShadow(Canvas canvas) {
            Paint shadowPaint = new Paint();
            canvas.drawBitmap(bitmap, 10, 10, shadowPaint);
        }
    }

    native void updateLastTouchPoint(ViewParent rootView, MotionEvent event);

    void startDrag(View view, long dragSessionId, ClipData clipData, Bitmap bitmap,
                   int touchPointX, int touchPointY, int lastTouchEventX, int lastTouchEventY) {
        final int DRAG_FLAG_GLOBAL = 1 << 8;
        final int DRAG_FLAG_GLOBAL_URI_READ = Intent.FLAG_GRANT_READ_URI_PERMISSION;
        final int flags = clipData != null ? DRAG_FLAG_GLOBAL | DRAG_FLAG_GLOBAL_URI_READ : 0;
        if (view != null) {
            ViewParent parent = view.getParent();
            while (parent.getParent() != null) {
                parent = parent.getParent();
            }
            int viewLocation[] = new int[2];
            view.getLocationOnScreen(viewLocation);
            MotionEvent event = MotionEvent.obtain(SystemClock.uptimeMillis(), SystemClock.uptimeMillis(),
                                                   MotionEvent.ACTION_MOVE,
                                                   lastTouchEventX + viewLocation[0],
                                                   lastTouchEventY + viewLocation[1],
                                                   0);
            event.setSource(InputDevice.SOURCE_CLASS_POINTER);
            // Simulate touch event before starting drag which will be the return position
            // on failed drop
            updateLastTouchPoint(parent, event);
            view.startDrag(clipData,
                    new DragShadowBuilder(bitmap, new Point(touchPointX, touchPointY)), new SessionId(dragSessionId),
                    flags
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

    void registerDropHandler(View view, long handlerId) {
        if (view != null) {
            view.setOnDragListener((v, event) -> onDrag(event, handlerId));
        }
    }
}
