//! Bảng luật kiểu gõ dạng DỮ LIỆU (Strategy pattern). Port của `vfk_core/rules.py`.
//!
//! Thêm kiểu gõ mới = thêm 1 `InputMethodSpec`, không sửa thuật toán.
//! THỨ TỰ phần tử trong các mảng là có nghĩa (giống thứ tự dict trong Python):
//! `mark_key`, `tone_key` và thứ tự luật trong `marks` ảnh hưởng trực tiếp tới kết quả.

use vi_syllable::Mark;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkRule {
    pub mark: Mark,
    /// Các chữ cơ sở nhận dấu.
    pub targets: &'static str,
    /// `true`: áp cho cặp "uo" -> "ươ".
    pub pair: bool,
}

const fn rule(mark: Mark, targets: &'static str) -> MarkRule {
    MarkRule { mark, targets, pair: false }
}

const fn pair(mark: Mark, targets: &'static str) -> MarkRule {
    MarkRule { mark, targets, pair: true }
}

#[derive(Debug)]
pub struct InputMethodSpec {
    pub name: &'static str,
    /// phím -> thanh (1..=5).
    pub tones: &'static [(char, u8)],
    /// phím xoá thanh.
    pub remove_tone: char,
    /// phím -> luật dấu (theo thứ tự ưu tiên khai báo).
    pub marks: &'static [(char, &'static [MarkRule])],
    /// Chỉ khi không có đích: Telex "w" -> "ư".
    pub standalone: &'static [(char, (char, Mark))],
    /// Luôn chèn: "[" -> "ơ", "]" -> "ư".
    pub insert_keys: &'static [(char, (char, Mark))],
    /// Chữ số là phím chức năng (VNI).
    pub digit_keys: bool,
}

impl InputMethodSpec {
    pub fn tone_of(&self, k: char) -> Option<u8> {
        self.tones.iter().find(|(c, _)| *c == k).map(|(_, t)| *t)
    }

    pub fn marks_for(&self, k: char) -> Option<&'static [MarkRule]> {
        self.marks.iter().find(|(c, _)| *c == k).map(|(_, r)| *r)
    }

    pub fn standalone_for(&self, k: char) -> Option<(char, Mark)> {
        self.standalone.iter().find(|(c, _)| *c == k).map(|(_, v)| *v)
    }

    pub fn insert_for(&self, k: char) -> Option<(char, Mark)> {
        self.insert_keys.iter().find(|(c, _)| *c == k).map(|(_, v)| *v)
    }

    pub fn is_function_key(&self, k: char) -> bool {
        let k = vi_syllable::lower(k);
        self.tone_of(k).is_some()
            || k == self.remove_tone
            || self.marks_for(k).is_some()
            || self.standalone_for(k).is_some()
            || self.insert_for(k).is_some()
    }

    /// Phím tạo dấu `mark` trên chữ `base` (bỏ qua luật cặp). Dùng để đảo ngược chữ -> phím.
    pub fn mark_key(&self, base: char, mark: Mark) -> Option<char> {
        for (key, rules) in self.marks {
            for r in rules.iter() {
                if r.mark == mark && r.targets.contains(base) && !r.pair {
                    return Some(*key);
                }
            }
        }
        None
    }

    pub fn tone_key(&self, tone: u8) -> Option<char> {
        self.tones.iter().find(|(_, t)| *t == tone).map(|(k, _)| *k)
    }

    pub fn by_name(name: &str) -> Option<&'static InputMethodSpec> {
        match name {
            "telex" => Some(&TELEX),
            "simple_telex" => Some(&SIMPLE_TELEX),
            "vni" => Some(&VNI),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------- Telex

const TELEX_TONES: &[(char, u8)] = &[('s', 1), ('f', 2), ('r', 3), ('x', 4), ('j', 5)];
const TELEX_A: &[MarkRule] = &[rule(Mark::Circumflex, "a")];
const TELEX_E: &[MarkRule] = &[rule(Mark::Circumflex, "e")];
const TELEX_O: &[MarkRule] = &[rule(Mark::Circumflex, "o")];
const TELEX_W: &[MarkRule] = &[pair(Mark::Horn, "uo"), rule(Mark::Horn, "uo"), rule(Mark::Breve, "a")];
const TELEX_D: &[MarkRule] = &[rule(Mark::Stroke, "d")];
const TELEX_MARKS: &[(char, &[MarkRule])] =
    &[('a', TELEX_A), ('e', TELEX_E), ('o', TELEX_O), ('w', TELEX_W), ('d', TELEX_D)];

pub static TELEX: InputMethodSpec = InputMethodSpec {
    name: "telex",
    tones: TELEX_TONES,
    remove_tone: 'z',
    marks: TELEX_MARKS,
    standalone: &[('w', ('u', Mark::Horn))],
    insert_keys: &[('[', ('o', Mark::Horn)), (']', ('u', Mark::Horn))],
    digit_keys: false,
};

/// Simple Telex: không có "w" đứng riêng, không có [ ].
pub static SIMPLE_TELEX: InputMethodSpec = InputMethodSpec {
    name: "simple_telex",
    tones: TELEX_TONES,
    remove_tone: 'z',
    marks: TELEX_MARKS,
    standalone: &[],
    insert_keys: &[],
    digit_keys: false,
};

// ---------------------------------------------------------------- VNI

const VNI_6: &[MarkRule] = &[rule(Mark::Circumflex, "aeo")];
const VNI_7: &[MarkRule] = &[pair(Mark::Horn, "uo"), rule(Mark::Horn, "uo")];
const VNI_8: &[MarkRule] = &[rule(Mark::Breve, "a")];
const VNI_9: &[MarkRule] = &[rule(Mark::Stroke, "d")];

pub static VNI: InputMethodSpec = InputMethodSpec {
    name: "vni",
    tones: &[('1', 1), ('2', 2), ('3', 3), ('4', 4), ('5', 5)],
    remove_tone: '0',
    marks: &[('6', VNI_6), ('7', VNI_7), ('8', VNI_8), ('9', VNI_9)],
    standalone: &[],
    insert_keys: &[],
    digit_keys: true,
};

/// Gõ nhanh phụ âm đầu (Quick Telex): cc=ch, gg=gi, kk=kh, nn=ng, qq=qu, pp=ph, tt=th.
pub fn quick_telex(c: char) -> Option<&'static str> {
    match c {
        'c' => Some("ch"),
        'g' => Some("gi"),
        'k' => Some("kh"),
        'n' => Some("ng"),
        'q' => Some("qu"),
        'p' => Some("ph"),
        't' => Some("th"),
        _ => None,
    }
}
