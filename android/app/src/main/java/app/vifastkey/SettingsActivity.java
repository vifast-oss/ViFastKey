package app.vifastkey;

import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageInfo;
import android.os.Build;
import android.os.Bundle;
import android.graphics.Color;
import android.graphics.drawable.GradientDrawable;
import android.provider.Settings;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.AdapterView;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.TextView;
import android.widget.Toast;

import java.util.ArrayList;
import java.util.List;

/** Cấu hình bàn phím Android; lưu vào SharedPreferences rồi engine đọc JSON. */
public final class SettingsActivity extends Activity {
    private static final String[] METHODS = {"telex", "simple_telex", "vni", "viqr"};
    private static final String[] METHOD_LABELS = {"Telex", "Simple Telex", "VNI", "VIQR"};
    private static final String ADVANCED_RETURN_SYMBOL = "return_to_letters_after_symbol";
    private static final String ADVANCED_UNDERLINE = "show_underline";
    private static final String NUMBER_ROW = "show_number_row";
    private static final String HEIGHT_KEY = "keyboard_height_percent";
    private static final int[] HEIGHT_VALUES = {100, 115, 130, 145};
    private static final String[] HEIGHT_LABELS = {
            "Chuẩn (100%)", "Cao (115%)", "Rất cao (130%)", "Tối đa (145%)",
    };
    private static final String[] BOOL_KEYS = {
            "spell_check", "w_as_u", "bracket_keys",
            "quick_telex", "allow_coda_k", "macro_enabled", "macro_digits",
            "macro_undo", "macro_in_off_mode", "auto_capitalize", "smart_caps",
            "auto_correct", "edit_previous_word", "restore_after_space",
    };
    private final List<CheckBox> checks = new ArrayList<>();
    private Spinner methodSpinner;
    private Spinner toneSpinner;
    private Spinner heightSpinner;
    private EditText macroEdit;
    private CheckBox numberRowCheck;
    private TextView numberRowHint;
    private boolean numberRowPreference;
    private boolean updatingNumberRow;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        LinearLayout screen = new LinearLayout(this);
        screen.setOrientation(LinearLayout.VERTICAL);
        ScrollView scroll = new ScrollView(this);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(dp(24), dp(48), dp(24), dp(24));
        scroll.addView(root);

        ImageView logo = new ImageView(this);
        logo.setImageResource(R.drawable.vifastkey_icon);
        logo.setContentDescription("ViFastKey");
        logo.setAdjustViewBounds(true);
        logo.setScaleType(ImageView.ScaleType.CENTER_INSIDE);
        LinearLayout.LayoutParams logoParams = new LinearLayout.LayoutParams(dp(72), dp(72));
        logoParams.gravity = Gravity.CENTER_HORIZONTAL;
        logoParams.bottomMargin = dp(6);
        root.addView(logo, logoParams);

        TextView title = new TextView(this);
        title.setText("ViFastKey");
        title.setTextSize(30);
        title.setGravity(Gravity.CENTER);
        root.addView(title);

        TextView version = new TextView(this);
        version.setText(versionLabel());
        version.setTextSize(13);
        version.setTextColor(0xFF5F6673);
        version.setGravity(Gravity.CENTER);
        root.addView(version);

        TextView body = new TextView(this);
        body.setTextSize(15);
        body.setPadding(0, dp(12), 0, dp(12));
        body.setText("Bật bàn phím trong Keyboard & input methods, chọn ViFastKey, "
                + "sau đó chọn làm bàn phím hiện tại. Telex: tieengs → tiếng.");
        root.addView(body);

        Button enable = new Button(this);
        enable.setText("Mở cài đặt bàn phím");
        enable.setOnClickListener(v ->
                startActivity(new Intent(Settings.ACTION_INPUT_METHOD_SETTINGS)));
        root.addView(enable, matchWrap());

        Button test = new Button(this);
        test.setText("Mở ô gõ thử");
        test.setOnClickListener(v -> startActivity(new Intent(this, TestActivity.class)));
        root.addView(test, matchWrap());

        root.addView(section("Kiểu gõ"));
        methodSpinner = new Spinner(this);
        ArrayAdapter<String> methodAdapter = new ArrayAdapter<>(this,
                android.R.layout.simple_spinner_dropdown_item, METHOD_LABELS);
        methodSpinner.setAdapter(methodAdapter);
        root.addView(methodSpinner, matchWrap());

        root.addView(section("Đặt dấu"));
        toneSpinner = new Spinner(this);
        ArrayAdapter<String> toneAdapter = new ArrayAdapter<>(this,
                android.R.layout.simple_spinner_dropdown_item,
                new String[]{"Cũ (hòa)", "Mới (hoà)"});
        toneSpinner.setAdapter(toneAdapter);
        root.addView(toneSpinner, matchWrap());

