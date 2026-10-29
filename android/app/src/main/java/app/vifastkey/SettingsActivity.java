package app.vifastkey;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.provider.Settings;
import android.view.Gravity;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;

/** Màn hình hướng dẫn bật bàn phím; không chứa logic gõ. */
public final class SettingsActivity extends Activity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(48, 64, 48, 48);
        root.setGravity(Gravity.CENTER_HORIZONTAL);

        TextView title = new TextView(this);
        title.setText("ViFastKey");
        title.setTextSize(30);
        title.setGravity(Gravity.CENTER);
        root.addView(title);

        TextView body = new TextView(this);
        body.setText("Bật bàn phím:\n1. Mở Keyboard & input methods\n2. Chọn ViFastKey\n"
                + "3. Chọn ViFastKey làm bàn phím hiện tại\n\n"
                + "Gõ Telex: tieengs → tiếng. Nút VI bật/tắt tiếng Việt.");
        body.setTextSize(17);
        body.setPadding(0, 32, 0, 32);
        root.addView(body);

        Button enable = new Button(this);
        enable.setText("Mở cài đặt bàn phím");
        enable.setOnClickListener(v ->
                startActivity(new Intent(Settings.ACTION_INPUT_METHOD_SETTINGS)));
        root.addView(enable, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        Button pick = new Button(this);
        pick.setText("Chọn bàn phím hiện tại");
        pick.setOnClickListener(v -> startActivity(new Intent(Settings.ACTION_INPUT_METHOD_SETTINGS)));
        root.addView(pick, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        setContentView(root);
    }
}
