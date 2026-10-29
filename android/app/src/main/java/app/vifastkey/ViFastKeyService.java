package app.vifastkey;

import android.content.Intent;
import android.content.SharedPreferences;
import android.inputmethodservice.InputMethodService;
import android.os.SystemClock;
import android.text.InputType;
import android.view.KeyEvent;
import android.view.View;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.ArrayList;
import java.util.List;

/** IME Android dùng engine Rust qua JNI + bàn phím QWERTY đầy đủ. */
public final class ViFastKeyService extends InputMethodService {
    private static final long SELECTION_GUARD_MS = 120;
    private long handle;
    private ViFastKeyKeyboard keyboard;
    private boolean applying;
    private long lastAppliedAt;
    private int fieldKind = 0;
    private SharedPreferences.OnSharedPreferenceChangeListener prefListener;

    @Override
    public void onCreate() {
        super.onCreate();
        handle = NativeEngine.create(Prefs.configJson(this));
        new Thread(() -> NativeEngine.suggest(0L, "a"), "vfk-suggest-warmup").start();
        prefListener = (sharedPreferences, key) -> {
            if (handle != 0) {
                String action = NativeEngine.setConfig(handle, Prefs.configJson(this));
                applyProjected(action, "", snapshot(getCurrentInputConnection()));
                syncKeyboardState();
                updateSuggestions();
            }
        };
        Prefs.shared(this).registerOnSharedPreferenceChangeListener(prefListener);
    }

    @Override
    public View onCreateInputView() {
        keyboard = new ViFastKeyKeyboard(this, new ViFastKeyKeyboard.Listener() {
            @Override public void onKey(String key) { press(key); }
            @Override public void onDirect(String text) { commitDirect(text); }
            @Override public void onSettings() { openSettings(); }
            @Override public void onGlobe() {
                android.view.inputmethod.InputMethodManager imm =
                        getSystemService(android.view.inputmethod.InputMethodManager.class);
                if (imm != null) imm.showInputMethodPicker();
            }
            @Override public void onVietnameseToggle() { toggleVietnamese(); }
        });
        syncKeyboardState();
        keyboard.setNumericMode(fieldKind == 2);
        return keyboard;
    }

    @Override
    public void onStartInput(EditorInfo attribute, boolean restarting) {
        super.onStartInput(attribute, restarting);
        fieldKind = fieldKind(attribute.inputType);
        if (handle == 0) {
            handle = NativeEngine.create(Prefs.configJson(this));
        } else {
            String action = NativeEngine.setConfig(handle, Prefs.configJson(this));
            applyProjected(action, "", snapshot(getCurrentInputConnection()));
        }
        NativeEngine.setField(handle, fieldKind);
        String focusAction = NativeEngine.focus(
                handle, attribute.packageName == null ? "" : attribute.packageName);
        applyProjected(focusAction, "", snapshot(getCurrentInputConnection()));
        syncKeyboardState();
        if (keyboard != null) {
            keyboard.setNumericMode(fieldKind == 2);
            keyboard.setSuggestions(null);
        }
    }

    @Override
    public void onStartInputView(EditorInfo info, boolean restarting) {
        super.onStartInputView(info, restarting);
        syncKeyboardState();
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
        if (keyboard != null) {
            keyboard.setSuggestions(null);
        }
        super.onFinishInput();
    }

    @Override
    public void onDestroy() {
        if (prefListener != null) {
            Prefs.shared(this).unregisterOnSharedPreferenceChangeListener(prefListener);
            prefListener = null;
        }
        if (handle != 0) {
            NativeEngine.free(handle);
            handle = 0;
        }
        super.onDestroy();
    }

    @Override
    public boolean onEvaluateInputViewShown() {
        return true;
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

    private void syncKeyboardState() {
        if (keyboard == null || handle == 0) return;
        keyboard.setVietnamese(NativeEngine.isEnabled(handle));
    }

    private void toggleVietnamese() {
        if (handle == 0) return;
        InputConnection ic = getCurrentInputConnection();
        Snapshot snapshot = snapshot(ic);
        applyProjected(NativeEngine.toggle(handle), "", snapshot);
        syncKeyboardState();
    }

    private void openSettings() {
        Intent intent = new Intent(this, SettingsActivity.class);
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        startActivity(intent);
    }

    private void press(String key) {
        if (handle == 0) return;
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return;
        else if (key.length() == 0) return;
        Snapshot snapshot = snapshot(ic);
        NativeEngine.setSurrounding(handle, snapshot.selection ? "" : snapshot.before);
        applyProjected(NativeEngine.process(handle, key, false, false, false), key, snapshot);
    }

    private void commitDirect(String text) {
        if (handle == 0 || text == null || text.isEmpty()) return;
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return;
        NativeEngine.reset(handle);
        applying = true;
        try {
            ic.setComposingText("", 1);
            ic.finishComposingText();
            ic.commitText(text, 1);
            lastAppliedAt = SystemClock.uptimeMillis();
        } finally {
            applying = false;
        }
        if (keyboard != null) keyboard.setSuggestions(null);
    }

    private static final class Snapshot {
        String before = "";
        String text = "";
        int cursor;
        int anchor;
        boolean selection;
    }

    private Snapshot snapshot(InputConnection ic) {
        Snapshot result = new Snapshot();
        if (ic == null) return result;
        CharSequence before = ic.getTextBeforeCursor(96, 0);
        CharSequence selected = ic.getSelectedText(0);
        CharSequence after = ic.getTextAfterCursor(2, 0);
        result.before = before == null ? "" : before.toString();
        String selectedText = selected == null ? "" : selected.toString();
        String afterText = after == null ? "" : after.toString();
        result.text = result.before + selectedText + afterText;
        result.cursor = (result.before + selectedText).length();
        result.anchor = result.before.length();
        result.selection = !selectedText.isEmpty();
        return result;
    }

    private void applyProjected(String actionJson, String originalKey, Snapshot snapshot) {
        if (handle == 0 || actionJson == null) {
            passOriginal(getCurrentInputConnection(), originalKey);
            return;
        }
        if (snapshot == null) snapshot = new Snapshot();
        String beforeDisplay = NativeEngine.display(handle);
        String plan = NativeEngine.plan(
                handle,
                actionJson,
                fieldKind,
                beforeDisplay == null ? "" : beforeDisplay,
                snapshot.text,
                snapshot.cursor,
                snapshot.anchor,
                false
        );
        applyPlan(plan, originalKey);
    }

    private void applyPlan(String planJson, String originalKey) {
        if (planJson == null) {
            passOriginal(getCurrentInputConnection(), originalKey);
            return;
        }
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return;
        applying = true;
        try {
            JSONObject plan = new JSONObject(planJson);
            String kind = plan.optString("kind");
            String text = plan.optString("text", "");
            boolean pass = plan.optBoolean("pass", false);
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
                    if (pass) passOriginal(ic, originalKey);
                    break;
                case "replace": {
                    int delete = plan.optInt("delete_utf16", 0);
                    if (delete > 0) ic.deleteSurroundingText(delete, 0);
                    ic.commitText(text, 1);
                    ic.finishComposingText();
                    if (pass) passOriginal(ic, originalKey);
                    break;
                }
                case "fail_safe":
                    NativeEngine.reset(handle);
                    passOriginal(ic, originalKey);
                    break;
                case "pass":
                default:
                    passOriginal(ic, originalKey);
                    break;
            }
            lastAppliedAt = SystemClock.uptimeMillis();
        } catch (Exception ignored) {
            passOriginal(ic, originalKey);
        } finally {
            applying = false;
        }
        updateSuggestions();
    }