        root.addView(section("Bàn phím"));
        TextView heightLabel = new TextView(this);
        heightLabel.setText("Chiều cao bàn phím");
        heightLabel.setTextSize(14);
        heightLabel.setPadding(0, dp(4), 0, 0);
        root.addView(heightLabel, matchWrap());
        heightSpinner = new Spinner(this);
        ArrayAdapter<String> heightAdapter = new ArrayAdapter<>(this,
                android.R.layout.simple_spinner_dropdown_item, HEIGHT_LABELS);
        heightSpinner.setAdapter(heightAdapter);
        root.addView(heightSpinner, matchWrap());
        TextView heightHint = new TextView(this);
        heightHint.setText("Cao hơn để phím dễ bấm; áp dụng ngay sau khi lưu.");
        heightHint.setTextSize(12);
        heightHint.setTextColor(0xFF5F6673);
        heightHint.setPadding(0, 0, 0, dp(6));
        root.addView(heightHint, matchWrap());
        numberRowCheck = new CheckBox(this);
        numberRowCheck.setText("Hiện hàng số 1–0 trên bàn phím chữ");
        numberRowCheck.setTag(NUMBER_ROW);
        numberRowCheck.setOnCheckedChangeListener((button, checked) -> {
            if (!updatingNumberRow && !isVniSelected()) numberRowPreference = checked;
        });
        root.addView(numberRowCheck, matchWrap());
        numberRowHint = new TextView(this);
        numberRowHint.setTextSize(12);
        numberRowHint.setTextColor(0xFF5F6673);
        numberRowHint.setPadding(0, 0, 0, dp(4));
        root.addView(numberRowHint, matchWrap());

        root.addView(section("Tùy chọn"));
        for (String key : BOOL_KEYS) {
            CheckBox check = new CheckBox(this);
            check.setText(label(key));
            check.setTag(key);
            checks.add(check);
            root.addView(check, matchWrap());
        }

        root.addView(section("Macro"));
        macroEdit = new EditText(this);
        macroEdit.setHint("ko=không\nvn=Việt Nam");
        macroEdit.setMinLines(4);
        macroEdit.setGravity(Gravity.TOP);
        root.addView(macroEdit, matchWrap());

        root.addView(section("Nâng cao"));
        TextView advancedHint = new TextView(this);
        advancedHint.setText("Chỉ dành cho người dùng nâng cao. Mặc định gõ ký tự đặc biệt "
                + "vẫn ở bàn phím ký tự, không tự quay về bàn phím chữ.");
        advancedHint.setTextSize(12);
        advancedHint.setTextColor(0xFF5F6673);
        advancedHint.setPadding(0, 0, 0, dp(4));
        root.addView(advancedHint, matchWrap());
        CheckBox advancedReturn = new CheckBox(this);
        advancedReturn.setText(label(ADVANCED_RETURN_SYMBOL));
        advancedReturn.setTag(ADVANCED_RETURN_SYMBOL);
        checks.add(advancedReturn);
        root.addView(advancedReturn, matchWrap());

        CheckBox underline = new CheckBox(this);
        underline.setText(label(ADVANCED_UNDERLINE));
        underline.setTag(ADVANCED_UNDERLINE);
        checks.add(underline);
        root.addView(underline, matchWrap());
        TextView underlineHint = new TextView(this);
        underlineHint.setText("Mặc định tắt giống Gboard; bật để thấy vùng preedit "
                + "đang soạn khi gõ.");
        underlineHint.setTextSize(12);
        underlineHint.setTextColor(0xFF5F6673);
        underlineHint.setPadding(0, 0, 0, dp(4));
        root.addView(underlineHint, matchWrap());

