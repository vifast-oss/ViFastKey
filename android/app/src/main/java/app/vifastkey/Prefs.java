package app.vifastkey;

import android.content.Context;
import android.content.SharedPreferences;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.LinkedHashMap;
import java.util.Map;

/** Cấu hình Android, serialize cùng schema JSON với macOS/engine Rust. */
public final class Prefs {
    private static final String NAME = "vifastkey";
    private static final String MACROS = "macros_text";

    private Prefs() {}

    public static SharedPreferences shared(Context context) {
        return context.getSharedPreferences(NAME, Context.MODE_PRIVATE);
    }

    private static SharedPreferences sp(Context context) {
        return shared(context);
    }

    public static String method(Context c) { return sp(c).getString("input_method", "telex"); }
    public static void method(Context c, String v) { sp(c).edit().putString("input_method", v).apply(); }

    public static boolean bool(Context c, String key, boolean fallback) {
        return sp(c).getBoolean(key, fallback);
    }
    public static void setBool(Context c, String key, boolean value) {
        sp(c).edit().putBoolean(key, value).apply();
    }

    public static String macrosText(Context c) {
        return sp(c).getString(MACROS, "ko=không\nvn=Việt Nam\ndc=được");
    }
    public static void macrosText(Context c, String value) {
        sp(c).edit().putString(MACROS, value).apply();
    }

    public static String configJson(Context c) {
        try {
            JSONObject o = new JSONObject();
            o.put("schema_version", 1);
            o.put("input_method", method(c));
            o.put("tone_style", sp(c).getString("tone_style", "new"));
            o.put("spell_check", bool(c, "spell_check", true));
            o.put("w_as_u", bool(c, "w_as_u", false));
            o.put("bracket_keys", bool(c, "bracket_keys", false));
            o.put("quick_telex", bool(c, "quick_telex", false));
            o.put("allow_coda_k", bool(c, "allow_coda_k", false));
            o.put("auto_restore_english", bool(c, "auto_restore_english", true));
            o.put("macro_enabled", bool(c, "macro_enabled", true));
            o.put("macro_digits", bool(c, "macro_digits", false));
            o.put("macro_undo", bool(c, "macro_undo", true));
            o.put("macro_in_off_mode", bool(c, "macro_in_off_mode", false));
            o.put("auto_capitalize", bool(c, "auto_capitalize", false));
            o.put("default_vietnamese", true);
            o.put("edit_previous_word", bool(c, "edit_previous_word", true));
            o.put("restore_after_space", bool(c, "restore_after_space", true));
            o.put("smart_caps", bool(c, "smart_caps", false));
            o.put("unicode_form", "NFC");
            JSONObject macros = new JSONObject();
            for (Map.Entry<String, String> e : parseMacros(macrosText(c)).entrySet()) {
                macros.put(e.getKey(), e.getValue());
            }
            o.put("macros", macros);
            return o.toString();
        } catch (Exception e) {
            return "{}";
        }
    }

    public static Map<String, String> parseMacros(String text) {
        Map<String, String> out = new LinkedHashMap<>();
        if (text == null) return out;
        for (String line : text.split("\\n")) {
            int i = line.indexOf('=');
            if (i <= 0) continue;
            String key = line.substring(0, i).trim().toLowerCase();
            String value = line.substring(i + 1).trim();
            if (!key.isEmpty() && !value.isEmpty()) out.put(key, value);
        }
        return out;
    }
}
