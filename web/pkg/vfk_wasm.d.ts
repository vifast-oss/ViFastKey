/* tslint:disable */
/* eslint-disable */

export class WasmSession {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Cấu hình hiện tại dạng JSON.
     */
    config_json(): string;
    /**
     * Người dùng click chuột / con trỏ đổi chỗ.
     */
    cursor_moved(): string;
    /**
     * Chữ đang hiển thị của từ hiện tại.
     */
    display(): string;
    /**
     * Đổi app đang focus; có thể trả COMMIT khi đang có preedit.
     */
    focus_changed(app_id?: string | null): string;
    is_enabled(): boolean;
    /**
     * Nạp từ điển từ 2 danh sách (mỗi từ cách nhau bởi khoảng trắng/xuống dòng).
     */
    load_dictionary(english: string, vietnamese: string): void;
    constructor(config_json: string | null | undefined, mode: string);
    /**
     * Xử lý một phím. `key` là 1 ký tự hoặc tên phím
     * ("Backspace", "Enter", "Escape", "Left", ...).
     */
    process_key(key: string, ctrl: boolean, alt: boolean, meta: boolean): string;
    /**
     * Chuỗi phím thô của từ hiện tại (để hiển thị debug).
     */
    raw(): string;
    /**
     * Xoá từ đang gõ mà không phát Action.
     */
    reset(): void;
    /**
     * Cập nhật một phần cấu hình (merge với cấu hình hiện tại).
     */
    set_config(json: string): boolean;
    /**
     * 0 text, 1 password, 2 number, 3 email, 4 url, 5 terminal.
     */
    set_field_kind(kind: string): string;
    /**
     * Cập nhật phần văn bản ngay trước con trỏ (surrounding text).
     */
    set_surrounding(text?: string | null): void;
    /**
     * Bật/tắt tiếng Việt; có thể trả COMMIT.
     */
    toggle_vietnamese(): string;
}

export function charset_decode(name: string, text: string): string;

export function charset_encode(name: string, text: string): string;

/**
 * Bytes thô cho bảng một byte; mảng rỗng nếu bảng không hỗ trợ.
 */
export function charset_encode_bytes(name: string, text: string): Uint8Array;

/**
 * Danh sách bảng mã dạng JSON: [{key,name}].
 */
export function charset_list(): string;

/**
 * Chuẩn hoá một đoạn văn bản: chuyển Telex/VNI thô thành chữ Việt và
 * chuẩn hoá các từ đã có dấu. Giữ nguyên dấu câu/khoảng trắng.
 * Từ tiếng Anh đã biết trong lexicon được giữ nguyên thay vì bẻ dấu thành tiếng Việt.
 */
export function normalize_text(text: string, method: string, tone_style: string): string;

export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmsession_free: (a: number, b: number) => void;
    readonly charset_decode: (a: number, b: number, c: number, d: number) => [number, number];
    readonly charset_encode: (a: number, b: number, c: number, d: number) => [number, number];
    readonly charset_encode_bytes: (a: number, b: number, c: number, d: number) => [number, number];
    readonly charset_list: () => [number, number];
    readonly normalize_text: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly version: () => [number, number];
    readonly wasmsession_config_json: (a: number) => [number, number];
    readonly wasmsession_cursor_moved: (a: number) => [number, number];
    readonly wasmsession_display: (a: number) => [number, number];
    readonly wasmsession_focus_changed: (a: number, b: number, c: number) => [number, number];
    readonly wasmsession_is_enabled: (a: number) => number;
    readonly wasmsession_load_dictionary: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly wasmsession_new: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmsession_process_key: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly wasmsession_raw: (a: number) => [number, number];
    readonly wasmsession_reset: (a: number) => void;
    readonly wasmsession_set_config: (a: number, b: number, c: number) => number;
    readonly wasmsession_set_field_kind: (a: number, b: number, c: number) => [number, number];
    readonly wasmsession_set_surrounding: (a: number, b: number, c: number) => void;
    readonly wasmsession_toggle_vietnamese: (a: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
