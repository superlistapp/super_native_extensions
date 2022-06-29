package com.superlist.super_native_extensions;

import android.content.ClipData;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Point;
import android.util.Log;
import android.view.DragEvent;
import android.view.View;

import java.util.HashMap;
import java.util.Map;

import io.flutter.embedding.android.FlutterView;
import io.flutter.embedding.engine.plugins.FlutterPlugin;

// used from JNI
@SuppressWarnings("UnusedDeclaration")
public class DragDropUtil {
    public static native FlutterView getFlutterView(FlutterPlugin.FlutterPluginBinding binding);

    public static native boolean onDrag(DragEvent event, long dropHandlerId);

    private long _nextId = 1;
    private Map<Long, FlutterView> flutterViewMap = new HashMap<Long, FlutterView>();

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

        private Bitmap bitmap;
        private Point touchPoint;

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

    void startDrag(long viewId, ClipData clipData, Bitmap bitmap, int touchPointX, int touchPointY) {
        FlutterView view = flutterViewMap.get(viewId);
        if (view != null) {
            view.startDrag(clipData,
                    new DragShadowBuilder(bitmap, new Point(touchPointX, touchPointY)), null,
                    View.DRAG_FLAG_GLOBAL | View.DRAG_FLAG_GLOBAL_URI_READ
            );
        }
    }

    void registerDropHandler(long viewId, long handlerId) {
        FlutterView view = flutterViewMap.get(viewId);
        if (view != null) {
            view.setOnDragListener(new View.OnDragListener() {
                @Override
                public boolean onDrag(View v, DragEvent event) {
                    Log.i("flutter", "DragEvent " + event);
                    return DragDropUtil.onDrag(event, handlerId);
                }
            });
        }
    }


}
