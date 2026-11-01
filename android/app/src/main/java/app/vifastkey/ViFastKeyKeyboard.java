package app.vifastkey;

import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.SharedPreferences;
import android.content.res.Configuration;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.drawable.ColorDrawable;
import android.graphics.drawable.Drawable;
import android.graphics.drawable.GradientDrawable;
import android.graphics.drawable.InsetDrawable;
import android.graphics.drawable.StateListDrawable;
import android.media.AudioManager;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.text.SpannableString;
import android.text.Spanned;
import android.text.TextUtils;
import android.text.style.RelativeSizeSpan;
import android.text.style.ImageSpan;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.HapticFeedbackConstants;
import android.view.MotionEvent;
import android.view.SoundEffectConstants;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowInsets;
import android.widget.AbsListView;
import android.widget.BaseAdapter;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.FrameLayout;
import android.widget.GridView;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.PopupWindow;
import android.widget.ScrollView;
import android.widget.TextView;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.ArrayList;
import java.util.Collections;
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
        void onSuggestion(String text);
        void onEmoji(String emoji);
        void onTextAction(String action);
        void onSettings();
        void onGlobe();
        void onVietnameseToggle();
    }

    private static final String[] LETTER_ROWS = {
            "q w e r t y u i o p",
            "a s d f g h j k l",
    };
    private static final String[] SYMBOL_ROWS_COMMON = {
            "1 2 3 4 5 6 7 8 9 0",
            "@ # đ _ & - + ( ) /",
            "PAGE * \" ' : ; ! ? BACK",
    };
    private static final String[] SYMBOL_ROWS_MORE = {
            "` ~ ^ | \\ [ ] { } <",
            "> = $ % € £ ¥ ° ± ×",
            "PAGE © ® ™ • — – … BACK",
    };
    private static final String[] EMOJI = {
            "😀", "😁", "😂", "🤣", "😊", "😍", "🥰", "😘", "😎", "🤔",
            "😅", "😢", "😭", "😡", "👍", "👎", "👏", "🙏", "💪", "❤️",
            "🔥", "⭐", "✨", "🎉", "☕", "🍜", "🚀", "💡", "✅", "❌",
            "🐱", "🐶", "🌸", "🌞", "🌙", "🇻🇳", "😴", "🥺", "🤝", "💯",
            "😃", "😄", "😆", "😇", "🙂", "🙃", "😉", "😌", "😗", "😙",
            "😚", "😋", "😛", "😝", "😜", "🤪", "🤨", "🧐", "🤓", "🥸",
            "🤩", "🥳", "😏", "😒", "😞", "😔", "😟", "😕", "🙁", "☹️",
            "😣", "😖", "😫", "😩", "😤", "😠", "🤬", "🤯", "😳", "🥵",
            "🥶", "😱", "😨", "😰", "😥", "😓", "🤗", "🫣", "🤭", "🫢",
            "🫡", "🤫", "🤥", "😶", "🫥", "😐", "😑", "😬", "🫠", "🙄",
            "😯", "😦", "😧", "😮", "😲", "🥱", "🤤", "😪", "😵", "😵‍💫",
            "🤐", "🥴", "🤢", "🤮", "🤧", "😷", "🤒", "🤕", "🤑", "🤠",
            "😈", "👿", "👹", "👺", "🤡", "💩", "👻", "💀", "☠️", "👽",
            "👾", "🤖", "🎃", "😺", "😸", "😹", "😻", "😼", "😽", "🙀",
            "😿", "😾", "👋", "🤚", "🖐️", "✋", "🖖", "🫱", "🫲", "🫳",
            "🫴", "👌", "🤌", "🤏", "✌️", "🤞", "🫰", "🤟", "🤘", "🤙",
            "👈", "👉", "👆", "🖕", "👇", "☝️", "🫵", "✊", "👊", "🤛",
            "🤜", "🙌", "🫶", "👐", "🤲", "✍️", "💅", "🤳", "🦾", "🦿",
            "🦵", "🦶", "👂", "🦻", "👃", "👀", "👁️", "👅", "👄", "🧠",
            "🫀", "🫁", "🦷", "🦴", "👤", "👥", "🗣️", "👶", "🧒", "👦",
            "👧", "🧑", "👱", "👨", "🧔", "👩", "🧓", "👴", "👵", "🙍",
            "🙎", "🙅", "🙆", "💁", "🙋", "🧏", "🙇", "🤦", "🤷", "👮",
            "🕵️", "💂", "🥷", "👷", "🫅", "🤴", "👸", "👳", "👲", "🧕",
            "🤰", "🤱", "👼", "🎅", "🤶", "🧙", "🧚", "🧛", "🧜", "🧝",
            "🧞", "🧟", "🧌", "💆", "💇", "🚶", "🧍", "🧎", "🏃", "💃",
            "🕺", "🕴️", "👯", "🧑‍🤝‍🧑", "👭", "👫", "👬", "💏", "💑", "👪",
            "👨‍👩‍👧", "👨‍👩‍👧‍👦", "👨‍👩‍👦", "👩‍👩‍👧", "👩‍👩‍👧‍👦", "👨‍👨‍👧", "👨‍👨‍👧‍👦", "👩‍👦", "👩‍👧", "👨‍👦",
            "👨‍👧", "🐵", "🐒", "🦍", "🦧", "🐕", "🦮", "🐩", "🐺", "🦊",
            "🦝", "🐈", "🦁", "🐯", "🐅", "🐆", "🐴", "🐎", "🦄", "🦓",
            "🦌", "🦬", "🐮", "🐂", "🐃", "🐄", "🐷", "🐖", "🐗", "🐽",
            "🐏", "🐑", "🐐", "🐪", "🐫", "🦙", "🦒", "🐘", "🦣", "🦏",
            "🦛", "🐭", "🐁", "🐀", "🐹", "🐰", "🐇", "🐿️", "🦫", "🦔",
            "🐻", "🐻‍❄️", "🐨", "🐼", "🦥", "🦦", "🦨", "🦘", "🦡", "🐾",
            "🦃", "🐔", "🐓", "🐣", "🐤", "🐥", "🐦", "🐧", "🕊️", "🦅",
            "🦆", "🦢", "🦉", "🦩", "🦚", "🦜", "🪶", "🐸", "🐊", "🐢",
            "🦎", "🐍", "🐲", "🐉", "🦕", "🦖", "🐳", "🐋", "🐬", "🦭",
            "🐟", "🐠", "🐡", "🦈", "🐙", "🐚", "🪸", "🐌", "🦋", "🐛",
            "🐜", "🐝", "🪲", "🐞", "🦗", "🪳", "🕷️", "🕸️", "🦂", "🦟",
            "🪰", "🪱", "🦠", "💐", "💮", "🪷", "🏵️", "🌹", "🥀", "🌺",
            "🌻", "🌼", "🌷", "🌱", "🪴", "🌲", "🌳", "🌴", "🌵", "🌾",
            "🌿", "☘️", "🍀", "🍁", "🍂", "🍃", "🪹", "🪺", "🍄", "🪨",
            "🌰", "🌍", "🌎", "🌏", "🌐", "🌑", "🌒", "🌓", "🌔", "🌕",
            "🌖", "🌗", "🌘", "🌚", "🌛", "🌜", "🌡️", "☀️", "🌝", "🪐",
            "🌟", "🌠", "🌌", "☁️", "🌤️", "⛅", "🌥️", "🌦️", "🌧️", "⛈️",
            "🌩️", "🌨️", "❄️", "☃️", "⛄", "🌬️", "💨", "🌪️", "🌫️", "🌊",
            "💧", "💦", "☔", "☂️", "🌈", "☄️", "💥", "🍇", "🍈", "🍉",
            "🍊", "🍋", "🍌", "🍍", "🥭", "🍎", "🍏", "🍐", "🍑", "🍒",
            "🍓", "🫐", "🥝", "🍅", "🫒", "🥥", "🥑", "🍆", "🥔", "🥕",
            "🌽", "🌶️", "🫑", "🥒", "🥬", "🥦", "🧄", "🧅", "🥜", "🫘",
            "🍞", "🥐", "🥖", "🫓", "🥨", "🥯", "🥞", "🧇", "🧀", "🍖",
            "🍗", "🥩", "🥓", "🍔", "🍟", "🍕", "🌭", "🥪", "🧆", "🌮",
            "🌯", "🫔", "🥙", "🍳", "🥘", "🍲", "🫕", "🥣", "🥗", "🍿",
            "🧈", "🧂", "🥫", "🍱", "🍘", "🍙", "🍚", "🍛", "🍝", "🍠",
            "🍢", "🍣", "🍤", "🍥", "🥮", "🍡", "🥟", "🥠", "🥡", "🦪",
            "🍦", "🍧", "🍨", "🍩", "🍪", "🎂", "🍰", "🧁", "🥧", "🍫",
            "🍬", "🍭", "🍮", "🍯", "🍼", "🥛", "🫖", "🍵", "🧃", "🥤",
            "🧋", "🍶", "🍺", "🍻", "🥂", "🍷", "🥃", "🍸", "🍹", "🧉",
            "🍾", "🍽️", "🍴", "🥄", "🔪", "🫙", "🚗", "🚕", "🚙", "🚌",
            "🚎", "🏎️", "🚓", "🚑", "🚒", "🚐", "🛻", "🚚", "🚛", "🚜",
            "🦯", "🦽", "🦼", "🛴", "🚲", "🛵", "🏍️", "🛺", "🚨", "🚔",
            "🚍", "🚘", "🚖", "🚡", "🚠", "🚟", "🚃", "🚋", "🚞", "🚝",
            "🚄", "🚅", "🚈", "🚂", "🚆", "🚇", "🚊", "🚉", "✈️", "🛫",
            "🛬", "🛩️", "💺", "🛰️", "🛸", "🚁", "🛶", "⛵", "🚤", "🛥️",
            "🛳️", "⛴️", "🚢", "⚓", "⛽", "🚧", "🚦", "🚥", "🚏", "🗺️",
            "🗿", "🗽", "🗼", "🏰", "🏯", "🏟️", "🎡", "🎢", "🎠", "⛲",
            "⛱️", "🏖️", "🏝️", "🏜️", "🌋", "⛰️", "🏔️", "🗻", "🏕️", "⛺",
            "🪂", "🏠", "🏡", "🏘️", "🏚️", "🏗️", "🏭", "🏢", "🏬", "🏣",
            "🏤", "🏥", "🏦", "🏨", "🏪", "🏫", "🏩", "💒", "🏛️", "⛪",
            "🕌", "🕍", "🛕", "🕋", "⛩️", "🛤️", "🛣️", "🗾", "🎑", "🏞️",
            "🌅", "🌄", "🌇", "🌆", "🏙️", "🌃", "🌉", "🌁", "⚽", "🏀",
            "🏈", "⚾", "🥎", "🎾", "🏐", "🏉", "🥏", "🎱", "🪀", "🏓",
            "🏸", "🏒", "🏑", "🥍", "🏏", "🪃", "🥅", "⛳", "🪁", "🏹",
            "🎣", "🤿", "🥊", "🥋", "🎽", "🛹", "🛼", "🛷", "⛸️", "🥌",
            "🎿", "⛷️", "🏂", "🏋️", "🤼", "🤸", "⛹️", "🤺", "🤾", "🏌️",
            "🏇", "🧘", "🧗", "🏄", "🏊", "🤽", "🚣", "🎪", "🤹", "🎭",
            "🎨", "🎬", "🎤", "🎧", "🎼", "🎹", "🥁", "🪘", "🎷", "🎺",
            "🎸", "🪕", "🎻", "🎲", "♟️", "🎯", "🎳", "🎮", "🎰", "🧩",
            "⌚", "📱", "📲", "💻", "⌨️", "🖥️", "🖨️", "🖱️", "🖲️", "💽",
            "💾", "💿", "📀", "📼", "📷", "📸", "📹", "🎥", "📽️", "🎞️",
            "📞", "☎️", "📟", "📠", "📺", "📻", "🎙️", "🎚️", "🎛️", "🧭",
            "⏱️", "⏲️", "⏰", "🕰️", "⌛", "⏳", "📡", "🔋", "🔌", "🔦",
            "🕯️", "🪔", "🧯", "🛢️", "💸", "💵", "💴", "💶", "💷", "🪙",
            "💰", "💳", "🧾", "💎", "⚖️", "🪜", "🧰", "🪛", "🔧", "🔨",
            "⚒️", "🛠️", "⛏️", "🪝", "🔩", "⚙️", "🧱", "⛓️", "🧲", "🔫",
            "💣", "🧨", "🪓", "🗡️", "⚔️", "🛡️", "🚬", "⚰️", "🪦", "⚱️",
            "🏺", "🔮", "📿", "🧿", "🪬", "💈", "⚗️", "🔭", "🔬", "🕳️",
            "🩹", "🩺", "🩻", "💊", "💉", "🩸", "🧬", "🧫", "🧪", "🧹",
            "🧺", "🧻", "🚽", "🚰", "🚿", "🛁", "🛀", "🧼", "🪥", "🪒",
            "🧽", "🪣", "🧴", "🛎️", "🔑", "🗝️", "🚪", "🪑", "🛋️", "🛏️",
            "🛌", "🧸", "🪆", "🖼️", "🪞", "🪟", "🛍️", "🛒", "🎁", "🎈",
            "🎏", "🎀", "🪄", "🪅", "🎊", "🎎", "🏮", "🎐", "🧧", "✉️",
            "📩", "📨", "📧", "💌", "📥", "📤", "📦", "🏷️", "📪", "📫",
            "📬", "📭", "📮", "🗳️", "✏️", "✒️", "🖋️", "🖊️", "🖌️", "🖍️",
            "📝", "💼", "📁", "📂", "🗂️", "📅", "📆", "🗒️", "🗓️", "📇",
            "📈", "📉", "📊", "📋", "📌", "📍", "📎", "🖇️", "📏", "📐",
            "✂️", "🗃️", "🗄️", "🗑️", "🔒", "🔓", "🔏", "🔐", "🧡", "💛",
            "💚", "💙", "💜", "🖤", "🤍", "🤎", "💔", "❣️", "💕", "💞",
            "💓", "💗", "💖", "💘", "💝", "💟", "☮️", "✝️", "☪️", "🕉️",
            "☸️", "✡️", "🔯", "🕎", "☯️", "☦️", "🛐", "⛎", "♈️", "♉️",
            "♊️", "♋️", "♌️", "♍️", "♎️", "♏️", "♐️", "♑️", "♒️", "♓️",
            "🆔", "⚛️", "🉑", "☢️", "☣️", "📴", "📳", "📵", "🚫", "⛔",
            "⚠️", "🚸", "⚧️", "✳️", "❇️", "✴️", "🆚", "🏧", "♻️", "♾️",
            "💲", "💱", "🌀", "💤", "🗯️", "💬", "💭", "🗨️", "👁️‍🗨️", "🔴",
            "🟠", "🟡", "🟢", "🔵", "🟣", "🟤", "⚫", "⚪", "🟥", "🟧",
            "🟨", "🟩", "🟦", "🟪", "🟫", "⬛", "⬜", "◼️", "◻️", "▪️",
            "▫️", "🔶", "🔷", "🔸", "🔹", "🔺", "🔻", "🔘", "🔳", "🔲",
            "🏁", "🚩", "🎌", "🏴", "🏳️", "🏳️‍🌈", "🏳️‍⚧️", "🏴‍☠️", "🇺🇸", "🇬🇧",
            "🇯🇵", "🇰🇷", "🇨🇳", "🇫🇷", "🇩🇪", "🇮🇹", "🇪🇸", "🇷🇺", "🇹🇭", "🇸🇬",
            "🇲🇾", "🇮🇩", "🇵🇭", "🇮🇳", "🇦🇺", "🇨🇦", "🇧🇷", "🇲🇽", "🇳🇱", "🇸🇪",
            "🇨🇭", "🇦🇪", "🇸🇦", "🇹🇷", "🇺🇦", "🇵🇱", "🇵🇹", "🇬🇷", "🇮🇪", "🇳🇴",
            "🇩🇰", "🇫🇮", "🇳🇿", "🇿🇦", "🇦🇷", "🇨🇱", "🇨🇴", "🇵🇪", "🇪🇬", "🇮🇱",
            "🇱🇦", "🇰🇭", "🇲🇲", "🇳🇵", "🇱🇰", "🇧🇩", "🇵🇰", "🥲", "🙈", "🙉",
            "🙊",
    };
    private static final String CLIP_PREFS = "vifastkey_clipboard";
    private static final String CLIP_HISTORY = "history";
    private static final int CLIP_LIMIT = 20;
    private static final String EMOJI_PREFS = "vifastkey_emoji";
    private static final String EMOJI_USAGE = "usage";
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
    private final AudioManager audio;
    private final ClipboardManager clipboard;
    private final Handler repeatHandler = new Handler(Looper.getMainLooper());
    private final List<Button> letterButtons = new ArrayList<>();
    private final List<TextView> suggestionViews = new ArrayList<>();
    private final List<String> suggestionValues = new ArrayList<>();
    private final List<String> clipboardHistory = new ArrayList<>();
    private final Map<String, Integer> emojiCounts = new HashMap<>();
    private final Map<String, Long> emojiLastUsed = new HashMap<>();
    private PopupWindow clipboardPopup;
    private PopupWindow quickPopup;
    private long clipboardDismissedAt;
    private PopupWindow variantPopup;
    private Button variantAnchor;
    private final List<String> variantValues = new ArrayList<>();
    private final List<TextView> variantViews = new ArrayList<>();
    private LinearLayout variantRow;
    private boolean variantAbove;
    private int variantSelected;
    private LinearLayout suggestions;
    private HorizontalScrollView suggestionScroll;
    private Button vietnameseButton;
    private Button shiftButton;
    private boolean shift;
    private boolean numberRow;
    private boolean caps;
    private boolean autoShift;
    private boolean symbols;
    private boolean emojiPanel;
    private int emojiTargetHeightDp = 308;
    private boolean numeric;
    private int symbolPage = 1;
    private boolean vietnamese = true;
    private String inputMethod = "telex";
    private boolean dark;
    private boolean labelsShifted;
    private boolean shiftLabelActive;
    private boolean shiftLabelCaps;
    private int heightPercent = 100;
    private int repeatCount;
    private Runnable repeatRunnable;

    @SuppressWarnings("deprecation")
    ViFastKeyKeyboard(Context context, Listener listener) {
        super(context);
        this.context = context;
        this.listener = listener;
        audio = context.getSystemService(AudioManager.class);
        clipboard = context.getSystemService(ClipboardManager.class);
        loadClipboardHistory();
        loadEmojiUsage();
        numberRow = Prefs.bool(context, "show_number_row", false);
        heightPercent = Math.max(80, Math.min(160,
                Prefs.shared(context).getInt("keyboard_height_percent", 100)));
        dark = (context.getResources().getConfiguration().uiMode
                & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
        setOrientation(VERTICAL);
        setPadding(dp(2), dp(3), dp(2), dp(5));
        setBackgroundColor(dark ? 0xFF202124 : 0xFFE8EAED);
        setOnApplyWindowInsetsListener((view, insets) -> {
            int bottom = Build.VERSION.SDK_INT >= Build.VERSION_CODES.R
                    ? insets.getInsets(WindowInsets.Type.navigationBars()).bottom
                    : insets.getSystemWindowInsetBottom();
            view.setPadding(dp(2), dp(3), dp(2), dp(5) + bottom);
            return insets;
        });
        rebuild();
    }

    @Override
    public boolean dispatchTouchEvent(MotionEvent ev) {
        if (ev.getActionMasked() == MotionEvent.ACTION_DOWN) {
            if (clipboardPopup != null && clipboardPopup.isShowing()) {
                dismissClipboardMenu();
            }
            if (variantPopup != null) {
                dismissVariant();
            }
        }
        return super.dispatchTouchEvent(ev);
    }

    void setSuggestions(List<String> values) {
        if (suggestions == null) return;
        List<String> next = values == null ? java.util.Collections.emptyList() : values;
        boolean visible = suggestionScroll.getVisibility() == VISIBLE;
        if (next.isEmpty()) {
            suggestionValues.clear();
            if (visible) {
                // Giữ chiều cao cố định để bàn phím không nhảy khi gợi ý xuất hiện.
                suggestionScroll.setVisibility(INVISIBLE);
                suggestions.setVisibility(INVISIBLE);
            }
            return;
        }
        if (visible && next.equals(suggestionValues)) return;
        suggestionValues.clear();
        suggestionValues.addAll(next);
        suggestionScroll.setVisibility(VISIBLE);
        suggestions.setVisibility(VISIBLE);
        while (suggestionViews.size() < next.size()) {
            TextView item = new TextView(context);
            item.setTextSize(18);
            item.setGravity(Gravity.CENTER);
            item.setPadding(dp(14), dp(5), dp(14), dp(5));
            item.setTextColor(dark ? 0xFFE8EAED : 0xFF202124);
            wireTouch(item);
            item.setOnClickListener(v -> listener.onSuggestion(String.valueOf(v.getTag())));
            suggestionViews.add(item);
            suggestions.addView(item);
        }
        for (int i = 0; i < suggestionViews.size(); i++) {
            TextView item = suggestionViews.get(i);
            if (i < next.size()) {
                String value = next.get(i);
                item.setTag(value);
                if (!value.contentEquals(item.getText())) item.setText(value);
                item.setVisibility(VISIBLE);
            } else {
                item.setVisibility(GONE);
            }
        }
    }

    void setVietnamese(boolean enabled) {
        vietnamese = enabled;
        if (vietnameseButton != null) {
            vietnameseButton.setText(enabled ? "VI" : "EN");
            vietnameseButton.setTextColor(enabled ? Color.WHITE : (dark ? 0xFFE8EAED : 0xFF202124));
            vietnameseButton.setBackground(smallKeyBackground(enabled ? 0xFFC2410C : keyColor()));
        }
    }

    void setShift(boolean enabled) {
        shift = enabled;
        updateLetters();
    }

    /** Shift tự động ở đầu câu; không ghi đè shift/caps người dùng đang bật. */
    void setAutoShift(boolean enabled) {
        if (autoShift == enabled) return;
        autoShift = enabled;
        updateLetters();
    }

    void setMethod(String method) {
        String next = method == null ? "telex" : method;
        if (next.equals(inputMethod)) return;
        inputMethod = next;
        if (emojiPanel) emojiTargetHeightDp = baseLayoutHeightDp();
        rebuild();
    }

    void setNumberRow(boolean enabled) {
        if (numberRow == enabled) return;
        numberRow = enabled;
        if (emojiPanel) emojiTargetHeightDp = baseLayoutHeightDp();
        rebuild();
    }

    void setHeightScale(int percent) {
        int next = Math.max(80, Math.min(160, percent));
        if (heightPercent == next) return;
        heightPercent = next;
        if (emojiPanel) emojiTargetHeightDp = baseLayoutHeightDp();
        rebuild();
    }

    void setNumericMode(boolean enabled) {
        numeric = enabled;
        rebuild();
    }

    /** Vào field/app mới: luôn mở bằng layout chữ, trừ field số. */
    void resetLayoutForField(boolean numericMode) {
        emojiPanel = false;
        symbols = false;
        symbolPage = 1;
        numeric = numericMode;
        rebuild();
    }

    private void rebuild() {
        dismissVariant();
        if (clipboardPopup != null) {
            clipboardPopup.dismiss();
            clipboardPopup = null;
        }
        dismissQuickSettings();
        removeAllViews();
        letterButtons.clear();
        suggestionViews.clear();
        suggestionValues.clear();
        shiftButton = null;
        labelsShifted = false;
        shiftLabelActive = false;
        shiftLabelCaps = false;
        labelsShifted = !isShifted();
        addTopBar();
        if (emojiPanel) {
            // Tab emoji không có gợi ý từ, bỏ luôn hàng trống 36dp phía trên.
            suggestions = null;
            suggestionScroll = null;
        } else {
            addSuggestionsBar();
        }
        if (numeric) {
            addNumberPadRows();
        } else if (emojiPanel) {
            addEmojiPanel();
        } else if (symbols) {
            if (symbolPage == 0) {
                addNumberPadRows();
            } else {
                addSymbolRows();
            }
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
        Button quick = smallKey("", "Cài đặt nhanh");
        setKeyIcon(quick, R.drawable.ic_quick_settings, textColor(), 20);
        quick.setOnClickListener(v -> showQuickSettings(quick));
        row.addView(quick);
        Button gear = smallKey("⚙", "Cấu hình");
        gear.setOnClickListener(v -> listener.onSettings());
        row.addView(gear);
        Button globe = smallKey("🌐", "Chọn bàn phím");
        globe.setOnClickListener(v -> listener.onGlobe());
        row.addView(globe);
        Button emojiKey = smallKey(numeric || emojiPanel ? "ABC" : "😊", "Emoji");
        emojiKey.setOnClickListener(v -> {
            if (numeric) {
                numeric = false;
                symbols = false;
                emojiPanel = false;
            } else if (emojiPanel) {
                emojiPanel = false;
                symbols = false;
            } else {
                emojiTargetHeightDp = baseLayoutHeightDp();
                emojiPanel = true;
                symbols = false;
                numeric = false;
            }
            rebuild();
        });
        row.addView(emojiKey);
        Button clip = smallKey("📋", "Bảng nhớ tạm");
        clip.setOnClickListener(v -> showClipboardMenu(clip));
        row.addView(clip);
        Button sym = smallKey(numeric || symbols ? "ABC" : "?123", "Ký tự");
        sym.setOnClickListener(v -> {
            if (numeric) {
                numeric = false;
                symbols = false;
            } else if (symbols) {
                symbols = false;
            } else {
                symbols = true;
                emojiPanel = false;
            }
            rebuild();
        });
        row.addView(sym);
        vietnameseButton = smallKey("VI", "Bật/tắt tiếng Việt");
        vietnameseButton.setOnClickListener(v -> listener.onVietnameseToggle());
        row.addView(vietnameseButton);
        addView(row, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(36)));
    }

    private void addSuggestionsBar() {
        suggestionScroll = new HorizontalScrollView(context);
        suggestionScroll.setHorizontalScrollBarEnabled(false);
        suggestionScroll.setVisibility(INVISIBLE);
        suggestions = new LinearLayout(context);
        suggestions.setOrientation(HORIZONTAL);
        suggestions.setGravity(Gravity.CENTER_VERTICAL);
        suggestionScroll.addView(suggestions);
        addView(suggestionScroll, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(36)));
    }

    private void addLetterRows() {
        if (showNumberRow()) {
            LinearLayout numbers = new LinearLayout(context);
            numbers.setOrientation(HORIZONTAL);
            for (String key : "1 2 3 4 5 6 7 8 9 0".split(" ")) {
                Button button = actionKey(key, 1f);
                button.setTextSize(18);
                numbers.addView(button);
            }
            addView(numbers, new LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, dp(numberRowHeightDp())));
        }
        for (int rowIndex = 0; rowIndex < LETTER_ROWS.length; rowIndex++) {
            LinearLayout row = new LinearLayout(context);
            row.setOrientation(HORIZONTAL);
            row.setGravity(Gravity.CENTER);
            if (rowIndex == 1) {
                // Hàng home-row của Gboard hẹp hơn và được căn giữa.
                row.setPadding(dp(20.5f), 0, dp(20.5f), 0);
            }
            String[] items = LETTER_ROWS[rowIndex].split(" ");
            for (String key : items) {
                row.addView(letterKey(key));
            }
            addView(row, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(rowHeightDp())));
        }
        LinearLayout third = new LinearLayout(context);
        third.setOrientation(HORIZONTAL);
        third.setGravity(Gravity.CENTER);
        shiftButton = actionKey("", 1.5f);
        shiftButton.setTextSize(24);
        shiftButton.setContentDescription("Shift");
        setKeyIcon(shiftButton, R.drawable.ic_shift, textColor(), 24);
        shiftButton.setOnClickListener(v -> {
            if (caps) {
                caps = false;
                shift = false;
            } else if (shift) {
                caps = true;
                shift = true;
            } else if (autoShift) {
                autoShift = false;
            } else {
                shift = true;
            }
            updateLetters();
        });
        shiftButton.setOnLongClickListener(v -> {
            caps = !caps;
            shift = caps;
            autoShift = false;
            updateLetters();
            return true;
        });
        third.addView(shiftButton);
        for (String key : "z x c v b n m".split(" ")) {
            third.addView(letterKey(key));
        }
        Button backspace = actionKey("", 1.5f);
        backspace.setTextSize(24);
        backspace.setContentDescription("Xoá");
        setKeyIcon(backspace, R.drawable.ic_backspace, textColor(), 26);
        backspace.setOnTouchListener((v, event) -> handleRepeat(v, event, "Backspace"));
        third.addView(backspace);
        addView(third, new LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(rowHeightDp())));
    }

    private void addSymbolRows() {
        String[] rows = symbolPage == 1 ? SYMBOL_ROWS_COMMON : SYMBOL_ROWS_MORE;
        for (String keys : rows) {
            LinearLayout row = new LinearLayout(context);
            row.setOrientation(HORIZONTAL);
            for (String key : keys.split(" ")) {
                if ("PAGE".equals(key)) {
                    Button page = actionKey(symbolPage == 1 ? "=\\" : "?123", 1.5f);
                    page.setTextSize(18);
                    page.setContentDescription("Trang ký tự khác");
                    page.setOnClickListener(v -> {
                        symbolPage = symbolPage == 1 ? 2 : 1;
                        rebuild();
                    });
                    row.addView(page);
                } else if ("BACK".equals(key)) {
                    row.addView(backspaceKey(1.5f));
                } else {
                    row.addView(isDigitKey(key) ? actionKey(key, 1f) : symbolKey(key, 1f));
                }
            }
            addView(row, new LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, dp(rowHeightDp())));
        }
    }

    private void addNumberPadRows() {
        String[][] rows = {
                {"+", "1", "2", "3", "%"},
                {"-", "4", "5", "6"},
                {"*", "7", "8", "9", "/"},
        };
        for (int r = 0; r < rows.length; r++) {
            LinearLayout row = new LinearLayout(context);
            row.setOrientation(HORIZONTAL);
            String[] keys = rows[r];
            for (int i = 0; i < keys.length; i++) {
                boolean sideKey = i == 0 || (keys.length == 5 && i == 4);
                float weight = sideKey ? 0.9f : 1f;
                row.addView(isDigitKey(keys[i])
                        ? actionKey(keys[i], weight)
                        : symbolKey(keys[i], weight));
            }
            if (r == 1) {
                row.addView(backspaceKey(0.9f));
            }
            addView(row, new LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, dp(rowHeightDp())));
        }
    }

    private Button backspaceKey(float weight) {
        Button backspace = actionKey("", weight);
        backspace.setTextSize(24);
        backspace.setContentDescription("Xoá");
        setKeyIcon(backspace, R.drawable.ic_backspace, textColor(), 26);
        backspace.setOnTouchListener((v, event) -> handleRepeat(v, event, "Backspace"));
        return backspace;
    }

    private void addEmojiPanel() {
        List<String> frequent = frequentEmoji(6);
        int frequentHeight = frequent.isEmpty() ? 0 : 44;
        if (!frequent.isEmpty()) {
            LinearLayout row = new LinearLayout(context);
            row.setOrientation(HORIZONTAL);
            row.setGravity(Gravity.CENTER_VERTICAL);
            TextView heading = new TextView(context);
            heading.setText("Hay dùng");
            heading.setTextSize(12);
            heading.setTextColor(dark ? 0xFF9AA0A6 : 0xFF5F6368);
            heading.setPadding(dp(10), 0, dp(8), 0);
            row.addView(heading, new LayoutParams(
                    ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.MATCH_PARENT));
            View spacer = new View(context);
            row.addView(spacer, new LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, 1f));
            for (String emoji : frequent) {
                Button button = createEmojiButton(emoji);
                row.addView(button, new LayoutParams(dp(44), dp(frequentHeight)));
            }
            addView(row, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(frequentHeight)));
        }
        GridView grid = new GridView(context);
        grid.setNumColumns(8);
        grid.setStretchMode(GridView.STRETCH_COLUMN_WIDTH);
        grid.setHorizontalSpacing(0);
        grid.setVerticalSpacing(0);
        grid.setSelector(new ColorDrawable(Color.TRANSPARENT));
        grid.setDrawSelectorOnTop(false);
        grid.setVerticalScrollBarEnabled(true);
        grid.setScrollbarFadingEnabled(false);
        grid.setFocusable(false);
        grid.setAdapter(new EmojiAdapter());
        // Bù đúng chiều cao thanh gợi ý 36dp của layout chữ (tab emoji không
        // hiện thanh này) và hàng Hay dùng, để tổng cao bằng layout đang dùng;
        // lưới vẫn lộ một phần hàng kế tiếp.
        int fixedHeight = 36 + frequentHeight + rowHeightDp();
        int gridHeight = Math.max(rowHeightDp() * 2, emojiTargetHeightDp - fixedHeight);
        addView(grid, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(gridHeight)));
    }

    private Button createEmojiButton(String emoji) {
        Button button = new Button(context);
        button.setText(emoji);
        button.setTag(emoji);
        button.setTextSize(24);
        button.setAllCaps(false);
        button.setGravity(Gravity.CENTER);
        button.setMinWidth(0);
        button.setMinimumWidth(0);
        button.setPadding(0, 0, 0, 0);
        button.setIncludeFontPadding(false);
        button.setTextColor(textColor());
        button.setBackground(keyBackground(keyColor()));
        button.setSoundEffectsEnabled(false);
        // Không rung khi bắt đầu chạm/kéo để cuộn; chỉ rung lúc thực sự chọn emoji.
        button.setOnClickListener(v -> {
            tap(v);
            String value = String.valueOf(v.getTag());
            recordEmoji(value);
            listener.onEmoji(value);
            returnToLetters();
        });
        return button;
    }

    private final class EmojiAdapter extends BaseAdapter {
        @Override public int getCount() { return EMOJI.length; }
        @Override public Object getItem(int position) { return EMOJI[position]; }
        @Override public long getItemId(int position) { return position; }

        @Override
        public View getView(int position, View convertView, ViewGroup parent) {
            Button button;
            if (convertView instanceof Button) {
                button = (Button) convertView;
            } else {
                button = createEmojiButton("");
            }
            String emoji = EMOJI[position];
            button.setTag(emoji);
            if (!emoji.contentEquals(button.getText())) button.setText(emoji);
            button.setLayoutParams(new AbsListView.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, dp(rowHeightDp())));
            return button;
        }
    }

    private void loadEmojiUsage() {
        try {
            SharedPreferences prefs = context.getSharedPreferences(EMOJI_PREFS, Context.MODE_PRIVATE);
            JSONArray array = new JSONArray(prefs.getString(EMOJI_USAGE, "[]"));
            for (int i = 0; i < array.length(); i++) {
                JSONObject item = array.optJSONObject(i);
                if (item == null) continue;
                String emoji = item.optString("e", "");
                int count = item.optInt("c", 0);
                long last = item.optLong("t", 0);
                if (!emoji.isEmpty() && count > 0) {
                    emojiCounts.put(emoji, count);
                    emojiLastUsed.put(emoji, last);
                }
            }
        } catch (Exception ignored) {
            // Bắt đầu lại nếu dữ liệu cũ hỏng.
        }
    }

    private void saveEmojiUsage() {
        try {
            JSONArray array = new JSONArray();
            for (Map.Entry<String, Integer> entry : emojiCounts.entrySet()) {
                JSONObject item = new JSONObject();
                item.put("e", entry.getKey());
                item.put("c", entry.getValue());
                item.put("t", emojiLastUsed.getOrDefault(entry.getKey(), 0L));
                array.put(item);
            }
            context.getSharedPreferences(EMOJI_PREFS, Context.MODE_PRIVATE)
                    .edit().putString(EMOJI_USAGE, array.toString()).apply();
        } catch (Exception ignored) {
            // Không lưu được cũng không ảnh hưởng việc gõ.
        }
    }

    private void recordEmoji(String emoji) {
        if (emoji == null || emoji.isEmpty()) return;
        emojiCounts.put(emoji, emojiCounts.getOrDefault(emoji, 0) + 1);
        emojiLastUsed.put(emoji, System.currentTimeMillis());
        saveEmojiUsage();
    }

    private List<String> frequentEmoji(int limit) {
        List<String> values = new ArrayList<>(emojiCounts.keySet());
        Collections.sort(values, (a, b) -> {
            int byCount = Integer.compare(
                    emojiCounts.getOrDefault(b, 0), emojiCounts.getOrDefault(a, 0));
            if (byCount != 0) return byCount;
            return Long.compare(
                    emojiLastUsed.getOrDefault(b, 0L), emojiLastUsed.getOrDefault(a, 0L));
        });
        return values.subList(0, Math.min(limit, values.size()));
    }

    private void addBottomRow() {
        LinearLayout row = new LinearLayout(context);
        row.setOrientation(HORIZONTAL);
        row.setGravity(Gravity.CENTER);

        boolean numberPad = numeric || (symbols && symbolPage == 0);
        Button mode = actionKey(numeric || symbols || emojiPanel ? "ABC" : "?123", 1.5f);
        mode.setOnClickListener(v -> {
            if (emojiPanel || numeric || symbols) {
                emojiPanel = false;
                numeric = false;
                symbols = false;
            } else {
                symbols = true;
            }
            rebuild();
        });
        row.addView(mode);
        row.addView(commaKey(1f));

        if (numberPad) {
            row.addView(contextKey(1.2f));
            row.addView(actionKey("0", 2.2f));
            row.addView(symbolKey("=", 1f));
            row.addView(symbolKey(".", 1f));
        } else {
            row.addView(contextKey(1f));
            Button space = actionKey("ViFastKey", 4f);
            space.setTextSize(16);
            space.setContentDescription("Dấu cách");
            space.setOnClickListener(v -> listener.onKey(" "));
            row.addView(space);
            row.addView(periodKey(1f));
        }
        row.addView(enterKey(1.46f));
        addView(row, new LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(rowHeightDp())));
    }

    private Button commaKey(float weight) {
        Button comma = actionKey("", weight);
        SpannableString label = new SpannableString("😊\n,");
        label.setSpan(new RelativeSizeSpan(0.55f), 0, 2, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
        comma.setText(label);
        comma.setTextSize(20);
        comma.setLineSpacing(0, 0.8f);
        comma.setPadding(0, 0, 0, dp(10));
        comma.setContentDescription("Dấu phẩy; giữ để mở emoji");
        comma.setOnClickListener(v -> {
            listener.onKey(",");
            if (emojiPanel || ((symbols || numeric) && returnAfterSymbol())) returnToLetters();
        });
        comma.setOnLongClickListener(v -> {
            openEmoji();
            return true;
        });
        return comma;
    }

    private Button contextKey(float weight) {
        boolean numberPad = numeric || (symbols && symbolPage == 0);
        if (numberPad) {
            Button page = actionKey("!?#", weight);
            page.setTextSize(16);
            page.setContentDescription("Mở trang ký tự");
            page.setOnClickListener(v -> {
                symbols = true;
                symbolPage = 1;
                numeric = false;
                emojiPanel = false;
                rebuild();
            });
            return page;
        }
        if (symbols) {
            Button page = actionKey("12\n34", weight);
            page.setTextSize(12);
            page.setContentDescription("Mở bàn phím số");
            page.setOnClickListener(v -> {
                symbolPage = 0;
                numeric = false;
                rebuild();
            });
            return page;
        }
        Button globe = actionKey("🌐", weight);
        globe.setTextSize(20);
        globe.setContentDescription("Chọn bàn phím");
        globe.setOnClickListener(v -> listener.onGlobe());
        return globe;
    }

    private Button symbolKey(String label, float weight) {
        Button button = actionKey(label, weight);
        button.setOnClickListener(v -> {
            listener.onKey(label);
            if (returnAfterSymbol()) returnToLetters();
        });
        return button;
    }

    private Button periodKey(float weight) {
        Button button = actionKey(".", weight);
        button.setOnClickListener(v -> {
            listener.onKey(".");
            if (emojiPanel || ((symbols || numeric) && returnAfterSymbol())) returnToLetters();
        });
        return button;
    }

    private static boolean isDigitKey(String key) {
        return key.length() == 1 && key.charAt(0) >= '0' && key.charAt(0) <= '9';
    }

    private Button enterKey(float weight) {
        Button enter = actionKey("", weight);
        enter.setContentDescription("Enter");
        setKeyIcon(enter, R.drawable.ic_enter, textColor(), 24);
        enter.setOnClickListener(v -> listener.onKey("Enter"));
        return enter;
    }

    private void openEmoji() {
        if (!emojiPanel) emojiTargetHeightDp = baseLayoutHeightDp();
        emojiPanel = true;
        symbols = false;
        numeric = false;
        rebuild();
    }

    /**
     * Chiều cao layout chữ/ký tự/số đang dùng (dp), để tab emoji cao bằng
     * và không làm bàn phím nhảy khi chuyển tab.
     */
    private int baseLayoutHeightDp() {
        int height = 36 + 36 + rowHeightDp() * (LETTER_ROWS.length + 1) + rowHeightDp();
        if (!numeric && !symbols && showNumberRow()) height += numberRowHeightDp();
        return height;
    }

    private int rowHeightDp() {
        return Math.max(40, Math.round(59 * heightPercent / 100f));
    }

    private int numberRowHeightDp() {
        return Math.max(34, Math.round(48 * heightPercent / 100f));
    }

    private boolean showNumberRow() {
        return "vni".equals(inputMethod) || numberRow;
    }

    private boolean returnAfterSymbol() {
        return Prefs.bool(context, "return_to_letters_after_symbol", false);
    }

    private void returnToLetters() {
        emojiPanel = false;
        symbols = false;
        numeric = false;
        rebuild();
    }

    private Button letterKey(String lower) {
        LetterButton button = new LetterButton(context);
        actionKey(lower, 1f, button);
        button.setTextSize(24);
        button.hint = topNumber(lower);
        letterButtons.add(button);
        button.setTag(lower);
        button.setOnClickListener(v -> {
            String key = isShifted() ? lower.toUpperCase() : lower;
            if (!caps) {
                shift = false;
                autoShift = false;
            }
            updateLetters();
            listener.onKey(key);
        });
        button.setOnTouchListener((v, event) -> handleLetterTouch(v, event));
        button.setOnLongClickListener(v -> {
            String number = topNumber(lower);
            String[] accents = ACCENTS.get(lower);
            if (number != null && (accents == null || accents.length == 0)) {
                listener.onDirect(number);
            } else if ((accents != null && accents.length > 0) || number != null) {
                showVariants(button, number, accents);
            }
            return true;
        });
        return button;
    }

    private Button actionKey(String label, float weight) {
        return actionKey(label, weight, new Button(context));
    }

    private Button actionKey(String label, float weight, Button button) {
        button.setText(label);
        button.setTextSize(20);
        button.setAllCaps(false);
        button.setGravity(Gravity.CENTER);
        button.setMinWidth(0);
        button.setMinimumWidth(0);
        button.setPadding(0, 0, 0, 0);
        button.setIncludeFontPadding(false);
        button.setTextColor(dark ? 0xFFE8EAED : 0xFF202124);
        button.setBackground(keyBackground(keyColor()));
        LayoutParams params = new LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, weight);
        button.setLayoutParams(params);
        wireTouch(button);
        button.setOnClickListener(v -> listener.onKey(label));
        return button;
    }

    private Button smallKey(String label, String description) {
        Button button = new Button(context);
        button.setText(label);
        button.setTextSize(17);
        button.setContentDescription(description);
        button.setAllCaps(false);
        button.setMinWidth(0);
        button.setMinimumWidth(0);
        button.setPadding(0, 0, 0, 0);
        button.setTextColor(dark ? 0xFFE8EAED : 0xFF202124);
        button.setBackground(smallKeyBackground(keyColor()));
        LayoutParams params = new LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, 1f);
        params.setMargins(dp(1), dp(2), dp(1), dp(2));
        button.setLayoutParams(params);
        wireTouch(button);
        return button;
    }

    private boolean handleRepeat(View view, MotionEvent event, String key) {
        switch (event.getActionMasked()) {
            case MotionEvent.ACTION_DOWN:
                repeatHandler.removeCallbacksAndMessages(null);
                repeatCount = 0;
                view.setPressed(true);
                tap(view);
                listener.onKey(key);
                repeatRunnable = new Runnable() {
                    @Override public void run() {
                        listener.onKey(key);
                        repeatCount++;
                        int delay = Math.max(22, 65 - repeatCount * 4);
                        repeatHandler.postDelayed(this, delay);
                    }
                };
                repeatHandler.postDelayed(repeatRunnable, 420);
                return true;
            case MotionEvent.ACTION_UP:
            case MotionEvent.ACTION_CANCEL:
                repeatHandler.removeCallbacksAndMessages(null);
                repeatRunnable = null;
                repeatCount = 0;
                view.setPressed(false);
                return true;
            default:
                return false;
        }
    }

    private boolean isShifted() {
        return shift || caps || autoShift;
    }

    /** Phản hồi ngay khi chạm xuống để bàn phím nhạy như Gboard. */
    private void wireTouch(View view) {
        view.setSoundEffectsEnabled(false);
        view.setOnTouchListener((v, event) -> {
            if (event.getActionMasked() == MotionEvent.ACTION_DOWN) tap(v);
            return false;
        });
    }

    private void tap(View view) {
        haptic(view);
        if (audio != null) audio.playSoundEffect(SoundEffectConstants.CLICK);
    }

    private int textColor() {
        return dark ? 0xFFE8EAED : 0xFF202124;
    }

    private void setKeyIcon(Button button, int resId, int tint, float sizeDp) {
        Drawable icon = context.getDrawable(resId);
        if (icon == null) return;
        icon = icon.mutate();
        icon.setTint(tint);
        int size = dp(sizeDp);
        icon.setBounds(0, 0, size, size);
        SpannableString label = new SpannableString(" ");
        label.setSpan(new ImageSpan(icon, ImageSpan.ALIGN_CENTER),
                0, 1, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
        button.setText(label);
    }

    private void haptic(View view) {
        if (view != null) {
            view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP);
        }
    }

    private boolean handleLetterTouch(View view, MotionEvent event) {
        switch (event.getActionMasked()) {
            case MotionEvent.ACTION_DOWN:
                tap(view);
                return false;
            case MotionEvent.ACTION_MOVE:
                if (variantPopup != null && variantAnchor == view) {
                    updateVariantSelection(event.getRawX(), event.getRawY());
                    return true;
                }
                return false;
            case MotionEvent.ACTION_UP:
                if (variantPopup != null && variantAnchor == view) {
                    view.setPressed(false);
                    commitVariant(event.getRawX(), event.getRawY());
                    return true;
                }
                return false;
            case MotionEvent.ACTION_CANCEL:
                if (variantPopup != null && variantAnchor == view) {
                    view.setPressed(false);
                    commitVariant(event.getRawX(), event.getRawY());
                    return true;
                }
                return false;
            default:
                return false;
        }
    }

    private void showVariants(Button anchor, String number, String[] accents) {
        dismissVariant();
        variantValues.clear();
        if (number != null) variantValues.add(number);
        if (accents != null) {
            for (String accent : accents) {
                variantValues.add(isShifted() ? accent.toUpperCase() : accent);
            }
        }
        if (variantValues.isEmpty()) return;

        int panel = dark ? 0xFF2D2E31 : 0xFFFFFFFF;
        LinearLayout row = new LinearLayout(context);
        row.setOrientation(HORIZONTAL);
        row.setPadding(dp(5), dp(5), dp(5), dp(5));
        row.setBackground(panelBackground(panel));
        row.setElevation(dp(6));
        variantViews.clear();
        for (String value : variantValues) {
            TextView choice = new TextView(context);
            choice.setText(value);
            choice.setTextSize(19);
            choice.setGravity(Gravity.CENTER);
            choice.setMinWidth(dp(36));
            choice.setPadding(dp(7), dp(6), dp(7), dp(6));
            row.addView(choice);
            variantViews.add(choice);
        }
        variantSelected = 0;
        updateVariantHighlight();
        row.measure(View.MeasureSpec.makeMeasureSpec(
                        getResources().getDisplayMetrics().widthPixels - dp(16),
                        View.MeasureSpec.AT_MOST),
                View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED));
        int width = row.getMeasuredWidth();
        int height = row.getMeasuredHeight();
        int[] location = new int[2];
        anchor.getLocationOnScreen(location);
        variantAbove = location[1] - height - dp(6) >= dp(8);
        int yoff = variantAbove ? -(anchor.getHeight() + height + dp(6)) : dp(6);
        variantAnchor = anchor;
        variantRow = row;
        variantPopup = new PopupWindow(row, width, height, false);
        variantPopup.setTouchable(false);
        variantPopup.setFocusable(false);
        variantPopup.setOutsideTouchable(false);
        variantPopup.setTouchModal(false);
        variantPopup.setClippingEnabled(false);
        variantPopup.setBackgroundDrawable(new ColorDrawable(Color.TRANSPARENT));
        variantPopup.showAsDropDown(anchor, (anchor.getWidth() - width) / 2, yoff);
    }

    private void updateVariantSelection(float rawX, float rawY) {
        if (variantPopup == null || variantAnchor == null || variantRow == null) return;
        int[] anchorLoc = new int[2];
        variantAnchor.getLocationOnScreen(anchorLoc);
        boolean towardPopup = variantAbove
                ? rawY < anchorLoc[1] + dp(8)
                : rawY > anchorLoc[1] + variantAnchor.getHeight() - dp(8);
        if (!towardPopup) return;

        int[] rowLoc = new int[2];
        variantRow.getLocationOnScreen(rowLoc);
        float x = rawX - rowLoc[0] - variantRow.getPaddingLeft();
        int offset = 0;
        int total = 0;
        for (TextView choice : variantViews) total += choice.getWidth();
        if (total <= 0) return;
        if (x <= 0) {
            selectVariant(0);
            return;
        }
        if (x >= total) {
            selectVariant(variantViews.size() - 1);
            return;
        }
        for (int i = 0; i < variantViews.size(); i++) {
            int w = variantViews.get(i).getWidth();
            if (x >= offset && x < offset + w) {
                selectVariant(i);
                return;
            }
            offset += w;
        }
    }

    private void selectVariant(int index) {
        if (variantSelected != index) {
            variantSelected = index;
            updateVariantHighlight();
        }
    }

    private void updateVariantHighlight() {
        for (int i = 0; i < variantViews.size(); i++) {
            boolean selected = i == variantSelected;
            TextView choice = variantViews.get(i);
            choice.setTextColor(selected ? Color.WHITE : textColor());
            choice.setBackground(roundColor(selected ? 0xFFC2410C : 0x00000000));
        }
    }

    private void commitVariant(float rawX, float rawY) {
        updateVariantSelection(rawX, rawY);
        if (variantValues.isEmpty()) {
            dismissVariant();
            return;
        }
        int index = Math.max(0, Math.min(variantSelected, variantValues.size() - 1));
        String value = variantValues.get(index);
        dismissVariant();
        listener.onDirect(value);
    }

    private void dismissVariant() {
        if (variantPopup != null) {
            variantPopup.dismiss();
            variantPopup = null;
        }
        if (variantAnchor != null) variantAnchor.setPressed(false);
        variantAnchor = null;
        variantRow = null;
        variantValues.clear();
        variantViews.clear();
        variantSelected = 0;
    }

    private static String topNumber(String key) {
        switch (key) {
            case "q": return "1";
            case "w": return "2";
            case "e": return "3";
            case "r": return "4";
            case "t": return "5";
            case "y": return "6";
            case "u": return "7";
            case "i": return "8";
            case "o": return "9";
            case "p": return "0";
            default: return null;
        }
    }

    private void updateLetters() {
        boolean shifted = isShifted();
        if (shifted != labelsShifted) {
            labelsShifted = shifted;
            for (Button button : letterButtons) {
                String key = (String) button.getTag();
                button.setText(shifted ? key.toUpperCase() : key);
            }
        }
        if (shiftButton != null && (shifted != shiftLabelActive || caps != shiftLabelCaps)) {
            shiftLabelActive = shifted;
            shiftLabelCaps = caps;
            setKeyIcon(shiftButton,
                    caps ? R.drawable.ic_shift_caps : R.drawable.ic_shift,
                    shifted ? Color.WHITE : textColor(), 24);
            shiftButton.setTextColor(shifted ? Color.WHITE : textColor());
            shiftButton.setBackground(keyBackground(shifted ? 0xFFC2410C : keyColor()));
        }
    }

    private Drawable keyBackground(int color) {
        StateListDrawable states = new StateListDrawable();
        states.addState(new int[]{android.R.attr.state_pressed}, roundColor(0xFF9AA0A6));
        states.addState(new int[]{}, roundColor(color));
        return new InsetDrawable(states, dp(2.65f), dp(5.9f), dp(2.65f), dp(5.9f));
    }

    private StateListDrawable smallKeyBackground(int color) {
        StateListDrawable states = new StateListDrawable();
        states.addState(new int[]{android.R.attr.state_pressed}, roundColor(0xFF9AA0A6));
        states.addState(new int[]{}, roundColor(color));
        return states;
    }

    private GradientDrawable panelBackground(int color) {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(color);
        drawable.setCornerRadius(dp(16));
        return drawable;
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

    private void loadClipboardHistory() {
        try {
            SharedPreferences prefs = context.getSharedPreferences(CLIP_PREFS, Context.MODE_PRIVATE);
            JSONArray array = new JSONArray(prefs.getString(CLIP_HISTORY, "[]"));
            for (int i = 0; i < array.length() && clipboardHistory.size() < CLIP_LIMIT; i++) {
                String value = array.optString(i, "");
                if (!value.isEmpty() && !clipboardHistory.contains(value)) {
                    clipboardHistory.add(value);
                }
            }
        } catch (Exception ignored) {
            // History hỏng/không đọc được thì bắt đầu rỗng.
        }
    }

    private void saveClipboardHistory() {
        try {
            JSONArray array = new JSONArray();
            for (String value : clipboardHistory) array.put(value);
            context.getSharedPreferences(CLIP_PREFS, Context.MODE_PRIVATE)
                    .edit().putString(CLIP_HISTORY, array.toString()).apply();
        } catch (Exception ignored) {
            // Không lưu được cũng không ảnh hưởng việc gõ.
        }
    }

    private void captureCurrentClip() {
        if (clipboard == null) return;
        try {
            ClipData clip = clipboard.getPrimaryClip();
            if (clip == null || clip.getItemCount() == 0) return;
            CharSequence text = clip.getItemAt(0).coerceToText(context);
            if (text != null) addClip(text.toString());
        } catch (Exception ignored) {
            // App không cho đọc clipboard.
        }
    }

    private void addClip(String text) {
        if (text == null) return;
        String value = text;
        if (value.isEmpty() || value.length() > 4000) return;
        if (!clipboardHistory.isEmpty() && value.equals(clipboardHistory.get(0))) return;
        clipboardHistory.remove(value);
        clipboardHistory.add(0, value);
        while (clipboardHistory.size() > CLIP_LIMIT) {
            clipboardHistory.remove(clipboardHistory.size() - 1);
        }
        saveClipboardHistory();
    }

    private void showQuickSettings(View anchor) {
        if (quickPopup != null && quickPopup.isShowing()) {
            dismissQuickSettings();
            return;
        }
        int panel = dark ? 0xFF2D2E31 : 0xFFFFFFFF;
        LinearLayout content = new LinearLayout(context);
        content.setOrientation(VERTICAL);
        content.setPadding(dp(12), dp(10), dp(12), dp(10));

        TextView title = new TextView(context);
        title.setText("Cài đặt nhanh");
        title.setTextSize(16);
        title.setTypeface(null, android.graphics.Typeface.BOLD);
        title.setTextColor(textColor());
        content.addView(title);

        TextView heightLabel = new TextView(context);
        heightLabel.setText("Chiều cao bàn phím");
        heightLabel.setTextSize(13);
        heightLabel.setTextColor(dark ? 0xFF9AA0A6 : 0xFF5F6368);
        heightLabel.setPadding(0, dp(8), 0, dp(4));
        content.addView(heightLabel);

        LinearLayout heights = new LinearLayout(context);
        heights.setOrientation(HORIZONTAL);
        int currentHeight = Math.max(80, Math.min(160,
                Prefs.shared(context).getInt("keyboard_height_percent", 100)));
        int[] values = {100, 115, 130, 145};
        String[] labels = {"100%", "115%", "130%", "145%"};
        for (int i = 0; i < values.length; i++) {
            final int percent = values[i];
            TextView choice = new TextView(context);
            choice.setText(labels[i]);
            choice.setTextSize(13);
            choice.setGravity(Gravity.CENTER);
            choice.setPadding(0, dp(8), 0, dp(8));
            boolean selected = percent == currentHeight;
            choice.setTextColor(selected ? Color.WHITE : textColor());
            choice.setBackground(roundColor(selected ? 0xFFC2410C : keyColor()));
            LinearLayout.LayoutParams params =
                    new LinearLayout.LayoutParams(0, dp(38), 1f);
            params.setMargins(dp(2), 0, dp(2), 0);
            choice.setLayoutParams(params);
            choice.setOnClickListener(v -> {
                Prefs.shared(context).edit()
                        .putInt("keyboard_height_percent", percent).apply();
                dismissQuickSettings();
                setHeightScale(percent);
            });
            heights.addView(choice);
        }
        content.addView(heights);

        CheckBox number = new CheckBox(context);
        number.setText("Hiện hàng số 1–0");
        number.setTextSize(14);
        number.setTextColor(textColor());
        boolean vni = "vni".equals(inputMethod);
        number.setChecked(vni || numberRow);
        number.setEnabled(!vni);
        number.setOnCheckedChangeListener((button, checked) -> {
            if (vni) return;
            Prefs.setBool(context, "show_number_row", checked);
            dismissQuickSettings();
            setNumberRow(checked);
        });
        content.addView(number);

        TextView full = menuAction("Cài đặt đầy đủ", () -> {
            dismissQuickSettings();
            listener.onSettings();
        });
        full.setGravity(Gravity.CENTER);
        full.setTextSize(14);
        full.setPadding(0, dp(10), 0, dp(10));
        content.addView(full, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        int screenW = getResources().getDisplayMetrics().widthPixels;
        int screenH = getResources().getDisplayMetrics().heightPixels;
        int width = Math.min(dp(320), screenW - dp(24));
        content.setBackground(panelBackground(panel));
        content.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
                View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED));
        int height = content.getMeasuredHeight();
        int[] location = new int[2];
        anchor.getLocationOnScreen(location);
        int x = location[0] + anchor.getWidth() / 2 - width / 2;
        x = Math.max(dp(8), Math.min(x, screenW - width - dp(8)));
        int y = location[1] + anchor.getHeight() + dp(8);
        if (y + height > screenH - dp(8)) {
            y = location[1] - height - dp(8);
        }
        if (y < dp(8)) y = dp(8);

        FrameLayout root = new FrameLayout(context);
        FrameLayout.LayoutParams contentParams =
                new FrameLayout.LayoutParams(width, ViewGroup.LayoutParams.WRAP_CONTENT);
        contentParams.leftMargin = x;
        contentParams.topMargin = y;
        root.addView(content, contentParams);
        root.setOnTouchListener((v, event) -> {
            if (event.getActionMasked() == MotionEvent.ACTION_DOWN) {
                float ex = event.getX();
                float ey = event.getY();
                if (ex < content.getLeft() || ex > content.getRight()
                        || ey < content.getTop() || ey > content.getBottom()) {
                    dismissQuickSettings();
                    return true;
                }
            }
            return false;
        });

        quickPopup = new PopupWindow(root, screenW, screenH, false);
        quickPopup.setOutsideTouchable(false);
        quickPopup.setFocusable(false);
        quickPopup.showAtLocation(this, Gravity.TOP | Gravity.START, 0, 0);
    }

    private void dismissQuickSettings() {
        if (quickPopup != null) {
            quickPopup.dismiss();
            quickPopup = null;
        }
    }

    private void showClipboardMenu(View anchor) {
        if (clipboardPopup != null && clipboardPopup.isShowing()) {
            dismissClipboardMenu();
            return;
        }
        if (SystemClock.uptimeMillis() - clipboardDismissedAt < 250) return;
        captureCurrentClip();
        int panel = dark ? 0xFF2D2E31 : 0xFFFFFFFF;
        LinearLayout content = new LinearLayout(context);
        content.setOrientation(VERTICAL);
        content.setPadding(dp(10), dp(10), dp(10), dp(8));

        LinearLayout header = new LinearLayout(context);
        header.setOrientation(HORIZONTAL);
        header.setGravity(Gravity.CENTER_VERTICAL);
        TextView title = new TextView(context);
        title.setText("Bảng nhớ tạm");
        title.setTextSize(16);
        title.setTypeface(null, android.graphics.Typeface.BOLD);
        title.setTextColor(textColor());
        title.setPadding(dp(6), 0, dp(6), dp(6));
        header.addView(title, new LinearLayout.LayoutParams(
                0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        TextView close = new TextView(context);
        close.setText("✕");
        close.setTextSize(20);
        close.setGravity(Gravity.CENTER);
        close.setTextColor(dark ? 0xFF9AA0A6 : 0xFF5F6368);
        close.setContentDescription("Đóng");
        close.setPadding(dp(10), 0, dp(6), dp(6));
        close.setOnClickListener(v -> dismissClipboardMenu());
        header.addView(close);
        content.addView(header);

        LinearLayout row1 = new LinearLayout(context);
        row1.setOrientation(HORIZONTAL);
        row1.addView(menuAction("Dán", () -> {
            if (!clipboardHistory.isEmpty()) pasteClip(clipboardHistory.get(0));
        }));
        row1.addView(menuAction("Chọn tất cả", () -> {
            listener.onTextAction("select_all");
            dismissClipboardMenu();
        }));
        content.addView(row1);

        LinearLayout row2 = new LinearLayout(context);
        row2.setOrientation(HORIZONTAL);
        row2.addView(menuAction("Sao chép", () -> {
            listener.onTextAction("copy");
            dismissClipboardMenu();
        }));
        row2.addView(menuAction("Cắt", () -> {
            listener.onTextAction("cut");
            dismissClipboardMenu();
        }));
        content.addView(row2);

        View divider = new View(context);
        divider.setBackgroundColor(dark ? 0x33FFFFFF : 0x22000000);
        LinearLayout.LayoutParams dividerParams =
                new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(1));
        dividerParams.setMargins(0, dp(6), 0, dp(4));
        content.addView(divider, dividerParams);

        ScrollView scroll = new ScrollView(context);
        scroll.setVerticalScrollBarEnabled(true);
        LinearLayout list = new LinearLayout(context);
        list.setOrientation(VERTICAL);
        if (clipboardHistory.isEmpty()) {
            TextView empty = new TextView(context);
            empty.setText("Chưa có nội dung");
            empty.setTextSize(14);
            empty.setTextColor(dark ? 0xFF9AA0A6 : 0xFF5F6368);
            empty.setPadding(dp(8), dp(10), dp(8), dp(10));
            list.addView(empty);
        } else {
            for (String value : clipboardHistory) {
                TextView item = new TextView(context);
                item.setText(clipPreview(value));
                item.setTextSize(16);
                item.setTextColor(textColor());
                item.setMaxLines(1);
                item.setEllipsize(TextUtils.TruncateAt.END);
                item.setPadding(dp(8), dp(10), dp(8), dp(10));
                item.setBackground(menuItemBackground());
                item.setOnClickListener(v -> pasteClip(value));
                list.addView(item, new LinearLayout.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT, dp(44)));
            }
        }
        scroll.addView(list);
        int visible = Math.max(1, Math.min(clipboardHistory.size(), 5));
        content.addView(scroll, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(46 * visible)));

        if (!clipboardHistory.isEmpty()) {
            TextView clear = menuAction("Xoá lịch sử", () -> {});
            clear.setLayoutParams(new LinearLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
            clear.setTextColor(dark ? 0xFF9AA0A6 : 0xFF5F6368);
            clear.setOnClickListener(v -> {
                clipboardHistory.clear();
                saveClipboardHistory();
                dismissClipboardMenu();
            });
            content.addView(clear);
        }

        int screenW = getResources().getDisplayMetrics().widthPixels;
        int screenH = getResources().getDisplayMetrics().heightPixels;
        int width = Math.min(dp(320), screenW - dp(24));
        content.setBackground(panelBackground(panel));
        content.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
                View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED));
        int height = content.getMeasuredHeight();
        int[] location = new int[2];
        anchor.getLocationOnScreen(location);
        int x = location[0] + anchor.getWidth() / 2 - width / 2;
        x = Math.max(dp(8), Math.min(x, screenW - width - dp(8)));
        int y = location[1] + anchor.getHeight() + dp(8);
        if (y + height > screenH - dp(8)) {
            y = location[1] - height - dp(32);
        }
        if (y < dp(8)) y = dp(8);

        FrameLayout root = new FrameLayout(context);
        FrameLayout.LayoutParams contentParams =
                new FrameLayout.LayoutParams(width, ViewGroup.LayoutParams.WRAP_CONTENT);
        contentParams.leftMargin = x;
        contentParams.topMargin = y;
        root.addView(content, contentParams);
        root.setOnTouchListener((v, event) -> {
            if (event.getActionMasked() == MotionEvent.ACTION_DOWN) {
                float ex = event.getX();
                float ey = event.getY();
                if (ex < content.getLeft() || ex > content.getRight()
                        || ey < content.getTop() || ey > content.getBottom()) {
                    dismissClipboardMenu();
                    return true;
                }
            }
            return false;
        });

        clipboardPopup = new PopupWindow(root, screenW, screenH, false);
        clipboardPopup.setOutsideTouchable(false);
        clipboardPopup.setFocusable(false);
        clipboardPopup.showAtLocation(this, Gravity.TOP | Gravity.START, 0, 0);
    }

    private TextView menuAction(String label, Runnable action) {
        TextView view = new TextView(context);
        view.setText(label);
        view.setTextSize(13);
        view.setTextColor(0xFFC2410C);
        view.setGravity(Gravity.CENTER);
        view.setPadding(dp(6), dp(8), dp(6), dp(8));
        view.setOnClickListener(v -> action.run());
        view.setLayoutParams(new LinearLayout.LayoutParams(
                0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        return view;
    }

    private StateListDrawable menuItemBackground() {
        StateListDrawable states = new StateListDrawable();
        states.addState(new int[]{android.R.attr.state_pressed}, roundColor(0x339AA0A6));
        states.addState(new int[]{}, roundColor(0x00000000));
        return states;
    }

    private String clipPreview(String value) {
        String preview = value.replace('\n', ' ').replace('\r', ' ').trim();
        if (preview.length() > 80) preview = preview.substring(0, 80) + "…";
        return preview.isEmpty() ? value : preview;
    }

    private void pasteClip(String text) {
        dismissClipboardMenu();
        listener.onDirect(text);
    }

    private void dismissClipboardMenu() {
        if (clipboardPopup != null) {
            clipboardPopup.dismiss();
            clipboardPopup = null;
        }
        clipboardDismissedAt = SystemClock.uptimeMillis();
    }

    private final class LetterButton extends Button {
        String hint;
        private final Paint hintPaint = new Paint(Paint.ANTI_ALIAS_FLAG);

        LetterButton(Context context) {
            super(context);
        }

        @Override
        protected void onDraw(Canvas canvas) {
            super.onDraw(canvas);
            if (hint == null) return;
            hintPaint.setColor(getCurrentTextColor());
            hintPaint.setAlpha(170);
            hintPaint.setTextSize(sp(9));
            hintPaint.setTextAlign(Paint.Align.RIGHT);
            canvas.drawText(hint, getWidth() - dp(8), dp(16), hintPaint);
        }
    }

    private float sp(float value) {
        return TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, value,
                getResources().getDisplayMetrics());
    }

    private int dp(float value) {
        return Math.round(TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, value,
                getResources().getDisplayMetrics()));
    }
}
