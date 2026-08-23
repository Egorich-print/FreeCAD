package com.freecad.viewer;

import android.app.Activity;
import android.os.Bundle;
import android.view.Choreographer;
import android.view.MotionEvent;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;

/**
 * Thin shell: owns the Surface lifecycle and forwards input to Rust.
 * All CAD logic (STEP import, OCCT, meshing, wgpu) lives in
 * libfreecad_android.so behind these five JNI entry points.
 */
public class MainActivity extends Activity
        implements SurfaceHolder.Callback, Choreographer.FrameCallback {

    static {
        System.loadLibrary("freecad_android");
    }

    private native long nativeInit(Surface surface, byte[] stepBytes);
    private native void nativeDestroy(long handle);
    private native void nativeOrbit(long handle, float dx, float dy);
    private native void nativeZoom(long handle, float factor);
    private native int nativeRender(long handle);
    private native int nativeTap(long handle, float x, float y);

    private SurfaceView surfaceView;
    private long handle = 0;
    private boolean looping = false;

    private float lastX = 0f, lastY = 0f;
    private float lastPinchDistance = -1f;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        surfaceView = new SurfaceView(this);
        surfaceView.getHolder().addCallback(this);
        setContentView(surfaceView);
    }

    private byte[] loadModel() {
        try (java.io.InputStream in = getAssets().open("demo.FCStd")) {
            java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
            byte[] buffer = new byte[16384];
            int read;
            while ((read = in.read(buffer)) > 0) {
                out.write(buffer, 0, read);
            }
            return out.toByteArray();
        } catch (java.io.IOException e) {
            return new byte[0];
        }
    }

    @Override
    public void surfaceCreated(SurfaceHolder holder) {
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        recreateSession(holder);
    }

    @Override
    public void surfaceDestroyed(SurfaceHolder holder) {
        stopLoop();
        dropHandle();
    }

    private void recreateSession(SurfaceHolder holder) {
        stopLoop();
        dropHandle();
        Surface surface = holder.getSurface();
        if (!surface.isValid()) {
            return;
        }
        long started = android.os.SystemClock.uptimeMillis();
        byte[] model = loadModel();
        if (model.length == 0) {
            android.util.Log.e("FreeCAD", "loadModel returned empty payload");
            return;
        }
        handle = nativeInit(surface, model);
        long elapsed = android.os.SystemClock.uptimeMillis() - started;
        // Success is any non-zero handle: on arm64, heap pointers carry a
        // top-byte tag and legitimately look negative when cast to jlong.
        if (handle != 0) {
            android.util.Log.i("FreeCAD",
                "nativeInit(OCCT+mesh+wgpu init) took " + elapsed + " ms, handle=0x"
                + Long.toHexString(handle));
            startLoop();
        } else {
            android.util.Log.e("FreeCAD",
                "nativeInit failed after " + elapsed + " ms (see prior logs)");
        }
    }

    private void dropHandle() {
        if (handle != 0) {
            nativeDestroy(handle);
            handle = 0;
        }
    }

    private void startLoop() {
        if (!looping) {
            looping = true;
            Choreographer.getInstance().postFrameCallback(this);
        }
    }

    private void stopLoop() {
        looping = false;
    }

    @Override
    public void doFrame(long frameTimeNanos) {
        if (!looping || handle == 0) {
            return;
        }
        int result = nativeRender(handle);
        if (result != 0) {
            android.util.Log.w("FreeCAD", "render returned " + result);
        }
        Choreographer.getInstance().postFrameCallback(this);
    }

    @Override
    public boolean onTouchEvent(MotionEvent event) {
        if (handle == 0) {
            return false;
        }
        switch (event.getActionMasked()) {
            case MotionEvent.ACTION_DOWN: {
                lastX = event.getX();
                lastY = event.getY();
                lastPinchDistance = -1f;
                break;
            }
            case MotionEvent.ACTION_MOVE: {
                if (event.getPointerCount() >= 2) {
                    float dx = event.getX(0) - event.getX(1);
                    float dy = event.getY(0) - event.getY(1);
                    float dist = (float) Math.sqrt(dx * dx + dy * dy);
                    if (lastPinchDistance > 0f && dist > 1f) {
                        nativeZoom(handle, lastPinchDistance / dist);
                    }
                    lastPinchDistance = dist;
                } else {
                    float dx = event.getX() - lastX;
                    float dy = event.getY() - lastY;
                    nativeOrbit(handle, dx * 0.006f, dy * 0.006f);
                }
                lastX = event.getX();
                lastY = event.getY();
                break;
            }
            case MotionEvent.ACTION_UP: {
                float dx = event.getX() - lastX;
                float dy = event.getY() - lastY;
                if (handle != 0 && Math.hypot(dx, dy) < 12f) {
                    int face = nativeTap(handle, event.getX(), event.getY());
                    android.util.Log.i("FreeCAD", "tap at (" + (int) event.getX() + "," +
                        (int) event.getY() + ") -> face " + face);
                }
                break;
            }
            case MotionEvent.ACTION_POINTER_UP: {
                lastPinchDistance = -1f;
                break;
            }
            default:
                break;
        }
        return true;
    }
}
