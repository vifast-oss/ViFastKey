package app.vifastkey;

import android.content.Context;
import android.content.res.Configuration;
import android.graphics.Color;
import android.graphics.drawable.GradientDrawable;
import android.graphics.drawable.StateListDrawable;
import android.os.Handler;
import android.os.Looper;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.HapticFeedbackConstants;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.PopupMenu;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Bàn phím QWERTY đầy đủ cho ViFastKey.
 *
 * Không chứa logic tiếng Việt: mọi phím chữ được đẩy về IME service; các phím
 * chọn dấu ACCENT dùng commit trực tiếp để hoạt động cả khi không gõ Telex.
 */
final class ViFastKeyKeyboard extends LinearLayout {
    interface Listener {
        void onKey(String key);
        void onDirect(String text);
        void onSettings();
        void onGlobe();
        void onVietnameseToggle();
    }

    private static final String[] LETTER_ROWS = {
            "q w e r t y u i o p",
            "a s d f g h j k l",
    };
    private static final String[] SYMBOL_ROWS = {
            "1 2 3 4 5 6 7 8 9 0",
            "@ # $ % & * - + ( )",
            "! \" ' : ; / ? = _",
            "` ~ ^ | \\ [ ] { }",
    };
    private static final String[] EMOJI = {
            "😀", "😁", "😂", "🤣", "😊", "😍", "🥰", "😘", "😎", "🤔",
            "😅", "😢", "😭", "😡", "👍", "👎", "👏", "🙏", "💪", "❤️",
            "🔥", "⭐", "✨", "🎉", "☕", "🍜", "🚀", "💡", "✅", "❌",
            "🐱", "🐶", "🌸", "🌞", "🌙", "🇻🇳", "😴", "🥺", "🤝", "💯",
    };
    private static final Map<String, String[]> ACCENTS = new HashMap<>();
    static {
        ACCENTS.put("a", new String[]{"à", "á", "ả", "ã", "ạ", "ă", "â"});
        ACCENTS.put("e", new String[]{"è", "é", "ẻ", "ẽ", "ẹ", "ê"});
        ACCENTS.put("i", new String[]{"ì", "í", "ỉ", "ĩ", "ị"});
        ACCENTS.put("o", new String[]{"ò", "ó", "ỏ", "õ", "ọ", "ô", "ơ"});
        ACCENTS.put("u", new String[]{"ù", "ú", "ủ", "ũ", "ụ", "ư"});
        ACCENTS.put("y", new String[]{"ỳ", "ý", "ỷ", "ỹ", "ỵ"});
        ACCENTS.put("d", new String[]{"đ"});
    }

    private final Listener listener;
    private final Context context;
    private final Handler repeatHandler = new Handler(Looper.getMainLooper());
    private final List<Button> letterButtons = new ArrayList<>();
    private LinearLayout suggestions;
    private HorizontalScrollView suggestionScroll;
    private Button vietnameseButton;
    private Button bottomVietnameseButton;
    private boolean shift;
    private boolean caps;
    private boolean symbols;
    private boolean emojiPanel;
    private boolean numeric;
    private boolean vietnamese = true;
    private boolean dark;
    private Runnable repeatRunnable;

    ViFastKeyKeyboard(Context context, Listener listener) {
        super(context);
        this.context = context;
        this.listener = listener;
        dark = (context.getResources().getConfiguration().uiMode
                & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
        setOrientation(VERTICAL);
        setPadding(dp(3), dp(4), dp(3), dp(6));
        setBackgroundColor(dark ? 0xFF202124 : 0xFFE8EAED);
        rebuild();
    }

    void setSuggestions(List<String> values) {
        if (suggestions == null) return;
        suggestions.removeAllViews();
        if (values == null || values.isEmpty()) {
            // Giữ chiều cao cố định để bàn phím không nhảy khi gợi ý xuất hiện.
            suggestionScroll.setVisibility(INVISIBLE);
            suggestions.setVisibility(INVISIBLE);
            return;
        }
        suggestionScroll.setVisibility(VISIBLE);
        suggestions.setVisibility(VISIBLE);
        for (String value : values) {
            TextView item = new TextView(context);
            item.setText(value);
            item.setTextSize(16);
            item.setGravity(Gravity.CENTER);
            item.setPadding(dp(12), dp(4), dp(12), dp(4));
            item.setTextColor(dark ? 0xFFE8EAED : 0xFF202124);
            item.setOnClickListener(v -> {
                item.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP);
                listener.onDirect(value);
            });
            suggestions.addView(item);
        }
    }