        LinearLayout bar = new LinearLayout(this);
        bar.setOrientation(LinearLayout.VERTICAL);
        bar.setPadding(dp(20), 0, dp(20), dp(20));
        bar.setBackgroundColor(0xFFFBF6EE);
        View divider = new View(this);
        divider.setBackgroundColor(0xFFE8DDD0);
        bar.addView(divider, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(1)));
        TextView hint = new TextView(this);
        hint.setText("Cấu hình áp dụng ngay sau khi lưu.");
        hint.setTextSize(12);
        hint.setTextColor(0xFF5F6673);
        hint.setPadding(0, dp(10), 0, dp(8));
        bar.addView(hint, matchWrap());
        Button save = new Button(this);
        save.setText("Lưu cấu hình");
        save.setAllCaps(false);
        save.setTextSize(16);
        save.setTextColor(Color.WHITE);
        save.setMinHeight(0);
        save.setPadding(0, dp(12), 0, dp(12));
        GradientDrawable saveBg = new GradientDrawable();
        saveBg.setColor(0xFFC2410C);
        saveBg.setCornerRadius(dp(14));
        save.setBackground(saveBg);
        save.setOnClickListener(v -> save());
        bar.addView(save, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        screen.addView(scroll, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));
        screen.addView(bar, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        setContentView(screen);
        load();
    }

    private TextView section(String text) {
        TextView view = new TextView(this);
        view.setText(text);
        view.setTextSize(18);
        view.setPadding(0, dp(20), 0, dp(4));
        return view;
    }

    private LinearLayout.LayoutParams matchWrap() {
        return new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
    }

    private void load() {
        String method = Prefs.method(this);
        for (int i = 0; i < METHODS.length; i++) {
            if (METHODS[i].equals(method)) methodSpinner.setSelection(i);
        }
        numberRowPreference = Prefs.bool(this, NUMBER_ROW, false);
        int height = Prefs.shared(this).getInt(HEIGHT_KEY, 100);
        int heightIndex = 0;
        for (int i = 0; i < HEIGHT_VALUES.length; i++) {
            if (HEIGHT_VALUES[i] == height) heightIndex = i;
        }
        heightSpinner.setSelection(heightIndex);
        toneSpinner.setSelection("new".equals(getSharedPreferences("vifastkey", MODE_PRIVATE)
                .getString("tone_style", "new")) ? 1 : 0);
        for (CheckBox check : checks) {
            String key = (String) check.getTag();
            boolean fallback = !key.equals("bracket_keys") && !key.equals("quick_telex")
                    && !key.equals("allow_coda_k") && !key.equals("macro_digits")
                    && !key.equals("macro_in_off_mode") && !key.equals("auto_capitalize")
                    && !key.equals("smart_caps") && !key.equals(ADVANCED_RETURN_SYMBOL)
                    && !key.equals(ADVANCED_UNDERLINE);
            check.setChecked(Prefs.bool(this, key, fallback));
        }
        macroEdit.setText(Prefs.macrosText(this));
        updateNumberRowState();
        methodSpinner.setOnItemSelectedListener(new AdapterView.OnItemSelectedListener() {
            @Override
            public void onItemSelected(AdapterView<?> parent, View view, int position, long id) {
                updateNumberRowState();
            }

            @Override
            public void onNothingSelected(AdapterView<?> parent) {
            }
        });
    }

    private boolean isVniSelected() {
        if (methodSpinner == null) return false;
        int position = methodSpinner.getSelectedItemPosition();
        return position >= 0 && position < METHODS.length && "vni".equals(METHODS[position]);
    }

    private void updateNumberRowState() {
        if (numberRowCheck == null) return;
        boolean vni = isVniSelected();
        updatingNumberRow = true;
        numberRowCheck.setEnabled(!vni);
        numberRowCheck.setChecked(vni || numberRowPreference);
        updatingNumberRow = false;
        if (numberRowHint != null) {
            numberRowHint.setText(vni
                    ? "VNI luôn bật hàng số 1–0 để gõ dấu/âm; không thể tắt."
                    : "Mặc định tắt. Bật nếu muốn gõ số nhanh ở hàng đầu; VNI luôn bật.");
        }
    }

    private void save() {
        Prefs.method(this, METHODS[methodSpinner.getSelectedItemPosition()]);
        Prefs.setBool(this, NUMBER_ROW, numberRowPreference);
        Prefs.shared(this).edit()
                .putInt(HEIGHT_KEY, HEIGHT_VALUES[heightSpinner.getSelectedItemPosition()])
                .apply();
        getSharedPreferences("vifastkey", MODE_PRIVATE).edit()
                .putString("tone_style", toneSpinner.getSelectedItemPosition() == 1 ? "new" : "old")
                .apply();
        for (CheckBox check : checks) {
            Prefs.setBool(this, (String) check.getTag(), check.isChecked());
        }
        Prefs.macrosText(this, macroEdit.getText().toString());
        Toast.makeText(this, "Đã lưu — bàn phím áp dụng ở lần gõ tiếp theo", Toast.LENGTH_SHORT).show();
    }

    private String label(String key) {
        switch (key) {
            case "spell_check": return "Kiểm tra chính tả";
            case "w_as_u": return "w → ư";
            case "bracket_keys": return "[ ] → ơ ư";
            case "quick_telex": return "Quick Telex (cc, gg, ...)";
            case "allow_coda_k": return "Âm cuối k (Đắk Lắk)";
            case "macro_enabled": return "Bật macro";
            case "macro_digits": return "Macro có số (mp3, k8s)";
            case "macro_undo": return "Backspace hoàn tác macro";
            case "macro_in_off_mode": return "Macro khi tắt tiếng Việt";
            case "auto_capitalize": return "Viết hoa đầu câu";
            case "smart_caps": return "Bảo vệ chữ HOA";
            case "auto_correct": return "Tự sửa lỗi gõ nhầm phím";
            case "edit_previous_word": return "Sửa từ vừa gõ";
            case "restore_after_space": return "Backspace sau dấu cách";
            case ADVANCED_RETURN_SYMBOL: return "Tự về bàn phím chữ sau ký tự đặc biệt";
            case ADVANCED_UNDERLINE: return "Hiện gạch chân chữ đang gõ";
            default: return key;
        }
    }

    @SuppressWarnings("deprecation")
    private String versionLabel() {
        try {
            PackageInfo info = getPackageManager().getPackageInfo(getPackageName(), 0);
            long build = Build.VERSION.SDK_INT >= Build.VERSION_CODES.P
                    ? info.getLongVersionCode() : info.versionCode;
            return "Phiên bản " + info.versionName + " · build " + build;
        } catch (Exception ignored) {
            return "Phiên bản 0.1.0";
        }
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }
}