    private void updateSuggestions() {
        if (keyboard == null) return;
        if (handle == 0 || fieldKind != 0) {
            keyboard.setSuggestions(null);
            return;
        }
        String display = NativeEngine.display(handle);
        if (display == null || display.isEmpty()) {
            keyboard.setSuggestions(null);
            return;
        }
        try {
            JSONArray array = new JSONArray(NativeEngine.suggest(handle, display));
            List<String> values = new ArrayList<>();
            for (int i = 0; i < array.length(); i++) values.add(array.getString(i));
            keyboard.setSuggestions(values);
        } catch (Exception e) {
            keyboard.setSuggestions(null);
        }
    }

    private void passOriginal(InputConnection ic, String key) {
        if (ic == null || key == null || key.isEmpty()) return;
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
            case "Left":
            case "Right":
            case "Up":
            case "Down":
                int code = "Left".equals(key) ? KeyEvent.KEYCODE_DPAD_LEFT
                        : "Right".equals(key) ? KeyEvent.KEYCODE_DPAD_RIGHT
                        : "Up".equals(key) ? KeyEvent.KEYCODE_DPAD_UP : KeyEvent.KEYCODE_DPAD_DOWN;
                long t = SystemClock.uptimeMillis();
                ic.sendKeyEvent(new KeyEvent(t, t, KeyEvent.ACTION_DOWN, code, 0));
                ic.sendKeyEvent(new KeyEvent(t, t, KeyEvent.ACTION_UP, code, 0));
                return;
            default:
                if (key.length() == 1) ic.commitText(key, 1);
        }
    }

    @Override
    public boolean onKeyDown(int keyCode, KeyEvent event) {
        if (handle == 0) return super.onKeyDown(keyCode, event);
        if (event.isCtrlPressed() || event.isAltPressed() || event.isMetaPressed()) {
            return super.onKeyDown(keyCode, event);
        }
        String key = hardwareKey(keyCode, event);
        if (key == null) return super.onKeyDown(keyCode, event);
        press(key);
        return true;
    }

    private String hardwareKey(int keyCode, KeyEvent event) {
        switch (keyCode) {
            case KeyEvent.KEYCODE_DEL: return "Backspace";
            case KeyEvent.KEYCODE_ENTER:
            case KeyEvent.KEYCODE_NUMPAD_ENTER: return "Enter";
            case KeyEvent.KEYCODE_TAB: return "Tab";
            case KeyEvent.KEYCODE_ESCAPE: return "Escape";
            case KeyEvent.KEYCODE_DPAD_LEFT: return "Left";
            case KeyEvent.KEYCODE_DPAD_RIGHT: return "Right";
            case KeyEvent.KEYCODE_DPAD_UP: return "Up";
            case KeyEvent.KEYCODE_DPAD_DOWN: return "Down";
            default:
                int unicode = event.getUnicodeChar();
                if (unicode == 0) return null;
                char c = (char) unicode;
                if (Character.isISOControl(c)) return null;
                return String.valueOf(c);
        }
    }

    @Override
    public void onUpdateSelection(int oldSelStart, int oldSelEnd, int newSelStart, int newSelEnd,
                                  int candidatesStart, int candidatesEnd) {
        super.onUpdateSelection(oldSelStart, oldSelEnd, newSelStart, newSelEnd, candidatesStart, candidatesEnd);
        if (applying || handle == 0) return;
        if (SystemClock.uptimeMillis() - lastAppliedAt < SELECTION_GUARD_MS) return;
        Snapshot snapshot = snapshot(getCurrentInputConnection());
        applyProjected(NativeEngine.cursorMoved(handle), "", snapshot);
    }
}