    void setVietnamese(boolean enabled) {
        vietnamese = enabled;
        if (vietnameseButton != null) {
            vietnameseButton.setText(enabled ? "VI" : "EN");
            vietnameseButton.setTextColor(enabled ? Color.WHITE : (dark ? 0xFFE8EAED : 0xFF202124));
            vietnameseButton.setBackground(keyBackground(enabled ? 0xFFC2410C : keyColor()));
        }
        if (bottomVietnameseButton != null) {
            bottomVietnameseButton.setText(enabled ? "VI" : "EN");
            bottomVietnameseButton.setTextColor(enabled ? Color.WHITE : (dark ? 0xFFE8EAED : 0xFF202124));
            bottomVietnameseButton.setBackground(keyBackground(enabled ? 0xFFC2410C : keyColor()));
        }
    }

    void setShift(boolean enabled) {
        shift = enabled;
        updateLetters();
    }

    void setNumericMode(boolean enabled) {
        numeric = enabled;
        rebuild();
    }

    private void rebuild() {
        removeAllViews();
        letterButtons.clear();
        addTopBar();
        addSuggestionsBar();
        if (numeric) {
            addNumericRows();
        } else if (emojiPanel) {
            addEmojiRows();
        } else if (symbols) {
            addSymbolRows();
        } else {
            addLetterRows();
        }
        addBottomRow();
        updateLetters();
        setVietnamese(vietnamese);
    }

