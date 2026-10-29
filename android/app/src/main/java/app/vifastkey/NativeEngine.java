package app.vifastkey;

/** Cầu nối JNI tới Rust engine. Action trả về dạng JSON. */
public final class NativeEngine {
    static {
        System.loadLibrary("vifastkey_jni");
    }

    private NativeEngine() {}

    public static native long create(String configJson);

    public static native void free(long handle);

    public static native String setConfig(long handle, String configJson);

    public static native String process(long handle, String key, boolean ctrl,
                                        boolean alt, boolean meta);

    public static native String display(long handle);

    public static native String suggest(long handle, String prefix);

    public static native void setSurrounding(long handle, String text);

    public static native String focus(long handle, String app);

    public static native String cursorMoved(long handle);

    public static native String toggle(long handle);

    public static native void reset(long handle);

    public static native boolean isEnabled(long handle);

    public static native void setField(long handle, int field);
}
