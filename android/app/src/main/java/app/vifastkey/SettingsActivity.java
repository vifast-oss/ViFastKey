package app.vifastkey;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.provider.Settings;
import android.view.Gravity;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
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
    private static final String[] BOOL_KEYS = {
            "spell_check", "auto_restore_english", "w_as_u", "bracket_keys",
            "quick_telex", "allow_coda_k", "macro_enabled", "macro_digits",
            "macro_undo", "macro_in_off_mode", "auto_capitalize", "smart_caps",
            "edit_previous_word", "restore_after_space",
    };
    private final List<CheckBox> checks = new ArrayList<>();
    private Spinner methodSpinner;
    private Spinner toneSpinner;
    private EditText macroEdit;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        ScrollView scroll = new ScrollView(this);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(dp(24), dp(48), dp(24), dp(32));
        scroll.addView(root);

        TextView title = new TextView(this);
        title.setText("ViFastKey");
        title.setTextSize(30);
        title.setGravity(Gravity.CENTER);
        root.addView(title);

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

        Button save = new Button(this);
        save.setText("Lưu cấu hình");
        save.setOnClickListener(v -> save());
        root.addView(save, matchWrap());

        setContentView(scroll);
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
        toneSpinner.setSelection("new".equals(getSharedPreferences("vifastkey", MODE_PRIVATE)
                .getString("tone_style", "new")) ? 1 : 0);
        for (CheckBox check : checks) {
            String key = (String) check.getTag();
            boolean fallback = !key.equals("bracket_keys") && !key.equals("quick_telex")
                    && !key.equals("allow_coda_k") && !key.equals("macro_digits")
                    && !key.equals("macro_in_off_mode") && !key.equals("auto_capitalize")
                    && !key.equals("smart_caps");
            check.setChecked(Prefs.bool(this, key, fallback));
        }
        macroEdit.setText(Prefs.macrosText(this));
    }

    private void save() {
        Prefs.method(this, METHODS[methodSpinner.getSelectedItemPosition()]);
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
            case "auto_restore_english": return "Giữ từ tiếng Anh";
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
            case "edit_previous_word": return "Sửa từ vừa gõ";
            case "restore_after_space": return "Backspace sau dấu cách";
            default: return key;
        }
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }
}
