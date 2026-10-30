# Third-party notices

ViFastKey code is MIT (see `LICENSE`). This file lists data/attribution and research sources.
Logo/wordmark are not MIT; see `resources/brand/README.md`.

## bamboo-core (MIT) — charset tables

`python/vfk_core/data/charsets.json` and `rust/crates/vfk-core/data/charsets.json` are
generated from the charset tables of [bamboo-vi/bamboo-core](https://github.com/bamboo-vi/bamboo-core)
at commit `06d4ff7`, used under the MIT license. Only mapping data was extracted; no engine
code, parser, tests, or documentation text was copied. We independently implemented encode/decode
and round-trip tests.

```
The MIT License (MIT)

Copyright (C) 2018 Luong Thanh Lam <ltlam93@gmail.com>
Copyright (C) 2024 nguien <nguyen10t2lhp@gmail.com>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## English suggestion lexicon (public domain)

`python/vfk_core/data/english_suggest.txt` and `rust/crates/vfk-suggest/data/english_suggest.txt`
are a filtered subset of [dwyl/english-words](https://github.com/dwyl/english-words)
(`words_alpha.txt`), released under the Unlicense/public domain. The Vietnamese syllable list
is generated from ViFastKey's own grammar, not copied from a dictionary.

## Fonts

- **Be Vietnam Pro** — SIL Open Font License 1.1; see
  `resources/fonts/BeVietnamPro-OFL.txt`.
- **egui/eframe** — MIT OR Apache-2.0; dùng cho `vifastkey-settings`.

## Research sources (behavior only)

The following projects were analyzed for behavior, architecture, UX and test ideas:

SKey, SKey Engine, ibus-bamboo, fcitx5-unikey, fcitx5-lotus, VnKey, Vigo, Funput, Bamboo Viet,
OpenKey, XKey, VKey, GõViệt, UVieMac, AVIM, v7, Telebit, UniKey/x-unikey, ibus-unikey.

Licenses include GPL-2.0/3.0, AGPL-3.0, MIT and Apache-2.0. **No GPL/AGPL source code,
tables, dictionaries, test corpora or UI strings were copied.** Test vectors in
`docs/IME_FEATURE_RESEARCH.md` and `python/tests/test_research_regressions.py` are
input/output facts independently restated and verified against ViFastKey's own implementation.

If you believe an attribution is missing, open an issue with the file and source.
