package app.vifastkey;

import android.content.ClipData;
import android.content.ClipboardManager;
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
    private boolean preeditUnderline;
    private long lastAppliedAt;
    private boolean smartSpacePending;
    private int fieldKind = 0;
    private String lastInputPackage = "";
    private SharedPreferences.OnSharedPreferenceChangeListener prefListener;

    @Override
    public void onCreate() {
        super.onCreate();
        handle = NativeEngine.create(Prefs.configJson(this));
        preeditUnderline = Prefs.bool(this, "show_underline", false);
        NativeEngine.setOutputMode(handle, preeditUnderline ? 1 : 2);
        new Thread(() -> NativeEngine.suggest(0L, "a"), "vfk-suggest-warmup").start();
        prefListener = (sharedPreferences, key) -> {
            if (handle != 0) {
                String beforeDisplay = NativeEngine.display(handle);
                String action = NativeEngine.setConfig(handle, Prefs.configJson(this));
                applyProjected(action, "", snapshotForEngine(getCurrentInputConnection()), beforeDisplay);
                syncUnderlineMode();
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
            @Override public void onSuggestion(String text) { commitSuggestion(text); }
            @Override public void onEmoji(String emoji) { commitDirect(emoji); }
            @Override public void onTextAction(String action) { performTextAction(action); }
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
        String inputPackage = attribute.packageName == null ? "" : attribute.packageName;
        boolean appChanged = !inputPackage.equals(lastInputPackage);
        lastInputPackage = inputPackage;
        smartSpacePending = false;
        if (handle != 0) discardStaleComposition(restarting);
        fieldKind = fieldKind(attribute.inputType);
        if (handle == 0) {
            handle = NativeEngine.create(Prefs.configJson(this));
            preeditUnderline = Prefs.bool(this, "show_underline", false);
            NativeEngine.setOutputMode(handle, preeditUnderline ? 1 : 2);
        } else {
            String beforeDisplay = NativeEngine.display(handle);
            String action = NativeEngine.setConfig(handle, Prefs.configJson(this));
            applyProjected(action, "", snapshotForEngine(getCurrentInputConnection()), beforeDisplay);
        }
        NativeEngine.setField(handle, fieldKind);
        String beforeFocus = NativeEngine.display(handle);
        String focusAction = NativeEngine.focus(
                handle, attribute.packageName == null ? "" : attribute.packageName);
        applyProjected(focusAction, "", snapshotForEngine(getCurrentInputConnection()), beforeFocus);
        syncKeyboardState();
        if (keyboard != null) {
            if (!restarting || appChanged) {
                keyboard.resetLayoutForField(fieldKind == 2);
            } else {
                keyboard.setNumericMode(fieldKind == 2);
            }
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
        smartSpacePending = false;
        InputConnection ic = getCurrentInputConnection();
        String display = handle == 0 ? "" : NativeEngine.display(handle);
        boolean present = display != null && !display.isEmpty()
                && composingTextPresent(ic, display);
        if (handle != 0) {
            NativeEngine.reset(handle);
        }
        if (ic != null) {
            // App đã gửi và xoá field trước khi IME kịp chốt composition: xoá
            // composing text thay vì commit lại chữ cũ vào field rỗng.
            if (display != null && !display.isEmpty() && !present) {
                clearComposing(ic);
            } else {
                ic.finishComposingText();
            }
        }
        if (keyboard != null) {
            keyboard.setSuggestions(null);
            keyboard.setAutoShift(false);
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

    /**
     * Không chuyển composition của field cũ sang field mới. Khi host gửi tin
     * nhắn và xoá field, composition cũ không còn trong field nên phải xoá,
     * tránh commit lại thành chữ sót.
     */
    private void discardStaleComposition(boolean restarting) {
        String display = NativeEngine.display(handle);
        if (display == null || display.isEmpty()) return;
        InputConnection ic = getCurrentInputConnection();
        boolean present = composingTextPresent(ic, display);
        if (!restarting) {
            NativeEngine.reset(handle);
            if (ic != null) {
                if (present) {
                    ic.finishComposingText();
                } else {
                    clearComposing(ic);
                }
            }
        } else if (!present) {
            NativeEngine.reset(handle);
            clearComposing(ic);
        }
    }

    private boolean composingTextPresent(InputConnection ic, String display) {
        if (ic == null || display == null || display.isEmpty()) return false;
        CharSequence before = ic.getTextBeforeCursor(display.length() + 2, 0);
        return before != null && before.toString().endsWith(display);
    }

    private void clearComposing(InputConnection ic) {
        if (ic == null) return;
        applying = true;
        try {
            ic.setComposingText("", 1);
            ic.finishComposingText();
        } catch (Exception ignored) {
            // InputConnection của app đã đóng.
        } finally {
            applying = false;
            lastAppliedAt = SystemClock.uptimeMillis();
        }
    }

    /** Xoá vùng bôi đen bằng một lần commit rỗng; trả false nếu không có selection. */
    private boolean deleteSelection(InputConnection ic) {
        if (ic == null) return false;
        CharSequence selected;
        try {
            selected = ic.getSelectedText(0);
        } catch (Exception ignored) {
            return false;
        }
        if (selected == null || selected.length() == 0) return false;
        if (handle != 0) NativeEngine.reset(handle);
        smartSpacePending = false;
        applying = true;
        boolean deleted = false;
        try {
            ic.commitText("", 1);
            ic.finishComposingText();
            deleted = true;
        } catch (Exception ignored) {
            // App đã đóng InputConnection.
        } finally {
            applying = false;
            lastAppliedAt = SystemClock.uptimeMillis();
        }
        if (deleted) {
            if (keyboard != null) keyboard.setSuggestions(null);
            updateAutoShift();
        }
        return deleted;
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

    private void syncUnderlineMode() {
        if (handle == 0) return;
        boolean show = Prefs.bool(this, "show_underline", false);
        if (show == preeditUnderline) return;
        String beforeDisplay = NativeEngine.display(handle);
        preeditUnderline = show;
        String action = NativeEngine.setOutputMode(handle, show ? 1 : 2);
        applyProjected(action, "", snapshotForEngine(getCurrentInputConnection()), beforeDisplay);
    }

    private void syncKeyboardState() {
        if (keyboard == null) return;
        keyboard.setMethod(Prefs.method(this));
        keyboard.setNumberRow(Prefs.bool(this, "show_number_row", false));
        keyboard.setHeightScale(Prefs.shared(this).getInt("keyboard_height_percent", 100));
        if (handle == 0) return;
        keyboard.setVietnamese(NativeEngine.isEnabled(handle));
        updateAutoShift();
    }

    private void updateAutoShift() {
        if (keyboard == null) return;
        keyboard.setAutoShift(shouldAutoShift());
    }

    /**
     * Shift tự động ở đầu field/câu, sau dấu chấm than, chấm hỏi hoặc dấu chấm
     * khi đã có khoảng trắng. Bỏ qua field mật khẩu/email/URL/số.
     */
    private boolean shouldAutoShift() {
        if (fieldKind != 0) return false;
        if (handle != 0) {
            String composing = NativeEngine.display(handle);
            if (composing != null && !composing.isEmpty()) return false;
        }
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return false;
        CharSequence selected = ic.getSelectedText(0);
        if (selected != null && selected.length() > 0) return false;
        CharSequence before = ic.getTextBeforeCursor(128, 0);
        if (before == null) return false;
        return startsSentence(before.toString());
    }

    private static boolean startsSentence(String text) {
        int i = text.length();
        boolean whitespace = false;
        while (i > 0) {
            char c = text.charAt(i - 1);
            if (c == '\n' || c == '\r') return true;
            if (c == ' ' || c == '\t' || c == '\u00A0') {
                whitespace = true;
                i--;
                continue;
            }
            break;
        }
        if (i == 0) return true;
        if (!whitespace) return false;
        while (i > 0 && isSentenceCloser(text.charAt(i - 1))) i--;
        if (i == 0) return true;
        char c = text.charAt(i - 1);
        return c == '.' || c == '!' || c == '?' || c == '\u2026';
    }

    private static boolean isSentenceCloser(char c) {
        return c == '"' || c == '\'' || c == '\u2019' || c == '\u201D'
                || c == ')' || c == ']' || c == '}' || c == '\u00BB';
    }

    private void toggleVietnamese() {
        if (handle == 0) return;
        Snapshot snapshot = snapshotForEngine(getCurrentInputConnection());
        String beforeDisplay = NativeEngine.display(handle);
        applyProjected(NativeEngine.toggle(handle), "", snapshot, beforeDisplay);
        syncKeyboardState();
    }

    private void performTextAction(String action) {
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return;
        if ("select_all".equals(action)) {
            ic.performContextMenuAction(android.R.id.selectAll);
            return;
        }
        CharSequence selected;
        try {
            selected = ic.getSelectedText(0);
        } catch (Exception ignored) {
            return;
        }
        if (selected == null || selected.length() == 0) return;
        ClipboardManager clipboard = getSystemService(ClipboardManager.class);
        if (clipboard != null) {
            clipboard.setPrimaryClip(ClipData.newPlainText("ViFastKey", selected));
        }
        if ("cut".equals(action)) {
            deleteSelection(ic);
        }
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

        if (smartSpacePending) {
            smartSpacePending = false;
            if (" ".equals(key)) {
                // Gợi ý đã thêm sẵn một dấu cách.
                return;
            }
            if ("Enter".equals(key) || attachesToPreviousWord(key)) {
                removeSmartSpace(ic);
            }
        }

        if ("Backspace".equals(key) && deleteSelection(ic)) return;

        maybeAutoCorrect(ic, key);

        Snapshot snapshot = snapshotForEngine(ic);
        NativeEngine.setSurrounding(handle, snapshot.selection ? "" : snapshot.before);
        String beforeDisplay = NativeEngine.display(handle);
        String action = NativeEngine.process(handle, key, false, false, false);
        applyProjected(action, key, snapshot, beforeDisplay);
    }

    private static boolean attachesToPreviousWord(String key) {
        return key.length() == 1 && ".,!?:;…)]}".indexOf(key.charAt(0)) >= 0;
    }

    /**
     * Tự sửa từ hiện tại khi sắp chốt từ (space/dấu câu/Enter) nếu engine
     * chắc chắn có đúng một ứng viên tiếng Việt thông dụng gần giống.
     */
    private void maybeAutoCorrect(InputConnection ic, String key) {
        if (handle == 0 || fieldKind != 0 || ic == null) return;
        if (!Prefs.bool(this, "auto_correct", true)) return;
        if (!(" ".equals(key) || "Enter".equals(key) || attachesToPreviousWord(key))) return;
        if (!NativeEngine.isEnabled(handle)) return;
        String display = NativeEngine.display(handle);
        if (display == null || display.isEmpty()) return;
        for (int i = 0; i < display.length(); i++) {
            if (Character.isWhitespace(display.charAt(i))) return;
        }
        String corrected = NativeEngine.autocorrect(display);
        if (corrected == null || corrected.isEmpty() || corrected.equals(display)) return;

        if (!preeditUnderline) {
            replaceCommittedWord(corrected, false);
            return;
        }

        NativeEngine.reset(handle);
        applying = true;
        try {
            ic.setComposingText("", 1);
            ic.finishComposingText();
            ic.commitText(corrected, 1);
            smartSpacePending = false;
            lastAppliedAt = SystemClock.uptimeMillis();
        } catch (Exception ignored) {
            // Không sửa được thì để từ gốc đi tiếp.
        } finally {
            applying = false;
        }
    }

    private void removeSmartSpace(InputConnection ic) {
        if (ic == null) return;
        try {
            CharSequence before = ic.getTextBeforeCursor(1, 0);
            if (before != null && before.length() == 1 && before.charAt(0) == ' ') {
                ic.deleteSurroundingText(1, 0);
                lastAppliedAt = SystemClock.uptimeMillis();
            }
        } catch (Exception ignored) {
            // App đã đóng InputConnection.
        }
    }

    private void commitDirect(String text) {
        commitText(text, false);
    }

    private void commitSuggestion(String text) {
        if (!preeditUnderline) {
            replaceCommittedWord(text, true);
            return;
        }
        commitText(text, true);
    }

    private void commitText(String text, boolean trailingSpace) {
        if (handle == 0 || text == null || text.isEmpty()) return;
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return;
        NativeEngine.reset(handle);
        applying = true;
        try {
            if (preeditUnderline) {
                ic.setComposingText("", 1);
                ic.finishComposingText();
            }
            ic.commitText(trailingSpace ? text + " " : text, 1);
            smartSpacePending = trailingSpace;
            lastAppliedAt = SystemClock.uptimeMillis();
        } finally {
            applying = false;
        }
        if (keyboard != null) keyboard.setSuggestions(null);
        updateAutoShift();
    }

    /**
     * Chế độ không gạch chân dùng text đã commit; thay từ hiện tại bằng
     * suggestion/autocorrect mà không chèn thêm một bản.
     */
    private boolean replaceCommittedWord(String replacement, boolean trailingSpace) {
        if (handle == 0 || replacement == null) return false;
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return false;
        String display = NativeEngine.display(handle);
        NativeEngine.reset(handle);
        applying = true;
        boolean batching = ic.beginBatchEdit();
        try {
            if (display != null && !display.isEmpty() && composingTextPresent(ic, display)) {
                ic.deleteSurroundingText(display.length(), 0);
            }
            ic.commitText(trailingSpace ? replacement + " " : replacement, 1);
            smartSpacePending = trailingSpace;
            lastAppliedAt = SystemClock.uptimeMillis();
            return true;
        } catch (Exception ignored) {
            return false;
        } finally {
            if (batching) ic.endBatchEdit();
            applying = false;
        }
    }

    private static final class Snapshot {
        String before = "";
        String text = "";
        int cursor;
        int anchor;
        boolean selection;
    }

    private Snapshot snapshotForEngine(InputConnection ic) {
        // Không đọc nội dung field nhạy cảm. Planner cũng trả pass cho các field này.
        if (fieldKind != 0 && fieldKind != 5) return new Snapshot();
        return snapshot(ic);
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

    private void applyProjected(String actionJson, String originalKey,
                                Snapshot snapshot, String beforeDisplay) {
        if (handle == 0 || actionJson == null) {
            passOriginal(getCurrentInputConnection(), originalKey);
            updateAutoShift();
            return;
        }
        if (snapshot == null) snapshot = new Snapshot();
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
            updateAutoShift();
            return;
        }
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return;
        applying = true;
        boolean batching = false;
        try {
            JSONObject plan = new JSONObject(planJson);
            String kind = plan.optString("kind");
            String text = plan.optString("text", "");
            boolean pass = plan.optBoolean("pass", false);
            if (!preeditUnderline && "replace".equals(kind)) {
                batching = ic.beginBatchEdit();
            }
            switch (kind) {
                case "preedit":
                    if (text.isEmpty()) {
                        // finishComposingText() sẽ commit lại chữ đang compose,
                        // khiến Backspace ký tự cuối bị "khựng". Phải xoá region.
                        ic.setComposingText("", 1);
                        ic.finishComposingText();
                    } else {
                        ic.setComposingText(text, 1);
                    }
                    break;
                case "commit":
                    ic.commitText(text, 1);
                    if (preeditUnderline) ic.finishComposingText();
                    if (pass) passOriginal(ic, originalKey);
                    break;
                case "replace": {
                    int delete = plan.optInt("delete_utf16", 0);
                    if (delete > 0) ic.deleteSurroundingText(delete, 0);
                    ic.commitText(text, 1);
                    if (preeditUnderline) ic.finishComposingText();
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
            if (batching) ic.endBatchEdit();
            applying = false;
        }
        updateSuggestions();
        updateAutoShift();
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
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return;

        if (SystemClock.uptimeMillis() - lastAppliedAt < SELECTION_GUARD_MS) {
            // Gửi tin nhắn ngay sau khi gõ: app xoá field trước khi guard hết hạn.
            if (newSelStart == 0 && newSelEnd == 0 && (oldSelStart != 0 || oldSelEnd != 0)) {
                String display = NativeEngine.display(handle);
                if (display != null && !display.isEmpty() && fieldLooksEmpty(ic)) {
                    smartSpacePending = false;
                    NativeEngine.reset(handle);
                    clearComposing(ic);
                    updateSuggestions();
                    updateAutoShift();
                }
            }
            return;
        }

        String display = NativeEngine.display(handle);
        boolean selectionChanged = oldSelStart != newSelStart || oldSelEnd != newSelEnd;
        boolean composing = candidatesStart >= 0 && candidatesEnd > candidatesStart;

        if (!preeditUnderline) {
            // Replace mode không có composing region. Chỉ xử lý khi con trỏ thực
            // sự rời khỏi từ đang gõ; callback do app tự chỉnh selection lúc
            // commit/autosave không được reset state giữa từ.
            if (!selectionChanged) return;
            if (display != null && !display.isEmpty() && wordAtCursor(ic, display)) return;
            smartSpacePending = false;
            NativeEngine.cursorMoved(handle);
            updateSuggestions();
            updateAutoShift();
            return;
        }

        if (!selectionChanged) {
            // Một số app (Notes) tự chốt composing region khi autosave. Callback
            // chỉ đổi candidates, không đổi selection. Không được flush lại
            // preedit ở đây, tránh commit thêm một bản thành "NguyễnNguyễn".
            if (display != null && !display.isEmpty() && !composing) {
                smartSpacePending = false;
                NativeEngine.reset(handle);
                updateSuggestions();
                updateAutoShift();
            }
            return;
        }

        smartSpacePending = false;
        Snapshot snapshot = snapshotForEngine(ic);
        if (display != null && !display.isEmpty() && !snapshot.text.contains(display)) {
            // Composition không còn ở quanh con trỏ; kiểm tra xa hơn trước khi
            // kết luận host đã xoá field (tránh mất chữ khi chỉ di chuyển con trỏ).
            CharSequence after = ic.getTextAfterCursor(128, 0);
            if (after == null || !after.toString().contains(display)) {
                NativeEngine.reset(handle);
                clearComposing(ic);
                updateSuggestions();
                updateAutoShift();
                return;
            }
        }

        // Chốt composition đang active bằng finishComposingText() thay vì
        // commitText(display): nếu app đã tự chốt trước đó, commitText sẽ nhân đôi.
        NativeEngine.cursorMoved(handle);
        if (display != null && !display.isEmpty()) {
            applying = true;
            try {
                ic.finishComposingText();
                lastAppliedAt = SystemClock.uptimeMillis();
            } catch (Exception ignored) {
                // InputConnection của app đã đóng.
            } finally {
                applying = false;
            }
        }
        updateSuggestions();
        updateAutoShift();
    }

    private boolean wordAtCursor(InputConnection ic, String display) {
        if (ic == null || display == null || display.isEmpty()) return false;
        CharSequence before = ic.getTextBeforeCursor(display.length(), 0);
        return before != null && before.toString().equals(display);
    }

    private boolean fieldLooksEmpty(InputConnection ic) {
        CharSequence before = ic.getTextBeforeCursor(1, 0);
        if (before == null || before.length() > 0) return false;
        CharSequence after = ic.getTextAfterCursor(1, 0);
        return after == null || after.length() == 0;
    }
}
