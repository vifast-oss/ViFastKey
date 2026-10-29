package app.vifastkey;

import android.inputmethodservice.InputMethodService;
import android.os.SystemClock;
import android.text.InputType;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;

import org.json.JSONObject;

/** IME tối giản: bàn phím QWERTY + engine Rust qua JNI. */
public final class ViFastKeyService extends InputMethodService {
    private long handle;
    private LinearLayout keyboard;

    @Override
    public void onCreate() {
        super.onCreate();
        handle = NativeEngine.create();
    }

    @Override
    public View onCreateInputView() {
        if (keyboard == null) {
            keyboard = buildKeyboard();
        }
        return keyboard;
    }

    @Override
    public void onStartInput(EditorInfo attribute, boolean restarting) {
        super.onStartInput(attribute, restarting);
        if (handle == 0) {
            handle = NativeEngine.create();
        }
        NativeEngine.setField(handle, fieldKind(attribute.inputType));
        NativeEngine.focus(handle, attribute.packageName == null ? "" : attribute.packageName);
    }

    @Override
    public void onFinishInput() {
        if (handle != 0) {
            NativeEngine.reset(handle);
        }
        InputConnection ic = getCurrentInputConnection();
        if (ic != null) {
            ic.finishComposingText();
        }
        super.onFinishInput();
    }

    @Override
    public void onDestroy() {
        if (handle != 0) {
            NativeEngine.free(handle);
            handle = 0;
        }
        super.onDestroy();
    }

    private int fieldKind(int inputType) {
        int cls = inputType & InputType.TYPE_MASK_CLASS;
        int variation = inputType & InputType.TYPE_MASK_VARIATION;
        if (cls == InputType.TYPE_CLASS_TEXT) {
            if (variation == InputType.TYPE_TEXT_VARIATION_PASSWORD
                    || variation == InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD
                    || variation == InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD) {
                return 1;
            }
            if (variation == InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS
                    || variation == InputType.TYPE_TEXT_VARIATION_WEB_EMAIL_ADDRESS) {
                return 3;
            }
            if (variation == InputType.TYPE_TEXT_VARIATION_URI) {
                return 4;
            }
        }
        if (cls == InputType.TYPE_CLASS_NUMBER || cls == InputType.TYPE_CLASS_PHONE) {
            return 2;
        }
        return 0;
    }

    private LinearLayout buildKeyboard() {
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(4, 6, 4, 10);
        root.setBackgroundColor(0xFFE8EAED);

        addRow(root, "q w e r t y u i o p");
        addRow(root, "a s d f g h j k l");
        addRow(root, "z x c v b n m");
        addRow(root, "1 2 3 4 5 6 7 8 9 0");

        LinearLayout bottom = new LinearLayout(this);
        bottom.setOrientation(LinearLayout.HORIZONTAL);
        bottom.setGravity(Gravity.CENTER);
        addKey(bottom, "VI", "VI", 90);
        addKey(bottom, "␣", " ", 0);
        addKey(bottom, "⌫", "Backspace", 80);
        addKey(bottom, "⏎", "Enter", 80);
        LinearLayout.LayoutParams bottomParams = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        root.addView(bottom, bottomParams);
        return root;
    }

    private void addRow(LinearLayout root, String keys) {
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.CENTER);
        for (String key : keys.split(" ")) {
            addKey(row, key, key, 0);
        }
        root.addView(row, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
    }

    private void addKey(LinearLayout row, String label, String key, int widthDp) {
        Button button = new Button(this);
        button.setText(label);
        button.setTextSize(17);
        button.setAllCaps(false);
        button.setMinWidth(0);
        button.setMinimumWidth(0);
        button.setPadding(0, 0, 0, 0);
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
                widthDp > 0 ? dp(widthDp) : 0,
                dp(52), widthDp > 0 ? 0 : 1f);
        params.setMargins(dp(2), dp(2), dp(2), dp(2));
        button.setLayoutParams(params);
        button.setOnClickListener(v -> press(key));
        row.addView(button);
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    private void press(String key) {
        if (handle == 0) {
            return;
        }
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) {
            return;
        }
        if ("VI".equals(key)) {
            String action = NativeEngine.toggle(handle);
            apply(ic, action, "");
            return;
        }
        CharSequence before = ic.getTextBeforeCursor(64, 0);
        NativeEngine.setSurrounding(handle, before == null ? "" : before.toString());
        String action = NativeEngine.process(handle, key, false, false, false);
        apply(ic, action, key);
    }

    private void apply(InputConnection ic, String json, String originalKey) {
        if (json == null) {
            return;
        }
        try {
            JSONObject action = new JSONObject(json);
            String kind = action.optString("kind");
            String text = action.optString("text", "");
            boolean pass = action.optBoolean("pass", false);
            switch (kind) {
                case "preedit":
                    if (text.isEmpty()) {
                        ic.finishComposingText();
                    } else {
                        ic.setComposingText(text, 1);
                    }
                    break;
                case "commit":
                    ic.commitText(text, 1);
                    ic.finishComposingText();
                    if (pass) {
                        passOriginal(ic, originalKey);
                    }
                    break;
                case "replace":
                    int delete = action.optInt("delete", 0);
                    if (delete > 0) {
                        ic.deleteSurroundingText(delete, 0);
                    }
                    ic.commitText(action.optString("insert", ""), 1);
                    ic.finishComposingText();
                    if (pass) {
                        passOriginal(ic, originalKey);
                    }
                    break;
                case "pass":
                default:
                    passOriginal(ic, originalKey);
                    break;
            }
        } catch (Exception ignored) {
            passOriginal(ic, originalKey);
        }
    }

    private void passOriginal(InputConnection ic, String key) {
        if (key == null || key.isEmpty()) {
            return;
        }
        switch (key) {
            case "Backspace":
                ic.deleteSurroundingText(1, 0);
                return;
            case "Enter":
                long now = SystemClock.uptimeMillis();
                ic.sendKeyEvent(new KeyEvent(now, now, KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ENTER, 0));
                ic.sendKeyEvent(new KeyEvent(now, now, KeyEvent.ACTION_UP, KeyEvent.KEYCODE_ENTER, 0));
                return;
            case "Tab":
                ic.commitText("\t", 1);
                return;
            default:
                if (key.length() == 1) {
                    ic.commitText(key, 1);
                }
        }
    }
}