    private void addTopBar() {
        LinearLayout row = new LinearLayout(context);
        row.setOrientation(HORIZONTAL);
        row.setGravity(Gravity.CENTER_VERTICAL);
        Button gear = smallKey("⚙", "Cấu hình");
        gear.setOnClickListener(v -> listener.onSettings());
        row.addView(gear);
        Button globe = smallKey("🌐", "Chọn bàn phím");
        globe.setOnClickListener(v -> listener.onGlobe());
        row.addView(globe);
        Button emojiKey = smallKey(numeric || emojiPanel ? "ABC" : "😊", "Emoji");
        emojiKey.setOnClickListener(v -> {
            if (numeric) { numeric = false; symbols = false; emojiPanel = false; }
            else if (emojiPanel) { emojiPanel = false; symbols = false; }
            else { emojiPanel = true; symbols = false; numeric = false; }
            rebuild();
        });
        row.addView(emojiKey);
        Button sym = smallKey(numeric ? "ABC" : "?123", "Ký tự");
        sym.setOnClickListener(v -> {
            if (numeric) { numeric = false; symbols = false; }
            else { symbols = !symbols; emojiPanel = false; }
            rebuild();
        });
        row.addView(sym);
        vietnameseButton = smallKey("VI", "Bật/tắt tiếng Việt");
        vietnameseButton.setOnClickListener(v -> listener.onVietnameseToggle());
        row.addView(vietnameseButton);
        addView(row, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(38)));
    }

    private void addSuggestionsBar() {
        suggestionScroll = new HorizontalScrollView(context);
        suggestionScroll.setHorizontalScrollBarEnabled(false);
        suggestionScroll.setVisibility(INVISIBLE);
        suggestions = new LinearLayout(context);
        suggestions.setOrientation(HORIZONTAL);
        suggestions.setGravity(Gravity.CENTER_VERTICAL);
        suggestionScroll.addView(suggestions);
        addView(suggestionScroll, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(38)));
    }

    private void addLetterRows() {
        for (String keys : LETTER_ROWS) {
            LinearLayout row = new LinearLayout(context);
            row.setOrientation(HORIZONTAL);
            row.setGravity(Gravity.CENTER);
            String[] items = keys.split(" ");
            for (String key : items) {
                row.addView(letterKey(key));
            }
            addView(row, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(52)));
        }
        LinearLayout third = new LinearLayout(context);
        third.setOrientation(HORIZONTAL);
        third.setGravity(Gravity.CENTER);
        Button shiftKey = actionKey("⇧", 1.2f);
        shiftKey.setOnClickListener(v -> {
            if (shift && !caps) { caps = true; shift = true; }
            else if (caps) { caps = false; shift = false; }
            else { shift = true; }
            updateLetters();
        });
        shiftKey.setOnLongClickListener(v -> { caps = !caps; shift = caps; updateLetters(); return true; });
        third.addView(shiftKey);
        for (String key : "z x c v b n m".split(" ")) {
            third.addView(letterKey(key));
        }
        Button backspace = actionKey("⌫", 1.2f);
        backspace.setOnTouchListener((v, event) -> handleRepeat(event, "Backspace"));
        third.addView(backspace);
        addView(third, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(52)));

        LinearLayout numbers = new LinearLayout(context);
        numbers.setOrientation(HORIZONTAL);
        for (String key : "1 2 3 4 5 6 7 8 9 0".split(" ")) {
            numbers.addView(actionKey(key, 1f));
        }
        addView(numbers, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(48)));
    }

    private void addSymbolRows() {
        for (String keys : SYMBOL_ROWS) {
            LinearLayout row = new LinearLayout(context);
            row.setOrientation(HORIZONTAL);
            for (String key : keys.split(" ")) {
                row.addView(actionKey(key, 1f));
            }
            addView(row, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(52)));
        }
    }

    private void addNumericRows() {
        String[] rows = {"1 2 3", "4 5 6", "7 8 9", ". 0 ,"};
        for (String keys : rows) {
            LinearLayout row = new LinearLayout(context);
            row.setOrientation(HORIZONTAL);
            for (String key : keys.split(" ")) {
                row.addView(actionKey(key, 1f));
            }
            addView(row, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(54)));
        }
    }

    private void addEmojiRows() {
        for (int start = 0; start < EMOJI.length; start += 10) {
            LinearLayout row = new LinearLayout(context);
            row.setOrientation(HORIZONTAL);
            for (int i = start; i < Math.min(start + 10, EMOJI.length); i++) {
                Button button = actionKey(EMOJI[i], 1f);
                button.setTextSize(21);
                final String emoji = EMOJI[i];
                button.setOnClickListener(v -> listener.onDirect(emoji));
                row.addView(button);
            }
            addView(row, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(46)));
        }
    }

    private void addBottomRow() {
        LinearLayout row = new LinearLayout(context);
        row.setOrientation(HORIZONTAL);
        row.setGravity(Gravity.CENTER);
        Button mode = actionKey(numeric || symbols ? "ABC" : "?123", 1.25f);
        mode.setOnClickListener(v -> {
            if (numeric) {
                numeric = false;
                symbols = false;
            } else {
                symbols = !symbols;
            }
            emojiPanel = false;
            rebuild();
        });
        row.addView(mode);
        row.addView(actionKey(",", 0.8f));
        bottomVietnameseButton = actionKey(vietnamese ? "VI" : "EN", 1f);
        bottomVietnameseButton.setOnClickListener(v -> listener.onVietnameseToggle());
        row.addView(bottomVietnameseButton);
        Button space = actionKey("␣", 3.2f);
        space.setOnClickListener(v -> {
            space.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP);
            listener.onKey(" ");
        });
        row.addView(space);
        row.addView(actionKey(".", 0.8f));
        Button enter = actionKey("⏎", 1.2f);
        enter.setOnClickListener(v -> {
            enter.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP);
            listener.onKey("Enter");
        });
        row.addView(enter);
        addView(row, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(52)));
    }

    private Button letterKey(String lower) {
        Button button = actionKey(lower, 1f);
        letterButtons.add(button);
        button.setTag(lower);
        button.setOnClickListener(v -> {
            button.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP);
            String key = (shift || caps) ? lower.toUpperCase() : lower;
            if (!caps) shift = false;
            updateLetters();
            listener.onKey(key);
        });
        button.setOnLongClickListener(v -> {
            showAccents(button, lower);
            return true;
        });
        return button;
    }

    private Button actionKey(String label, float weight) {
        Button button = new Button(context);
        button.setText(label);
        button.setTextSize(17);
        button.setAllCaps(false);
        button.setGravity(Gravity.CENTER);
        button.setMinWidth(0);
        button.setMinimumWidth(0);
        button.setPadding(0, 0, 0, 0);
        button.setIncludeFontPadding(false);
        button.setTextColor(dark ? 0xFFE8EAED : 0xFF202124);
        button.setBackground(keyBackground(keyColor()));
        button.setSoundEffectsEnabled(true);
        LayoutParams params = new LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, weight);
        params.setMargins(dp(2), dp(2), dp(2), dp(2));
        button.setLayoutParams(params);
        button.setOnClickListener(v -> {
            button.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP);
            listener.onKey(label);
        });
        return button;
    }

    private Button smallKey(String label, String description) {
        Button button = new Button(context);
        button.setText(label);
        button.setTextSize(15);
        button.setContentDescription(description);
        button.setAllCaps(false);
        button.setMinWidth(0);
        button.setMinimumWidth(0);
        button.setPadding(0, 0, 0, 0);
        button.setTextColor(dark ? 0xFFE8EAED : 0xFF202124);
        button.setBackground(keyBackground(keyColor()));
        LayoutParams params = new LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, 1f);
        params.setMargins(dp(2), dp(2), dp(2), dp(2));
        button.setLayoutParams(params);
        return button;
    }

    private boolean handleRepeat(android.view.MotionEvent event, String key) {
        switch (event.getActionMasked()) {
            case android.view.MotionEvent.ACTION_DOWN:
                repeatHandler.removeCallbacksAndMessages(null);
                listener.onKey(key);
                repeatRunnable = new Runnable() {
                    @Override public void run() {
                        listener.onKey(key);
                        repeatHandler.postDelayed(this, 65);
                    }
                };
                repeatHandler.postDelayed(repeatRunnable, 420);
                return true;
            case android.view.MotionEvent.ACTION_UP:
            case android.view.MotionEvent.ACTION_CANCEL:
                repeatHandler.removeCallbacksAndMessages(null);
                repeatRunnable = null;
                return true;
            default:
                return false;
        }
    }

    private void showAccents(Button anchor, String key) {
        String[] accents = ACCENTS.get(key);
        if (accents == null) return;
        PopupMenu popup = new PopupMenu(context, anchor);
        for (String accent : accents) {
            String value = (shift || caps) ? accent.toUpperCase() : accent;
            popup.getMenu().add(value);
        }
        popup.setOnMenuItemClickListener(item -> {
            listener.onDirect(item.getTitle().toString());
            return true;
        });
        popup.show();
    }

    private void updateLetters() {
        for (Button button : letterButtons) {
            String key = (String) button.getTag();
            button.setText((shift || caps) ? key.toUpperCase() : key);
        }
    }

    private StateListDrawable keyBackground(int color) {
        StateListDrawable states = new StateListDrawable();
        states.addState(new int[]{android.R.attr.state_pressed}, roundColor(0xFF9AA0A6));
        states.addState(new int[]{}, roundColor(color));
        return states;
    }

    private GradientDrawable roundColor(int color) {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(color);
        drawable.setCornerRadius(dp(6));
        return drawable;
    }

    private int keyColor() {
        return dark ? 0xFF3C4043 : 0xFFF8F9FA;
    }

    private int dp(int value) {
        return Math.round(TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, value,
                getResources().getDisplayMetrics()));
    }
}
