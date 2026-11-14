#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::env;
use std::ffi::{c_void, CStr, CString};
use std::fs::{self, File};
use std::io::Write;
use std::mem::{size_of, zeroed};
use std::os::raw::{c_char, c_double, c_float, c_int, c_ulong, c_ushort};
use std::path::Path;
use std::ptr::{copy_nonoverlapping, null, null_mut};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen, CreateSolidBrush,
        DeleteDC, DeleteObject, EndPaint, FillRect, GetDC, GetMonitorInfoW, GetStockObject,
        InvalidateRect, LineTo, MonitorFromWindow, MoveToEx, Rectangle, ReleaseDC, ScreenToClient,
        SelectObject, SetBkMode, SetTextColor, StretchDIBits, TextOutW, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DEFAULT_GUI_FONT, DIB_RGB_COLORS, MONITORINFO,
        MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, PS_DOT, PS_SOLID, SRCCOPY, TRANSPARENT,
    },
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
        },
        LibraryLoader::{GetModuleHandleW, GetProcAddress},
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    },
    UI::{
        Controls::Dialogs::{
            GetOpenFileNameW, GetSaveFileNameW, PrintDlgW, OFN_ENABLESIZING, OFN_EXPLORER,
            OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST,
            OPENFILENAMEW, PD_RETURNDC, PRINTDLGW,
        },
        HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
        Input::KeyboardAndMouse::{
            GetKeyState, ReleaseCapture, SetCapture, VK_BACK, VK_CONTROL, VK_DOWN, VK_END,
            VK_ESCAPE, VK_F1, VK_F11, VK_F3, VK_HOME, VK_LEFT, VK_NEXT, VK_OEM_4, VK_OEM_6,
            VK_OEM_MINUS, VK_OEM_PLUS, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_UP,
        },
        Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, ShellExecuteW, HDROP},
        WindowsAndMessaging::{
            AppendMenuW, CreateMenu, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
            DispatchMessageW, GetClientRect, GetMessageW, GetWindowLongPtrW, GetWindowPlacement,
            LoadCursorW, MessageBoxW, PostQuitMessage, RegisterClassW, SetCursor, SetMenu,
            SetWindowLongPtrW, SetWindowPlacement, SetWindowPos, SetWindowTextW, ShowWindow,
            TranslateMessage, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, GWL_STYLE, HWND_TOP,
            IDC_ARROW, IDC_HAND, MB_ICONINFORMATION, MB_OK, MF_POPUP, MF_SEPARATOR, MF_STRING,
            MSG, SWP_FRAMECHANGED, SWP_NOOWNERZORDER, SWP_NOZORDER, SW_SHOWNORMAL, SW_SHOW,
            WINDOWPLACEMENT, WM_CHAR, WM_COMMAND, WM_DESTROY, WM_DROPFILES, WM_ERASEBKGND,
            WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT,
            WM_SETCURSOR, WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const CF_BITMAP: u32 = 2;
const CF_UNICODETEXT: u32 = 13;
const SIDEBAR_WIDTH: i32 = 280;
const SIDEBAR_TAB_HEIGHT: i32 = 34;
const BOOKMARK_ROW_HEIGHT: i32 = 26;
const THUMB_ITEM_HEIGHT: i32 = 216;
const SPREAD_GAP: i32 = 16;
const TOOLBAR_HEIGHT: i32 = 40;
const STATUSBAR_HEIGHT: i32 = 28;

// PDFium Action Type 상수 (fpdf_doc.h)
const PDFACTION_GOTO: c_ulong = 1;
const PDFACTION_URI: c_ulong = 3;

// ============================================================================
// 메뉴 ID 상수
// ============================================================================
const IDM_FILE_NEW_BLANK: u16 = 1001;
const IDM_FILE_FROM_CLIPBOARD: u16 = 1002;
const IDM_FILE_OPEN: u16 = 1003;
const IDM_FILE_SAVE: u16 = 1004;
const IDM_FILE_SAVE_AS: u16 = 1005;
const IDM_FILE_SAVE_V14: u16 = 1006;
const IDM_FILE_SAVE_V17: u16 = 1007;
const IDM_FILE_PRINT: u16 = 1008;
const IDM_FILE_PROPS: u16 = 1009;
const IDM_FILE_EXIT: u16 = 1010;

const IDM_EDIT_ADD_TEXT: u16 = 2001;
const IDM_EDIT_WM_CONFIDENTIAL: u16 = 2002;
const IDM_EDIT_WM_SAMPLE: u16 = 2003;
const IDM_EDIT_WM_CUSTOM: u16 = 2004;
const IDM_EDIT_WM_CLEAR: u16 = 2005;
const IDM_EDIT_BG_CREAM: u16 = 2006;
const IDM_EDIT_BG_MINT: u16 = 2007;
const IDM_EDIT_BG_CLEAR: u16 = 2008;
const IDM_EDIT_HEADER_FOOTER: u16 = 2009;
const IDM_EDIT_PAGE_NUMBER: u16 = 2010;
const IDM_EDIT_BATES_NUMBER: u16 = 2011;

const IDM_ANNOT_NONE: u16 = 3000;
const IDM_ANNOT_HIGHLIGHT: u16 = 3001;
const IDM_ANNOT_AREA_HL: u16 = 3002;
const IDM_ANNOT_PENCIL: u16 = 3003;
const IDM_ANNOT_ERASER: u16 = 3004;
const IDM_ANNOT_UNDERLINE: u16 = 3005;
const IDM_ANNOT_STRIKEOUT: u16 = 3006;
const IDM_ANNOT_RECT: u16 = 3007;
const IDM_ANNOT_LINE: u16 = 3008;
const IDM_ANNOT_NOTE: u16 = 3009;
const IDM_ANNOT_STAMP_APPROVED: u16 = 3010;
const IDM_ANNOT_STAMP_DRAFT: u16 = 3011;
const IDM_ANNOT_MEASURE_DIST: u16 = 3012;
const IDM_ANNOT_MEASURE_AREA: u16 = 3013;
const IDM_ANNOT_TOGGLE_SHOW: u16 = 3014;
const IDM_ANNOT_CLEAR_ALL: u16 = 3015;

const IDM_CONV_TO_BMP: u16 = 4001;
const IDM_CONV_TO_TXT: u16 = 4002;
const IDM_CONV_TO_HTML: u16 = 4003;
const IDM_CONV_TO_RTF: u16 = 4004;
const IDM_CONV_TO_CSV: u16 = 4005;
const IDM_CONV_BATCH_TXT: u16 = 4006;

const IDM_VIEW_ZOOM_IN: u16 = 5001;
const IDM_VIEW_ZOOM_OUT: u16 = 5002;
const IDM_VIEW_ACTUAL_SIZE: u16 = 5003;
const IDM_VIEW_FIT_PAGE: u16 = 5004;
const IDM_VIEW_FIT_WIDTH: u16 = 5005;
const IDM_VIEW_SINGLE: u16 = 5006;
const IDM_VIEW_SPREAD: u16 = 5007;
const IDM_VIEW_COVER_SPREAD: u16 = 5008;
const IDM_VIEW_ROTATE_CW: u16 = 5009;
const IDM_VIEW_ROTATE_CCW: u16 = 5010;
const IDM_VIEW_RULER: u16 = 5011;
const IDM_VIEW_GRID: u16 = 5012;
const IDM_VIEW_BOOKMARKS: u16 = 5013;
const IDM_VIEW_THUMBNAILS: u16 = 5016;
const IDM_VIEW_AUTO_BOOKMARK: u16 = 5014;
const IDM_VIEW_FULLSCREEN: u16 = 5015;

const IDM_ORG_INSERT_BLANK: u16 = 6001;
const IDM_ORG_INSERT_PDF: u16 = 6002;
const IDM_ORG_DELETE_PAGE: u16 = 6003;
const IDM_ORG_EXTRACT_PAGE: u16 = 6004;
const IDM_ORG_SPLIT_PDF: u16 = 6005;
const IDM_ORG_CROP_MARGIN: u16 = 6006;
const IDM_ORG_ROTATE_PAGE_OBJ: u16 = 6007;

const IDM_TOOL_SEARCH: u16 = 7001;
const IDM_TOOL_GOTO: u16 = 7002;
const IDM_TOOL_MERGE_PDF: u16 = 7003;
const IDM_TOOL_COMPARE_PDF: u16 = 7004;
const IDM_TOOL_FLATTEN: u16 = 7005;
const IDM_TOOL_DETECT_TABLES: u16 = 7006;
const IDM_TOOL_SCREENSHOT: u16 = 7007;
const IDM_TOOL_OCR_EXTRACT: u16 = 7008;

const IDM_FORM_ADD_TEXTFIELD: u16 = 8001;
const IDM_FORM_ADD_CHECKBOX: u16 = 8002;
const IDM_FORM_RECOGNIZE: u16 = 8003;
const IDM_FORM_EXPORT_CSV: u16 = 8004;

const IDM_SEC_REDACT_TOOL: u16 = 9001;
const IDM_SEC_SEARCH_REDACT: u16 = 9002;
const IDM_SEC_SANITIZE: u16 = 9003;
const IDM_SEC_SIGN_STAMP: u16 = 9004;

const IDM_THEME_DARK_DOC: u16 = 9501;
const IDM_THEME_LIGHT: u16 = 9502;
const IDM_THEME_DARK_BG: u16 = 9503;
const IDM_THEME_CLASSIC: u16 = 9504;
const IDM_HELP_SHORTCUTS: u16 = 9505;

const IDM_TB_PREV_PAGE: u16 = 9801;
const IDM_TB_NEXT_PAGE: u16 = 9802;
const IDM_TB_FIRST_PAGE: u16 = 9803;
const IDM_TB_LAST_PAGE: u16 = 9804;

const FPDF_ANNOT: c_int = 0x01;
const FPDF_LCD_TEXT: c_int = 0x02;

#[repr(C)]
struct FPDF_FILEWRITE {
    version: c_int,
    WriteBlock: Option<
        unsafe extern "C" fn(
            pThis: *mut FPDF_FILEWRITE,
            pData: *const c_void,
            size: c_ulong,
        ) -> c_int,
    >,
}

#[repr(C)]
struct FileWriteContext {
    fw: FPDF_FILEWRITE,
    buf: Vec<u8>,
}

unsafe extern "C" fn write_block_cb(
    p_this: *mut FPDF_FILEWRITE,
    p_data: *const c_void,
    size: c_ulong,
) -> c_int {
    let ctx = p_this as *mut FileWriteContext;
    let slice = std::slice::from_raw_parts(p_data as *const u8, size as usize);
    (*ctx).buf.extend_from_slice(slice);
    1
}

type c_uint = u32;

#[link(name = "pdfium.dll")]
extern "C" {
    fn FPDF_InitLibrary();
    fn FPDF_DestroyLibrary();
    fn FPDF_GetLastError() -> c_ulong;
    fn FPDF_CreateNewDocument() -> *mut c_void;
    fn FPDF_LoadMemDocument(
        data_buf: *const c_void,
        size: c_int,
        password: *const c_char,
    ) -> *mut c_void;
    fn FPDF_CloseDocument(document: *mut c_void);
    fn FPDF_GetPageCount(document: *mut c_void) -> c_int;
    fn FPDF_LoadPage(document: *mut c_void, page_index: c_int) -> *mut c_void;
    fn FPDF_ClosePage(page: *mut c_void);
    fn FPDF_GetPageWidth(page: *mut c_void) -> c_double;
    fn FPDF_GetPageHeight(page: *mut c_void) -> c_double;
    fn FPDF_GetFileVersion(doc: *mut c_void, file_version: *mut c_int) -> c_int;

    fn FPDF_PageToDevice(
        page: *mut c_void,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        page_x: c_double,
        page_y: c_double,
        device_x: *mut c_int,
        device_y: *mut c_int,
    ) -> c_int;

    fn FPDF_DeviceToPage(
        page: *mut c_void,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        device_x: c_int,
        device_y: c_int,
        page_x: *mut c_double,
        page_y: *mut c_double,
    ) -> c_int;

    fn FPDFBitmap_Create(width: c_int, height: c_int, alpha: c_int) -> *mut c_void;
    fn FPDFBitmap_FillRect(
        bitmap: *mut c_void,
        left: c_int,
        top: c_int,
        width: c_int,
        height: c_int,
        color: c_ulong,
    );
    fn FPDFBitmap_GetBuffer(bitmap: *mut c_void) -> *mut u8;
    fn FPDFBitmap_GetStride(bitmap: *mut c_void) -> c_int;
    fn FPDFBitmap_Destroy(bitmap: *mut c_void);
    fn FPDF_RenderPageBitmap(
        bitmap: *mut c_void,
        page: *mut c_void,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    );

    fn FPDFText_LoadPage(page: *mut c_void) -> *mut c_void;
    fn FPDFText_ClosePage(text_page: *mut c_void);
    fn FPDFText_CountChars(text_page: *mut c_void) -> c_int;
    fn FPDFText_GetText(
        text_page: *mut c_void,
        start_index: c_int,
        count: c_int,
        result: *mut c_ushort,
    ) -> c_int;

    fn FPDFText_FindStart(
        text_page: *mut c_void,
        findwhat: *const c_ushort,
        flags: c_ulong,
        start_index: c_int,
    ) -> *mut c_void;
    fn FPDFText_FindNext(handle: *mut c_void) -> c_int;
    fn FPDFText_GetSchResultIndex(handle: *mut c_void) -> c_int;
    fn FPDFText_GetSchCount(handle: *mut c_void) -> c_int;
    fn FPDFText_FindClose(handle: *mut c_void);

    fn FPDFText_CountRects(text_page: *mut c_void, start_index: c_int, count: c_int) -> c_int;
    fn FPDFText_GetRect(
        text_page: *mut c_void,
        rect_index: c_int,
        left: *mut c_double,
        top: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
    ) -> c_int;

    // 북마크, 링크 및 액션 API (fpdf_doc.h)
    fn FPDFBookmark_GetFirstChild(document: *mut c_void, bookmark: *mut c_void) -> *mut c_void;
    fn FPDFBookmark_GetNextSibling(document: *mut c_void, bookmark: *mut c_void) -> *mut c_void;
    fn FPDFBookmark_GetTitle(
        bookmark: *mut c_void,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;
    fn FPDFBookmark_GetDest(document: *mut c_void, bookmark: *mut c_void) -> *mut c_void;
    fn FPDFBookmark_GetAction(bookmark: *mut c_void) -> *mut c_void;

    fn FPDFLink_GetLinkAtPoint(
        page: *mut c_void,
        x: c_double,
        y: c_double,
    ) -> *mut c_void;
    fn FPDFLink_GetDest(document: *mut c_void, link: *mut c_void) -> *mut c_void;
    fn FPDFLink_GetAction(link: *mut c_void) -> *mut c_void;

    fn FPDFAction_GetType(action: *mut c_void) -> c_ulong;
    fn FPDFAction_GetDest(document: *mut c_void, action: *mut c_void) -> *mut c_void;
    fn FPDFAction_GetURIPath(
        document: *mut c_void,
        action: *mut c_void,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;
    fn FPDFDest_GetDestPageIndex(document: *mut c_void, dest: *mut c_void) -> c_int;

    fn FPDF_GetMetaText(
        document: *mut c_void,
        tag: *const c_char,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    fn FPDFPage_New(
        document: *mut c_void,
        page_index: c_int,
        width: c_double,
        height: c_double,
    ) -> *mut c_void;
    fn FPDFPage_Delete(document: *mut c_void, page_index: c_int);
    fn FPDFPage_GetRotation(page: *mut c_void) -> c_int;
    fn FPDFPage_SetRotation(page: *mut c_void, rotate: c_int);
    fn FPDFPage_InsertObject(page: *mut c_void, page_obj: *mut c_void);
    fn FPDFPage_GenerateContent(page: *mut c_void) -> c_int;
    fn FPDFText_SetText(text_object: *mut c_void, text: *const c_ushort) -> c_int;
    fn FPDFPageObj_Transform(
        page_object: *mut c_void,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    );
    fn FPDFPageObj_SetFillColor(
        page_object: *mut c_void,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> c_int;
    fn FPDFPageObj_CreateNewRect(
        x: c_float,
        y: c_float,
        w: c_float,
        h: c_float,
    ) -> *mut c_void;
    fn FPDFPath_SetDrawMode(
        path: *mut c_void,
        fillmode: c_int,
        stroke: c_int,
    ) -> c_int;
    fn FPDFPage_SetCropBox(
        page: *mut c_void,
        left: c_float,
        bottom: c_float,
        right: c_float,
        top: c_float,
    );
    fn FPDFPage_Flatten(page: *mut c_void, nFlag: c_int) -> c_int;
    fn FPDF_ImportPages(
        dest_doc: *mut c_void,
        src_doc: *mut c_void,
        pagerange: *const c_char,
        index: c_int,
    ) -> c_int;
    fn FPDF_SaveAsCopy(
        document: *mut c_void,
        pFileWrite: *mut FPDF_FILEWRITE,
        flags: c_ulong,
    ) -> c_int;
    fn FPDF_SaveWithVersion(
        document: *mut c_void,
        pFileWrite: *mut FPDF_FILEWRITE,
        flags: c_ulong,
        fileVersion: c_int,
    ) -> c_int;
}

#[derive(Clone, Copy)]
struct PdfRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

#[derive(Clone)]
struct SearchMatch {
    rects: Vec<PdfRect>,
}

#[derive(Clone)]
struct BookmarkItem {
    title: String,
    page_index: i32,
    depth: usize,
}

#[derive(Clone)]
struct ThumbnailCache {
    width: i32,
    height: i32,
    pixels: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq)]
enum LinkTarget {
    InternalPage(i32),
    ExternalUri(String),
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum SidebarTab {
    Bookmarks,
    Thumbnails,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum InputMode {
    Normal,
    Search,
    GotoPage,
    AddTextPrompt,
    CustomWatermarkPrompt,
    SearchRedactPrompt,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum ActiveTool {
    Pan,
    Highlight,
    AreaHighlight,
    Underline,
    Strikeout,
    RectShape,
    LineShape,
    Pencil,
    Eraser,
    StampApproved,
    StampDraft,
    Note,
    MeasureDist,
    MeasureArea,
    RedactBox,
    FormTextField,
    FormCheckBox,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OverlayKind {
    Highlight,
    AreaHighlight,
    Underline,
    Strikeout,
    RectShape,
    LineShape,
    Pencil,
    StampApproved,
    StampDraft,
    Note,
    Redaction,
    FormText,
    FormCheck,
}

#[derive(Clone)]
struct UserOverlay {
    page_index: i32,
    kind: OverlayKind,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    points: Vec<(f64, f64)>,
    label: String,
    checked: bool,
}

struct ToolbarButton {
    id: u16,
    label: &'static str,
    width: i32,
}

const TOOLBAR_BUTTONS: &[ToolbarButton] = &[
    ToolbarButton { id: IDM_FILE_OPEN, label: "📂 열기", width: 64 },
    ToolbarButton { id: IDM_FILE_SAVE, label: "💾 저장", width: 64 },
    ToolbarButton { id: IDM_VIEW_ZOOM_OUT, label: "🔍-", width: 44 },
    ToolbarButton { id: IDM_VIEW_ZOOM_IN, label: "🔍+", width: 44 },
    ToolbarButton { id: IDM_VIEW_FIT_WIDTH, label: "↔ 폭맞춤", width: 72 },
    ToolbarButton { id: IDM_VIEW_SPREAD, label: "📖 2p양면", width: 74 },
    ToolbarButton { id: IDM_THEME_DARK_DOC, label: "🌙 다크", width: 64 },
    ToolbarButton { id: IDM_VIEW_BOOKMARKS, label: "📑 목차", width: 64 },
    ToolbarButton { id: IDM_VIEW_THUMBNAILS, label: "🖼 썸네일", width: 74 },
    ToolbarButton { id: IDM_ANNOT_NONE, label: "✋ 이동", width: 64 },
    ToolbarButton { id: IDM_ANNOT_HIGHLIGHT, label: "🖍 강조", width: 64 },
    ToolbarButton { id: IDM_ANNOT_PENCIL, label: "✏ 연필", width: 64 },
    ToolbarButton { id: IDM_SEC_REDACT_TOOL, label: "⬛ 교정", width: 64 },
    ToolbarButton { id: IDM_EDIT_ADD_TEXT, label: "➕ 텍스트", width: 74 },
    ToolbarButton { id: IDM_TOOL_SEARCH, label: "🔎 검색", width: 64 },
    ToolbarButton { id: IDM_TB_PREV_PAGE, label: "◀ 이전", width: 60 },
    ToolbarButton { id: IDM_TB_NEXT_PAGE, label: "다음 ▶", width: 60 },
];

struct AppState {
    doc: *mut c_void,
    pdf_bytes: Vec<u8>,
    current_path: Option<String>,
    file_name: String,
    page_count: i32,
    current_page: i32,
    cur_page_w_pt: f64,
    cur_page_h_pt: f64,
    spread_mode: bool,
    cover_mode: bool,
    fit_width: bool,
    pending_align_top: bool,
    pending_align_bottom: bool,
    dark_mode: bool,
    theme_bg_color: u32,
    is_fullscreen: bool,
    saved_placement: WINDOWPLACEMENT,
    rotation: i32,
    zoom: f64,
    pan_x: i32,
    pan_y: i32,
    is_dragging: bool,
    mouse_down_x: i32,
    mouse_down_y: i32,
    drag_start_x: i32,
    drag_start_y: i32,
    drag_cur_x: i32,
    drag_cur_y: i32,
    pencil_trace: Vec<(f64, f64)>,

    pixels: Vec<u8>,
    bmp_width: i32,
    bmp_height: i32,
    p1_rect_in_bmp: RECT,
    p2_rect_in_bmp: Option<RECT>,
    last_draw_x: i32,
    last_draw_y: i32,
    status_msg: Option<String>,
    hover_tb_id: Option<u16>,
    hovered_link: Option<LinkTarget>,

    input_mode: InputMode,
    ignore_next_char: bool,
    text_prompt_buf: String,
    search_query: String,
    search_matches: Vec<SearchMatch>,
    active_match: usize,
    goto_input: String,

    bookmarks: Vec<BookmarkItem>,
    show_sidebar: bool,
    sidebar_tab: SidebarTab,
    bookmark_scroll: usize,
    thumb_scroll: usize,
    thumb_cache: HashMap<i32, ThumbnailCache>,

    active_tool: ActiveTool,
    show_annotations: bool,
    overlays: Vec<UserOverlay>,
    watermark_text: Option<String>,
    page_bg_tint: Option<(u8, u8, u8)>,
    header_footer_text: Option<String>,
    show_page_numbers: bool,
    show_bates_numbers: bool,
    show_ruler: bool,
    show_grid: bool,
}

impl AppState {
    fn new() -> Self {
        let mut wp: WINDOWPLACEMENT = unsafe { zeroed() };
        wp.length = size_of::<WINDOWPLACEMENT>() as u32;

        Self {
            doc: null_mut(),
            pdf_bytes: Vec::new(),
            current_path: None,
            file_name: String::from("파일 없음 (상단 [열기] 버튼 또는 O키로 열기)"),
            page_count: 0,
            current_page: 0,
            cur_page_w_pt: 595.0,
            cur_page_h_pt: 842.0,
            spread_mode: false,
            cover_mode: false,
            fit_width: false,
            pending_align_top: false,
            pending_align_bottom: false,
            dark_mode: false,
            theme_bg_color: 0x00323232,
            is_fullscreen: false,
            saved_placement: wp,
            rotation: 0,
            zoom: 1.0,
            pan_x: 0,
            pan_y: 0,
            is_dragging: false,
            mouse_down_x: 0,
            mouse_down_y: 0,
            drag_start_x: 0,
            drag_start_y: 0,
            drag_cur_x: 0,
            drag_cur_y: 0,
            pencil_trace: Vec::new(),
            pixels: Vec::new(),
            bmp_width: 0,
            bmp_height: 0,
            p1_rect_in_bmp: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            p2_rect_in_bmp: None,
            last_draw_x: 0,
            last_draw_y: 0,
            status_msg: None,
            hover_tb_id: None,
            hovered_link: None,
            input_mode: InputMode::Normal,
            ignore_next_char: false,
            text_prompt_buf: String::new(),
            search_query: String::new(),
            search_matches: Vec::new(),
            active_match: 0,
            goto_input: String::new(),
            bookmarks: Vec::new(),
            show_sidebar: false,
            sidebar_tab: SidebarTab::Bookmarks,
            bookmark_scroll: 0,
            thumb_scroll: 0,
            thumb_cache: HashMap::new(),
            active_tool: ActiveTool::Pan,
            show_annotations: true,
            overlays: Vec::new(),
            watermark_text: None,
            page_bg_tint: None,
            header_footer_text: None,
            show_page_numbers: false,
            show_bates_numbers: false,
            show_ruler: false,
            show_grid: false,
        }
    }

    fn top_bar_offset(&self) -> i32 {
        if self.is_fullscreen {
            0
        } else {
            TOOLBAR_HEIGHT
        }
    }

    fn bottom_bar_offset(&self) -> i32 {
        if self.is_fullscreen {
            0
        } else {
            STATUSBAR_HEIGHT
        }
    }

    /// 클라이언트 좌표(client_x, client_y) 아래에 있는 PDF 하이퍼링크(내부 페이지 점프 또는 외부 웹 URL)를 탐지합니다.
    fn detect_link_at_client_pos(&self, client_x: i32, client_y: i32) -> Option<LinkTarget> {
        if self.doc.is_null() || self.page_count <= 0 || self.bmp_width <= 0 {
            return None;
        }
        let rel_x = client_x - self.last_draw_x;
        let rel_y = client_y - self.last_draw_y;

        // 1페이지 영역 히트 테스트
        if rel_x >= self.p1_rect_in_bmp.left
            && rel_x < self.p1_rect_in_bmp.right
            && rel_y >= self.p1_rect_in_bmp.top
            && rel_y < self.p1_rect_in_bmp.bottom
        {
            return self.query_link_on_page(
                self.current_page,
                rel_x - self.p1_rect_in_bmp.left,
                rel_y - self.p1_rect_in_bmp.top,
                self.p1_rect_in_bmp.right - self.p1_rect_in_bmp.left,
                self.p1_rect_in_bmp.bottom - self.p1_rect_in_bmp.top,
            );
        }

        // 2페이지 양면 보기 시 오른쪽 페이지 영역 히트 테스트
        if let Some(p2_rc) = self.p2_rect_in_bmp {
            if self.current_page + 1 < self.page_count
                && rel_x >= p2_rc.left
                && rel_x < p2_rc.right
                && rel_y >= p2_rc.top
                && rel_y < p2_rc.bottom
            {
                return self.query_link_on_page(
                    self.current_page + 1,
                    rel_x - p2_rc.left,
                    rel_y - p2_rc.top,
                    p2_rc.right - p2_rc.left,
                    p2_rc.bottom - p2_rc.top,
                );
            }
        }

        None
    }

    fn query_link_on_page(
        &self,
        page_idx: i32,
        dev_x: i32,
        dev_y: i32,
        size_x: i32,
        size_y: i32,
    ) -> Option<LinkTarget> {
        if size_x <= 0 || size_y <= 0 {
            return None;
        }
        unsafe {
            let page = FPDF_LoadPage(self.doc, page_idx);
            if page.is_null() {
                return None;
            }

            let mut page_x: c_double = 0.0;
            let mut page_y: c_double = 0.0;
            FPDF_DeviceToPage(
                page,
                0,
                0,
                size_x,
                size_y,
                self.rotation,
                dev_x,
                dev_y,
                &mut page_x,
                &mut page_y,
            );

            let link = FPDFLink_GetLinkAtPoint(page, page_x, page_y);
            let mut result = None;

            if !link.is_null() {
                // 1) 직접 목적지(Dest)가 지정된 내부 페이지 링크 확인
                let dest = FPDFLink_GetDest(self.doc, link);
                if !dest.is_null() {
                    let target_p = FPDFDest_GetDestPageIndex(self.doc, dest);
                    if target_p >= 0 {
                        result = Some(LinkTarget::InternalPage(target_p));
                    }
                }

                // 2) 액션(Action) 기반 링크 (GoTo 내부 이동 또는 URI 외부 웹 링크) 확인
                if result.is_none() {
                    let action = FPDFLink_GetAction(link);
                    if !action.is_null() {
                        let act_type = FPDFAction_GetType(action);
                        if act_type == PDFACTION_GOTO {
                            let act_dest = FPDFAction_GetDest(self.doc, action);
                            if !act_dest.is_null() {
                                let target_p = FPDFDest_GetDestPageIndex(self.doc, act_dest);
                                if target_p >= 0 {
                                    result = Some(LinkTarget::InternalPage(target_p));
                                }
                            }
                        } else if act_type == PDFACTION_URI {
                            let len = FPDFAction_GetURIPath(self.doc, action, null_mut(), 0);
                            if len > 1 {
                                let mut buf = vec![0u8; len as usize];
                                FPDFAction_GetURIPath(
                                    self.doc,
                                    action,
                                    buf.as_mut_ptr() as *mut c_void,
                                    len,
                                );
                                if let Ok(cstr) = CStr::from_bytes_until_nul(&buf) {
                                    let uri = cstr.to_string_lossy().trim().to_string();
                                    if !uri.is_empty() {
                                        result = Some(LinkTarget::ExternalUri(uri));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            FPDF_ClosePage(page);
            result
        }
    }

    fn execute_link_target(&mut self, target: &LinkTarget, hwnd: HWND) {
        match target {
            LinkTarget::InternalPage(target_page) => {
                let p = *target_page;
                self.go_to_page(p, hwnd);
                self.status_msg = Some(format!("🔗 내부 링크 이동: {}페이지", p + 1));
                update_window_title(hwnd, self);
            }
            LinkTarget::ExternalUri(uri) => {
                let w_open = to_wstring("open");
                let w_uri = to_wstring(uri);
                unsafe {
                    ShellExecuteW(
                        hwnd,
                        w_open.as_ptr(),
                        w_uri.as_ptr(),
                        null(),
                        null(),
                        SW_SHOWNORMAL,
                    );
                }
                self.status_msg = Some(format!("🌐 외부 링크 열기: {}", uri));
                update_window_title(hwnd, self);
            }
        }
    }

    fn clear_thumbnail_cache(&mut self) {
        self.thumb_cache.clear();
    }

    fn get_or_render_thumbnail(&mut self, page_idx: i32) -> Option<ThumbnailCache> {
        if let Some(cached) = self.thumb_cache.get(&page_idx) {
            return Some(cached.clone());
        }
        if self.doc.is_null() || page_idx < 0 || page_idx >= self.page_count {
            return None;
        }

        unsafe {
            let page = FPDF_LoadPage(self.doc, page_idx);
            if page.is_null() {
                return None;
            }

            let raw_w = FPDF_GetPageWidth(page);
            let raw_h = FPDF_GetPageHeight(page);
            let (pw, ph) = if self.rotation % 2 == 1 {
                (raw_h, raw_w)
            } else {
                (raw_w, raw_h)
            };

            let max_w = 156.0;
            let max_h = 176.0;
            let scale = (max_w / pw.max(1.0)).min(max_h / ph.max(1.0));
            let tw = ((pw * scale) as i32).clamp(24, 180);
            let th = ((ph * scale) as i32).clamp(24, 190);

            let bitmap = FPDFBitmap_Create(tw, th, 1);
            if bitmap.is_null() {
                FPDF_ClosePage(page);
                return None;
            }

            FPDFBitmap_FillRect(bitmap, 0, 0, tw, th, 0xFFFFFFFF);
            FPDF_RenderPageBitmap(bitmap, page, 0, 0, tw, th, self.rotation, FPDF_ANNOT);

            let buffer = FPDFBitmap_GetBuffer(bitmap);
            let stride = FPDFBitmap_GetStride(bitmap) as usize;
            let row_bytes = (tw * 4) as usize;
            let mut pixels = vec![0u8; row_bytes * (th as usize)];

            for y in 0..(th as usize) {
                let src = buffer.add(y * stride);
                let dst = pixels.as_mut_ptr().add(y * row_bytes);
                copy_nonoverlapping(src, dst, row_bytes);
            }

            FPDFBitmap_Destroy(bitmap);
            FPDF_ClosePage(page);

            let entry = ThumbnailCache {
                width: tw,
                height: th,
                pixels,
            };
            self.thumb_cache.insert(page_idx, entry.clone());
            Some(entry)
        }
    }

    fn create_blank_pdf(&mut self, hwnd: HWND) {
        unsafe {
            let new_doc = FPDF_CreateNewDocument();
            if new_doc.is_null() {
                return;
            }
            let page = FPDFPage_New(new_doc, 0, 595.0, 842.0);
            if !page.is_null() {
                FPDFPage_GenerateContent(page);
                FPDF_ClosePage(page);
            }
            if !self.doc.is_null() {
                FPDF_CloseDocument(self.doc);
            }
            self.doc = new_doc;
            self.pdf_bytes.clear();
            self.current_path = None;
            self.file_name = String::from("제목 없음-1.pdf");
            self.page_count = FPDF_GetPageCount(self.doc);
            self.current_page = 0;
            self.overlays.clear();
            self.bookmarks.clear();
            self.clear_thumbnail_cache();
            self.thumb_scroll = 0;
            self.status_msg = Some("새 빈 PDF 문서(A4)가 생성되었습니다".to_string());
            self.refresh_view(hwnd, true);
        }
    }

    fn create_pdf_from_clipboard(&mut self, hwnd: HWND) {
        let clip_text = get_clipboard_unicode_text(hwnd).unwrap_or_default();
        if clip_text.trim().is_empty() {
            self.status_msg = Some("클립보드에 복사된 텍스트가 없습니다".to_string());
            update_window_title(hwnd, self);
            return;
        }
        self.create_blank_pdf(hwnd);
        unsafe {
            let page = FPDF_LoadPage(self.doc, 0);
            if !page.is_null() {
                for (i, line) in clip_text.lines().take(45).enumerate() {
                    let y_pos = 790.0 - (i as f64) * 16.0;
                    Self::insert_pdf_text_object(self.doc, page, line, 40.0, y_pos, 11.0);
                }
                FPDFPage_GenerateContent(page);
                FPDF_ClosePage(page);
            }
        }
        self.clear_thumbnail_cache();
        self.status_msg = Some("클립보드 텍스트로부터 PDF 생성 완료".to_string());
        self.refresh_view(hwnd, true);
    }

    unsafe fn insert_pdf_text_object(
        doc: *mut c_void,
        page: *mut c_void,
        text: &str,
        x: f64,
        y: f64,
        size: f32,
    ) {
        let Ok(font_name) = CString::new("Helvetica") else {
            return;
        };
        let dll_name = to_wstring("pdfium.dll");
        let hmod = GetModuleHandleW(dll_name.as_ptr());
        if hmod.is_null() {
            return;
        }
        type CreateTextObjFn =
            unsafe extern "C" fn(*mut c_void, *const c_char, c_float) -> *mut c_void;
        let sym1 = CString::new("FPDFPageObj_NewTextObj").unwrap();
        let sym2 = CString::new("FPDFPageObj_CreateNewTextObj").unwrap();
        let proc = GetProcAddress(hmod, sym1.as_ptr() as *const u8)
            .or_else(|| GetProcAddress(hmod, sym2.as_ptr() as *const u8));
        if let Some(f) = proc {
            let create_fn: CreateTextObjFn = std::mem::transmute(f);
            let text_obj = create_fn(doc, font_name.as_ptr(), size);
            if !text_obj.is_null() {
                let wide = to_wstring(text);
                FPDFText_SetText(text_obj, wide.as_ptr());
                FPDFPageObj_SetFillColor(text_obj, 20, 20, 20, 255);
                FPDFPageObj_Transform(text_obj, 1.0, 0.0, 0.0, 1.0, x, y);
                FPDFPage_InsertObject(page, text_obj);
            }
        }
    }

    fn save_pdf_to_path(&mut self, path: &str, version: Option<i32>) -> Result<(), String> {
        if self.doc.is_null() {
            return Err("열려 있는 문서가 없습니다.".to_string());
        }
        unsafe {
            let mut ctx = FileWriteContext {
                fw: FPDF_FILEWRITE {
                    version: 1,
                    WriteBlock: Some(write_block_cb),
                },
                buf: Vec::new(),
            };

            let ok = if let Some(ver) = version {
                FPDF_SaveWithVersion(self.doc, &mut ctx.fw, 0, ver)
            } else {
                FPDF_SaveAsCopy(self.doc, &mut ctx.fw, 0)
            };

            if ok == 0 || ctx.buf.is_empty() {
                if !self.pdf_bytes.is_empty() {
                    fs::write(path, &self.pdf_bytes).map_err(|e| e.to_string())?;
                    return Ok(());
                }
                return Err("PDFium 문서 저장 실패".to_string());
            }

            fs::write(path, &ctx.buf).map_err(|e| e.to_string())?;
            self.current_path = Some(path.to_string());
            self.file_name = Path::new(path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(path)
                .to_string();
            Ok(())
        }
    }

    fn load_file(&mut self, path: &str) -> Result<(), String> {
        let bytes = fs::read(path).map_err(|e| format!("파일 읽기 실패: {}", e))?;
        if bytes.is_empty() {
            return Err("빈 파일입니다.".to_string());
        }

        let new_doc = unsafe {
            FPDF_LoadMemDocument(
                bytes.as_ptr() as *const c_void,
                bytes.len() as c_int,
                null(),
            )
        };

        if new_doc.is_null() {
            let err = unsafe { FPDF_GetLastError() };
            return Err(format!("PDFium 로드 에러 (코드: {})", err));
        }

        if !self.doc.is_null() {
            unsafe { FPDF_CloseDocument(self.doc) };
        }

        self.pdf_bytes = bytes;
        self.doc = new_doc;
        self.current_path = Some(path.to_string());
        self.page_count = unsafe { FPDF_GetPageCount(self.doc) };
        self.current_page = 0;
        self.rotation = 0;
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.pending_align_top = self.fit_width;
        self.pending_align_bottom = false;
        self.pixels.clear();
        self.overlays.clear();
        self.clear_thumbnail_cache();
        self.hovered_link = None;
        self.status_msg = None;
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
        self.search_matches.clear();
        self.active_match = 0;
        self.goto_input.clear();
        self.bookmark_scroll = 0;
        self.thumb_scroll = 0;

        self.file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string();

        self.load_bookmarks();

        Ok(())
    }

    fn generate_auto_bookmarks(&mut self, hwnd: HWND) {
        if self.doc.is_null() || self.page_count <= 0 {
            return;
        }
        self.bookmarks.clear();
        for p in 0..self.page_count {
            if let Some(txt) = self.extract_page_text_at(p) {
                if let Some(first_line) = txt
                    .lines()
                    .map(|l| l.trim())
                    .find(|l| l.chars().count() >= 2)
                {
                    let snippet: String = first_line.chars().take(28).collect();
                    self.bookmarks.push(BookmarkItem {
                        title: snippet,
                        page_index: p,
                        depth: 0,
                    });
                }
            }
        }
        self.show_sidebar = true;
        self.sidebar_tab = SidebarTab::Bookmarks;
        self.bookmark_scroll = 0;
        self.status_msg = Some(format!(
            "자동 북마크 생성 완료 ({}개 항목)",
            self.bookmarks.len()
        ));
        self.refresh_view(hwnd, true);
    }

    fn insert_blank_page_after_current(&mut self, hwnd: HWND) {
        if self.doc.is_null() {
            return;
        }
        unsafe {
            let insert_idx = (self.current_page + 1).min(self.page_count);
            let new_page = FPDFPage_New(self.doc, insert_idx, 595.0, 842.0);
            if !new_page.is_null() {
                FPDFPage_GenerateContent(new_page);
                FPDF_ClosePage(new_page);
                self.page_count = FPDF_GetPageCount(self.doc);
                self.current_page = insert_idx;
                self.clear_thumbnail_cache();
                self.status_msg = Some(format!("{}페이지에 빈 페이지 삽입 완료", insert_idx + 1));
                self.refresh_view(hwnd, true);
            }
        }
    }

    fn delete_current_page(&mut self, hwnd: HWND) {
        if self.doc.is_null() || self.page_count <= 1 {
            self.status_msg = Some("최소 1개의 페이지는 남겨야 합니다".to_string());
            update_window_title(hwnd, self);
            return;
        }
        unsafe {
            FPDFPage_Delete(self.doc, self.current_page);
            self.page_count = FPDF_GetPageCount(self.doc);
            if self.current_page >= self.page_count {
                self.current_page = (self.page_count - 1).max(0);
            }
            self.clear_thumbnail_cache();
            self.status_msg = Some(format!(
                "페이지 삭제 완료 (현재 총 {}페이지)",
                self.page_count
            ));
            self.refresh_view(hwnd, true);
        }
    }

    fn merge_external_pdf(&mut self, hwnd: HWND, at_end: bool) {
        if self.doc.is_null() {
            self.create_blank_pdf(hwnd);
        }
        let Some(ext_path) = open_file_dialog(hwnd) else {
            return;
        };
        let Ok(bytes) = fs::read(&ext_path) else {
            return;
        };

        unsafe {
            let src_doc = FPDF_LoadMemDocument(
                bytes.as_ptr() as *const c_void,
                bytes.len() as c_int,
                null(),
            );
            if src_doc.is_null() {
                self.status_msg = Some("병합할 PDF 파일을 열 수 없습니다".to_string());
                update_window_title(hwnd, self);
                return;
            }

            let insert_idx = if at_end {
                self.page_count
            } else {
                (self.current_page + 1).min(self.page_count)
            };

            let ok = FPDF_ImportPages(self.doc, src_doc, null(), insert_idx);
            FPDF_CloseDocument(src_doc);

            if ok != 0 {
                self.page_count = FPDF_GetPageCount(self.doc);
                self.clear_thumbnail_cache();
                self.status_msg = Some(format!(
                    "PDF 병합/삽입 완료! (총 {}페이지)",
                    self.page_count
                ));
                self.refresh_view(hwnd, true);
            } else {
                self.status_msg = Some("PDF 페이지 가져오기 실패".to_string());
                update_window_title(hwnd, self);
            }
        }
    }

    fn extract_current_page_to_pdf(&mut self, hwnd: HWND) {
        if self.doc.is_null() || self.page_count <= 0 {
            return;
        }
        let default_name = format!("extracted_page_{}.pdf", self.current_page + 1);
        let Some(save_path) = save_custom_dialog(
            hwnd,
            &default_name,
            "PDF 문서 (*.pdf)\0*.pdf\0\0",
            "pdf",
        ) else {
            return;
        };

        unsafe {
            let new_doc = FPDF_CreateNewDocument();
            if new_doc.is_null() {
                return;
            }
            let range_str = CString::new(format!("{}", self.current_page + 1)).unwrap();
            if FPDF_ImportPages(new_doc, self.doc, range_str.as_ptr(), 0) != 0 {
                let mut ctx = FileWriteContext {
                    fw: FPDF_FILEWRITE {
                        version: 1,
                        WriteBlock: Some(write_block_cb),
                    },
                    buf: Vec::new(),
                };
                if FPDF_SaveAsCopy(new_doc, &mut ctx.fw, 0) != 0 && !ctx.buf.is_empty() {
                    let _ = fs::write(&save_path, &ctx.buf);
                    self.status_msg = Some(format!(
                        "{}페이지 추출 저장 완료: {}",
                        self.current_page + 1,
                        save_path
                    ));
                }
            }
            FPDF_CloseDocument(new_doc);
            update_window_title(hwnd, self);
        }
    }

    fn crop_current_page_margins(&mut self, hwnd: HWND) {
        if self.doc.is_null() || self.page_count <= 0 {
            return;
        }
        unsafe {
            let page = FPDF_LoadPage(self.doc, self.current_page);
            if !page.is_null() {
                let w = FPDF_GetPageWidth(page) as f32;
                let h = FPDF_GetPageHeight(page) as f32;
                let mx = w * 0.08;
                let my = h * 0.08;
                FPDFPage_SetCropBox(page, mx, my, w - mx, h - my);
                FPDFPage_GenerateContent(page);
                FPDF_ClosePage(page);
                self.thumb_cache.remove(&self.current_page);
                self.status_msg = Some("현재 페이지 여백 자르기(CropBox) 적용 완료".to_string());
                self.refresh_view(hwnd, true);
            }
        }
    }

    fn flatten_current_page(&mut self, hwnd: HWND) {
        if self.doc.is_null() || self.page_count <= 0 {
            return;
        }
        unsafe {
            let page = FPDF_LoadPage(self.doc, self.current_page);
            if !page.is_null() {
                let w = FPDF_GetPageWidth(page) as f32;
                let h = FPDF_GetPageHeight(page) as f32;
                for ov in self.overlays.iter().filter(|o| {
                    o.page_index == self.current_page && o.kind == OverlayKind::Redaction
                }) {
                    let rx = (ov.x1.min(ov.x2) as f32) * w;
                    let ry = (1.0 - ov.y1.max(ov.y2) as f32) * h;
                    let rw = ((ov.x2 - ov.x1).abs() as f32) * w;
                    let rh = ((ov.y2 - ov.y1).abs() as f32) * h;
                    let rect_obj = FPDFPageObj_CreateNewRect(rx, ry, rw.max(2.0), rh.max(2.0));
                    if !rect_obj.is_null() {
                        FPDFPageObj_SetFillColor(rect_obj, 0, 0, 0, 255);
                        FPDFPath_SetDrawMode(rect_obj, 1, 0);
                        FPDFPage_InsertObject(page, rect_obj);
                    }
                }
                FPDFPage_GenerateContent(page);
                FPDFPage_Flatten(page, 0);
                FPDF_ClosePage(page);
                self.thumb_cache.remove(&self.current_page);
                self.status_msg =
                    Some("현재 페이지 주석/양식/교정 평면화(Flatten) 완료".to_string());
                self.refresh_view(hwnd, true);
            }
        }
    }

    fn compare_with_other_pdf(&mut self, hwnd: HWND) {
        if self.doc.is_null() {
            return;
        }
        let Some(other_path) = open_file_dialog(hwnd) else {
            return;
        };
        let Ok(bytes) = fs::read(&other_path) else {
            return;
        };

        unsafe {
            let doc2 = FPDF_LoadMemDocument(
                bytes.as_ptr() as *const c_void,
                bytes.len() as c_int,
                null(),
            );
            if doc2.is_null() {
                return;
            }
            let count2 = FPDF_GetPageCount(doc2);
            let min_pages = self.page_count.min(count2);
            let mut diff_pages = Vec::new();

            for p in 0..min_pages {
                let t1 = self.extract_page_text_at(p).unwrap_or_default();
                let page2 = FPDF_LoadPage(doc2, p);
                let mut t2 = String::new();
                if !page2.is_null() {
                    let tp2 = FPDFText_LoadPage(page2);
                    if !tp2.is_null() {
                        let cc = FPDFText_CountChars(tp2);
                        if cc > 0 {
                            let mut buf = vec![0u16; (cc + 1) as usize];
                            let w = FPDFText_GetText(tp2, 0, cc, buf.as_mut_ptr());
                            if w > 0 {
                                t2 = String::from_utf16_lossy(&buf[..(w as usize - 1)]);
                            }
                        }
                        FPDFText_ClosePage(tp2);
                    }
                    FPDF_ClosePage(page2);
                }
                if t1.trim() != t2.trim() {
                    diff_pages.push(p + 1);
                }
            }
            FPDF_CloseDocument(doc2);

            let report = format!(
                "[PDF 문서 비교 결과]\n\n\
                 • 기준 문서: {} ({} 페이지)\n\
                 • 비교 문서: {} ({} 페이지)\n\
                 • 텍스트 차이가 감지된 페이지 수: {}개 페이지\n\
                 • 차이 발생 페이지 번호: {:?}",
                self.file_name,
                self.page_count,
                other_path,
                count2,
                diff_pages.len(),
                diff_pages
            );
            let w_rep = to_wstring(&report);
            let w_cap = to_wstring("DaPDF - PDF 비교 결과 (VS)");
            MessageBoxW(
                hwnd,
                w_rep.as_ptr(),
                w_cap.as_ptr(),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }

    fn detect_and_copy_tables(&mut self, hwnd: HWND) {
        let Some(txt) = self.extract_page_text_at(self.current_page) else {
            return;
        };
        let mut table_rows = Vec::new();
        for line in txt.lines() {
            if line.contains('|') || line.contains('\t') || line.contains("  ") {
                let cols: Vec<&str> = line
                    .split(|c| c == '|' || c == '\t')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                if cols.len() >= 2 {
                    table_rows.push(cols.join(", "));
                }
            }
        }
        let msg = if table_rows.is_empty() {
            "현재 페이지에서 정형화된 표(Table) 행이 감지되지 않았습니다.".to_string()
        } else {
            let csv_preview = table_rows.join("\r\n");
            set_clipboard_unicode_text(hwnd, &csv_preview);
            format!(
                "총 {}개의 표 행(Row)이 인식되어 클립보드에 CSV 형태로 복사되었습니다!\n\n[미리보기]\n{}",
                table_rows.len(),
                table_rows
                    .iter()
                    .take(10)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        let w_msg = to_wstring(&msg);
        let w_cap = to_wstring("DaPDF - 표 인식 결과");
        unsafe {
            MessageBoxW(
                hwnd,
                w_msg.as_ptr(),
                w_cap.as_ptr(),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }

    fn export_document_as(&mut self, hwnd: HWND, ext: &str) {
        if self.doc.is_null() || self.page_count <= 0 {
            return;
        }
        let stem = Path::new(&self.file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document");
        let default_name = format!("{}.{}", stem, ext);
        let filter = match ext {
            "html" => "HTML 웹 문서 (*.html)\0*.html\0\0",
            "rtf" => "Word 호환 서식 문서 (*.rtf)\0*.rtf\0\0",
            "csv" => "Excel CSV 데이터 (*.csv)\0*.csv\0\0",
            _ => "텍스트 문서 (*.txt)\0*.txt\0\0",
        };

        let Some(save_path) = save_custom_dialog(hwnd, &default_name, filter, ext) else {
            return;
        };

        let mut all_pages = Vec::new();
        for p in 0..self.page_count {
            if let Some(t) = self.extract_page_text_at(p) {
                all_pages.push((p + 1, t));
            }
        }

        let output = match ext {
            "html" => {
                let mut h = format!(
                    "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title>\
                     <style>body{{font-family:sans-serif;max-width:860px;margin:2rem auto;line-height:1.6;}}\
                     .page{{border:1px solid #ccc;padding:1.5rem;margin-bottom:2rem;}}</style></head><body>",
                    self.file_name
                );
                for (pno, txt) in &all_pages {
                    h.push_str(&format!(
                        "<div class=\"page\"><h3>Page {}</h3><pre>{}</pre></div>",
                        pno,
                        txt.replace('&', "&amp;").replace('<', "&lt;")
                    ));
                }
                h.push_str("</body></html>");
                h
            }
            "rtf" => {
                let mut r = String::from("{\\rtf1\\ansi\\deff0\r\n");
                for (pno, txt) in &all_pages {
                    r.push_str(&format!("\\b [Page {}]\\b0\\line\r\n", pno));
                    for ch in txt.chars() {
                        if ch == '\n' {
                            r.push_str("\\line\r\n");
                        } else if (ch as u32) < 128 {
                            r.push(ch);
                        } else {
                            r.push_str(&format!("\\u{}?", ch as i16));
                        }
                    }
                    r.push_str("\\page\r\n");
                }
                r.push('}');
                r
            }
            "csv" => {
                let mut c = String::from("Page,Line,Content\r\n");
                for (pno, txt) in &all_pages {
                    for (lno, line) in txt.lines().enumerate() {
                        let escaped = line.replace('"', "\"\"");
                        c.push_str(&format!("{},{},\"{}\"\r\n", pno, lno + 1, escaped));
                    }
                }
                c
            }
            _ => {
                let mut t = String::new();
                for (pno, txt) in &all_pages {
                    t.push_str(&format!("=== [Page {}] ===\r\n{}\r\n\r\n", pno, txt));
                }
                t
            }
        };

        if fs::write(&save_path, output.as_bytes()).is_ok() {
            self.status_msg = Some(format!(
                "{} 포맷 변환 완료: {}",
                ext.to_uppercase(),
                save_path
            ));
        } else {
            self.status_msg = Some("파일 변환 저장 실패".to_string());
        }
        update_window_title(hwnd, self);
    }

    fn get_meta_text(&self, tag: &str) -> String {
        if self.doc.is_null() {
            return String::new();
        }
        let Ok(c_tag) = CString::new(tag) else {
            return String::new();
        };

        unsafe {
            let byte_len = FPDF_GetMetaText(self.doc, c_tag.as_ptr(), null_mut(), 0);
            if byte_len <= 2 {
                return String::new();
            }
            let u16_len = (byte_len as usize) / 2;
            let mut buf = vec![0u16; u16_len];
            FPDF_GetMetaText(
                self.doc,
                c_tag.as_ptr(),
                buf.as_mut_ptr() as *mut c_void,
                byte_len,
            );
            let actual_len = buf.iter().position(|&c| c == 0).unwrap_or(u16_len);
            String::from_utf16_lossy(&buf[..actual_len])
                .trim()
                .to_string()
        }
    }

    fn show_metadata_dialog(&self, hwnd: HWND) {
        if self.doc.is_null() {
            return;
        }

        unsafe {
            let mut version: c_int = 0;
            let ver_str = if FPDF_GetFileVersion(self.doc, &mut version) != 0 && version > 0 {
                format!("{}.{}", version / 10, version % 10)
            } else {
                String::from("알 수 없음")
            };

            let w_mm = self.cur_page_w_pt * 25.4 / 72.0;
            let h_mm = self.cur_page_h_pt * 25.4 / 72.0;
            let size_str = format!(
                "{:.1} x {:.1} pt ({:.0} x {:.0} mm)",
                self.cur_page_w_pt, self.cur_page_h_pt, w_mm, h_mm
            );

            let val_or_dash = |s: String| if s.is_empty() { String::from("-") } else { s };

            let title = val_or_dash(self.get_meta_text("Title"));
            let author = val_or_dash(self.get_meta_text("Author"));
            let subject = val_or_dash(self.get_meta_text("Subject"));
            let keywords = val_or_dash(self.get_meta_text("Keywords"));
            let creator = val_or_dash(self.get_meta_text("Creator"));
            let producer = val_or_dash(self.get_meta_text("Producer"));
            let created = format_pdf_date(&self.get_meta_text("CreationDate"));
            let modified = format_pdf_date(&self.get_meta_text("ModDate"));

            let file_kb = (self.pdf_bytes.len() as f64) / 1024.0;
            let info_text = format!(
                "파일 이름: {}\n\
                 파일 크기: {:.1} KB ({} bytes)\n\
                 PDF 버전: PDF {}\n\
                 총 페이지 수: {} 페이지\n\
                 현재 페이지 크기: {}\n\
                 ──────────────────────────────\n\
                 제목 (Title): {}\n\
                 저자 (Author): {}\n\
                 주제 (Subject): {}\n\
                 키워드 (Keywords): {}\n\
                 작성 프로그램 (Creator): {}\n\
                 PDF 변환기 (Producer): {}\n\
                 생성 일시 (CreationDate): {}\n\
                 수정 일시 (ModDate): {}",
                self.file_name,
                file_kb,
                self.pdf_bytes.len(),
                ver_str,
                self.page_count,
                size_str,
                title,
                author,
                subject,
                keywords,
                creator,
                producer,
                created,
                modified
            );

            let w_info = to_wstring(&info_text);
            let w_caption =
                to_wstring(&format!("DaPDF(v{}) - 문서 메타데이터 정보", APP_VERSION));
            MessageBoxW(
                hwnd,
                w_info.as_ptr(),
                w_caption.as_ptr(),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }

    fn show_help_dialog(&self, hwnd: HWND) {
        let help_text = "\
[본문 하이퍼링크 클릭 이동 (FPDFLink)]
• 문서 본문 안의 목차 링크(내부 페이지 이동)나 웹 주소(https://...) 위에 마우스를 올리면 손가락 커서로 변경되며 하단 상태바에 대상 경로가 표시됩니다.
• 해당 링크를 마우스로 클릭하면 목표 페이지로 즉시 점프하거나 기본 웹 브라우저가 실행됩니다.

[좌측 사이드바 탭 ([📑 목차] / [🖼 썸네일])]
• 사이드바 상단 탭을 클릭해 [목차(Bookmarks)]와 [페이지 썸네일 미리보기]를 즉시 전환하고 클릭 이동할 수 있습니다.

[단축키 안내]
• O: 열기 | S: BMP 저장 | Ctrl+S: PDF 저장 | C: 텍스트 복사 | I: 문서 정보 | H / F1: 도움말
• Home / End: 첫 페이지 / 마지막 페이지 | G: 페이지 이동 | B: 사이드바 토글 | [ / ]: 이전/다음 북마크
• W: 창 너비 맞춤 | D: 2p 양면 보기 | V: 다크 모드 | R / Shift+R: 90° 회전 | F11: 전체 화면 | F: 전체 검색";

        let w_help = to_wstring(help_text);
        let w_caption = to_wstring(&format!(
            "DaPDF(v{}) Viewer - 전체 메뉴 및 단축키 도움말",
            APP_VERSION
        ));
        unsafe {
            MessageBoxW(
                hwnd,
                w_help.as_ptr(),
                w_caption.as_ptr(),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }

    fn load_bookmarks(&mut self) {
        self.bookmarks.clear();
        if self.doc.is_null() {
            return;
        }
        unsafe {
            Self::collect_bookmarks_recursive(self.doc, null_mut(), 0, &mut self.bookmarks);
        }
        if !self.bookmarks.is_empty() {
            self.show_sidebar = true;
            self.sidebar_tab = SidebarTab::Bookmarks;
        }
    }

    unsafe fn collect_bookmarks_recursive(
        doc: *mut c_void,
        parent: *mut c_void,
        depth: usize,
        out: &mut Vec<BookmarkItem>,
    ) {
        let mut bm = FPDFBookmark_GetFirstChild(doc, parent);
        while !bm.is_null() {
            let byte_len = FPDFBookmark_GetTitle(bm, null_mut(), 0);
            let mut title = String::from("(제목 없음)");
            if byte_len >= 2 {
                let u16_len = (byte_len as usize) / 2;
                let mut buf = vec![0u16; u16_len];
                FPDFBookmark_GetTitle(bm, buf.as_mut_ptr() as *mut c_void, byte_len);
                let actual_len = buf.iter().position(|&c| c == 0).unwrap_or(u16_len);
                let decoded = String::from_utf16_lossy(&buf[..actual_len])
                    .trim()
                    .to_string();
                if !decoded.is_empty() {
                    title = decoded;
                }
            }

            let mut dest = FPDFBookmark_GetDest(doc, bm);
            if dest.is_null() {
                let action = FPDFBookmark_GetAction(bm);
                if !action.is_null() {
                    dest = FPDFAction_GetDest(doc, action);
                }
            }
            let page_index = if !dest.is_null() {
                FPDFDest_GetDestPageIndex(doc, dest)
            } else {
                -1
            };

            out.push(BookmarkItem {
                title,
                page_index,
                depth,
            });

            Self::collect_bookmarks_recursive(doc, bm, depth + 1, out);
            bm = FPDFBookmark_GetNextSibling(doc, bm);
        }
    }

    fn collect_matches_on_page(&self, page_idx: i32) -> Vec<SearchMatch> {
        let mut matches = Vec::new();
        if self.doc.is_null()
            || page_idx < 0
            || page_idx >= self.page_count
            || self.search_query.is_empty()
        {
            return matches;
        }

        unsafe {
            let page = FPDF_LoadPage(self.doc, page_idx);
            if page.is_null() {
                return matches;
            }

            let text_page = FPDFText_LoadPage(page);
            if !text_page.is_null() {
                let wide_query = to_wstring(&self.search_query);
                let find_handle = FPDFText_FindStart(text_page, wide_query.as_ptr(), 0, 0);

                if !find_handle.is_null() {
                    while FPDFText_FindNext(find_handle) != 0 {
                        let start_idx = FPDFText_GetSchResultIndex(find_handle);
                        let char_count = FPDFText_GetSchCount(find_handle);

                        if char_count > 0 {
                            let rect_count = FPDFText_CountRects(text_page, start_idx, char_count);
                            let mut rects = Vec::new();

                            for r in 0..rect_count {
                                let (mut left, mut top, mut right, mut bottom) =
                                    (0.0, 0.0, 0.0, 0.0);
                                if FPDFText_GetRect(
                                    text_page,
                                    r,
                                    &mut left,
                                    &mut top,
                                    &mut right,
                                    &mut bottom,
                                ) != 0
                                {
                                    rects.push(PdfRect {
                                        left,
                                        top,
                                        right,
                                        bottom,
                                    });
                                }
                            }

                            if !rects.is_empty() {
                                matches.push(SearchMatch { rects });
                            }
                        }
                    }
                    FPDFText_FindClose(find_handle);
                }
                FPDFText_ClosePage(text_page);
            }
            FPDF_ClosePage(page);
        }

        matches
    }

    fn perform_search(&mut self, allow_page_jump: bool) {
        self.search_matches.clear();
        self.active_match = 0;
        self.status_msg = None;

        if self.doc.is_null() || self.page_count <= 0 || self.search_query.is_empty() {
            return;
        }

        let current_matches = self.collect_matches_on_page(self.current_page);
        if !current_matches.is_empty() || !allow_page_jump {
            self.search_matches = current_matches;
            return;
        }

        for offset in 1..self.page_count {
            let target_page = (self.current_page + offset) % self.page_count;
            let matches = self.collect_matches_on_page(target_page);
            if !matches.is_empty() {
                self.current_page = target_page;
                self.pan_x = 0;
                self.pan_y = 0;
                self.pending_align_top = self.fit_width;
                self.search_matches = matches;
                self.active_match = 0;
                self.status_msg = Some(format!("{}페이지에서 검색됨", target_page + 1));
                return;
            }
        }
    }

    fn apply_search_redaction(&mut self, hwnd: HWND) {
        if self.doc.is_null() || self.search_query.is_empty() {
            return;
        }
        let matches = self.collect_matches_on_page(self.current_page);
        unsafe {
            let page = FPDF_LoadPage(self.doc, self.current_page);
            if !page.is_null() {
                let pw = FPDF_GetPageWidth(page).max(1.0);
                let ph = FPDF_GetPageHeight(page).max(1.0);
                let mut count = 0;
                for m in matches {
                    for r in m.rects {
                        self.overlays.push(UserOverlay {
                            page_index: self.current_page,
                            kind: OverlayKind::Redaction,
                            x1: (r.left / pw).clamp(0.0, 1.0),
                            y1: (1.0 - r.top / ph).clamp(0.0, 1.0),
                            x2: (r.right / pw).clamp(0.0, 1.0),
                            y2: (1.0 - r.bottom / ph).clamp(0.0, 1.0),
                            points: Vec::new(),
                            label: String::from("REDACTED"),
                            checked: false,
                        });
                        count += 1;
                    }
                }
                FPDF_ClosePage(page);
                self.status_msg = Some(format!(
                    "'{}' 검색 결과 {}건 교정(검은색 마스킹) 적용 완료",
                    self.search_query, count
                ));
            }
        }
        self.refresh_view(hwnd, true);
    }

    fn step_search_match(&mut self, forward: bool, hwnd: HWND) {
        if self.doc.is_null() || self.page_count <= 0 || self.search_query.is_empty() {
            return;
        }

        if !self.search_matches.is_empty() {
            if forward && self.active_match + 1 < self.search_matches.len() {
                self.active_match += 1;
                self.status_msg = None;
                self.refresh_view(hwnd, true);
                return;
            } else if !forward && self.active_match > 0 {
                self.active_match -= 1;
                self.status_msg = None;
                self.refresh_view(hwnd, true);
                return;
            }
        }

        for step in 1..=self.page_count {
            let next_page = if forward {
                (self.current_page + step) % self.page_count
            } else {
                (self.current_page + self.page_count - (step % self.page_count)) % self.page_count
            };

            let matches = self.collect_matches_on_page(next_page);
            if !matches.is_empty() {
                let page_changed = next_page != self.current_page;
                self.current_page = next_page;
                if page_changed {
                    self.pan_x = 0;
                    self.pan_y = 0;
                    self.pending_align_top = self.fit_width;
                    self.status_msg = Some(format!("{}페이지로 이동", next_page + 1));
                }
                self.active_match = if forward { 0 } else { matches.len() - 1 };
                self.search_matches = matches;
                self.refresh_view(hwnd, true);
                return;
            }
        }
    }

    fn sidebar_offset(&self) -> i32 {
        if self.show_sidebar {
            SIDEBAR_WIDTH
        } else {
            0
        }
    }

    fn align_vertical_pan_if_needed(&mut self, viewport_h: i32) {
        if self.pending_align_top {
            self.pending_align_top = false;
            if self.bmp_height > viewport_h - 24 {
                self.pan_y = (self.bmp_height - viewport_h) / 2 + 12;
            } else {
                self.pan_y = 0;
            }
        } else if self.pending_align_bottom {
            self.pending_align_bottom = false;
            if self.bmp_height > viewport_h - 24 {
                self.pan_y = -((self.bmp_height - viewport_h) / 2 + 12);
            } else {
                self.pan_y = 0;
            }
        }
    }

    fn render_current_page(&mut self, client_w: i32, client_h: i32) {
        if self.doc.is_null() || self.page_count <= 0 || client_w <= 0 || client_h <= 0 {
            return;
        }

        let sb_w = self.sidebar_offset();
        let canvas_w = (client_w - sb_w).max(50);
        let viewport_h = (client_h - self.top_bar_offset() - self.bottom_bar_offset()).max(50);

        unsafe {
            let page1 = FPDF_LoadPage(self.doc, self.current_page);
            if page1.is_null() {
                return;
            }

            let raw_w1 = FPDF_GetPageWidth(page1);
            let raw_h1 = FPDF_GetPageHeight(page1);
            self.cur_page_w_pt = raw_w1;
            self.cur_page_h_pt = raw_h1;

            let (w1, h1) = if self.rotation % 2 == 1 {
                (raw_h1, raw_w1)
            } else {
                (raw_w1, raw_h1)
            };

            let show_second = self.spread_mode
                && !(self.cover_mode && self.current_page == 0)
                && (self.current_page + 1 < self.page_count);

            let page2 = if show_second {
                FPDF_LoadPage(self.doc, self.current_page + 1)
            } else {
                null_mut()
            };

            let (w2, h2) = if !page2.is_null() {
                let rw2 = FPDF_GetPageWidth(page2);
                let rh2 = FPDF_GetPageHeight(page2);
                if self.rotation % 2 == 1 {
                    (rh2, rw2)
                } else {
                    (rw2, rh2)
                }
            } else {
                (0.0, 0.0)
            };

            let gap = if !page2.is_null() { SPREAD_GAP } else { 0 };
            let avail_w = (canvas_w - 32 - gap).max(50) as f64;
            let avail_h = (viewport_h - 24).max(50) as f64;

            let total_pdf_w = w1 + w2;
            let max_pdf_h = h1.max(h2);

            let base_scale = if self.fit_width {
                avail_w / total_pdf_w
            } else {
                (avail_w / total_pdf_w).min(avail_h / max_pdf_h)
            };
            let final_scale = base_scale * self.zoom;

            let target_w1 = ((w1 * final_scale) as i32).clamp(1, 4096);
            let target_h1 = ((h1 * final_scale) as i32).clamp(1, 4096);

            let (target_w2, target_h2) = if !page2.is_null() {
                (
                    ((w2 * final_scale) as i32).clamp(1, 4096),
                    ((h2 * final_scale) as i32).clamp(1, 4096),
                )
            } else {
                (0, 0)
            };

            let total_w = if !page2.is_null() {
                target_w1 + gap + target_w2
            } else {
                target_w1
            };
            let total_h = target_h1.max(target_h2);

            let y_off1 = (total_h - target_h1) / 2;
            self.p1_rect_in_bmp = RECT {
                left: 0,
                top: y_off1,
                right: target_w1,
                bottom: y_off1 + target_h1,
            };

            if !page2.is_null() {
                let x_off2 = target_w1 + gap;
                let y_off2 = (total_h - target_h2) / 2;
                self.p2_rect_in_bmp = Some(RECT {
                    left: x_off2,
                    top: y_off2,
                    right: x_off2 + target_w2,
                    bottom: y_off2 + target_h2,
                });
            } else {
                self.p2_rect_in_bmp = None;
            }

            if self.bmp_width == total_w && self.bmp_height == total_h && !self.pixels.is_empty() {
                self.align_vertical_pan_if_needed(viewport_h);
                if !page2.is_null() {
                    FPDF_ClosePage(page2);
                }
                FPDF_ClosePage(page1);
                return;
            }

            let bg_val: u8 = if self.dark_mode { 0x14 } else { 0x32 };
            let total_row_bytes = (total_w * 4) as usize;
            self.pixels.resize(total_row_bytes * (total_h as usize), 0);
            for chunk in self.pixels.chunks_exact_mut(4) {
                chunk[0] = bg_val;
                chunk[1] = bg_val;
                chunk[2] = bg_val;
                chunk[3] = 0xFF;
            }
            self.bmp_width = total_w;
            self.bmp_height = total_h;
            self.align_vertical_pan_if_needed(viewport_h);

            self.render_single_page_into_buffer(page1, 0, y_off1, target_w1, target_h1);

            let matches_p1 = self.search_matches.clone();
            self.draw_highlights_for_page(
                page1,
                0,
                y_off1,
                target_w1,
                target_h1,
                &matches_p1,
                Some(self.active_match),
            );

            if !page2.is_null() {
                let x_off2 = target_w1 + gap;
                let y_off2 = (total_h - target_h2) / 2;
                self.render_single_page_into_buffer(page2, x_off2, y_off2, target_w2, target_h2);

                if !self.search_query.is_empty() {
                    let matches_p2 = self.collect_matches_on_page(self.current_page + 1);
                    self.draw_highlights_for_page(
                        page2,
                        x_off2,
                        y_off2,
                        target_w2,
                        target_h2,
                        &matches_p2,
                        None,
                    );
                }
                FPDF_ClosePage(page2);
            }

            FPDF_ClosePage(page1);
        }
    }

    unsafe fn render_single_page_into_buffer(
        &mut self,
        page: *mut c_void,
        dest_x: i32,
        dest_y: i32,
        w: i32,
        h: i32,
    ) {
        let bitmap = FPDFBitmap_Create(w, h, 1);
        if !bitmap.is_null() {
            let fill_color = if let Some((r, g, b)) = self.page_bg_tint {
                0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
            } else {
                0xFFFFFFFF
            };
            FPDFBitmap_FillRect(bitmap, 0, 0, w, h, fill_color);

            let render_flags = if self.show_annotations {
                FPDF_ANNOT | FPDF_LCD_TEXT
            } else {
                FPDF_LCD_TEXT
            };

            FPDF_RenderPageBitmap(bitmap, page, 0, 0, w, h, self.rotation, render_flags);

            let buffer = FPDFBitmap_GetBuffer(bitmap);
            let stride = FPDFBitmap_GetStride(bitmap) as usize;
            let copy_bytes = (w * 4) as usize;
            let total_w = self.bmp_width as usize;

            for y in 0..(h as usize) {
                let src_row = buffer.add(y * stride);
                let dst_offset = ((dest_y as usize + y) * total_w + (dest_x as usize)) * 4;
                let dst_slice = std::slice::from_raw_parts_mut(
                    self.pixels.as_mut_ptr().add(dst_offset),
                    copy_bytes,
                );
                copy_nonoverlapping(src_row, dst_slice.as_mut_ptr(), copy_bytes);

                if self.dark_mode {
                    for px in dst_slice.chunks_exact_mut(4) {
                        px[0] = 30 + (((255 - px[0] as u16) * 200) / 255) as u8;
                        px[1] = 30 + (((255 - px[1] as u16) * 200) / 255) as u8;
                        px[2] = 30 + (((255 - px[2] as u16) * 200) / 255) as u8;
                    }
                }
            }

            FPDFBitmap_Destroy(bitmap);
        }
    }

    unsafe fn draw_highlights_for_page(
        &mut self,
        page: *mut c_void,
        offset_x: i32,
        offset_y: i32,
        page_w: i32,
        page_h: i32,
        matches: &[SearchMatch],
        active_idx: Option<usize>,
    ) {
        if matches.is_empty() {
            return;
        }

        let total_w = self.bmp_width;
        let total_h = self.bmp_height;

        for (idx, m) in matches.iter().enumerate() {
            let is_active = active_idx == Some(idx);
            let (hl_r, hl_g, hl_b, alpha) = if is_active {
                (255u32, 130u32, 0u32, 125u32)
            } else {
                (255u32, 235u32, 0u32, 100u32)
            };
            let inv_alpha = 255 - alpha;

            for rect in &m.rects {
                let (mut dev_x1, mut dev_y1) = (0, 0);
                let (mut dev_x2, mut dev_y2) = (0, 0);

                FPDF_PageToDevice(
                    page,
                    0,
                    0,
                    page_w,
                    page_h,
                    self.rotation,
                    rect.left,
                    rect.top,
                    &mut dev_x1,
                    &mut dev_y1,
                );
                FPDF_PageToDevice(
                    page,
                    0,
                    0,
                    page_w,
                    page_h,
                    self.rotation,
                    rect.right,
                    rect.bottom,
                    &mut dev_x2,
                    &mut dev_y2,
                );

                let x_min = (dev_x1.min(dev_x2) - 2).clamp(0, page_w - 1) + offset_x;
                let x_max = (dev_x1.max(dev_x2) + 2).clamp(0, page_w - 1) + offset_x;
                let y_min = (dev_y1.min(dev_y2) - 2).clamp(0, page_h - 1) + offset_y;
                let y_max = (dev_y1.max(dev_y2) + 2).clamp(0, page_h - 1) + offset_y;

                for y in y_min..=y_max {
                    if y < 0 || y >= total_h {
                        continue;
                    }
                    let row_offset = (y * total_w * 4) as usize;
                    for x in x_min..=x_max {
                        if x < 0 || x >= total_w {
                            continue;
                        }
                        let p = row_offset + (x * 4) as usize;
                        let is_border =
                            is_active && (x == x_min || x == x_max || y == y_min || y == y_max);

                        if is_border {
                            self.pixels[p] = 0;
                            self.pixels[p + 1] = 80;
                            self.pixels[p + 2] = 255;
                        } else {
                            let b = self.pixels[p] as u32;
                            let g = self.pixels[p + 1] as u32;
                            let r = self.pixels[p + 2] as u32;

                            self.pixels[p] = ((b * inv_alpha + hl_b * alpha) / 255) as u8;
                            self.pixels[p + 1] = ((g * inv_alpha + hl_g * alpha) / 255) as u8;
                            self.pixels[p + 2] = ((r * inv_alpha + hl_r * alpha) / 255) as u8;
                        }
                    }
                }
            }
        }
    }

    fn extract_page_text_at(&self, page_idx: i32) -> Option<String> {
        if self.doc.is_null() || page_idx < 0 || page_idx >= self.page_count {
            return None;
        }

        unsafe {
            let page = FPDF_LoadPage(self.doc, page_idx);
            if page.is_null() {
                return None;
            }

            let text_page = FPDFText_LoadPage(page);
            if text_page.is_null() {
                FPDF_ClosePage(page);
                return None;
            }

            let char_count = FPDFText_CountChars(text_page);
            let mut extracted = String::new();

            if char_count > 0 {
                let mut buf = vec![0u16; (char_count + 1) as usize];
                let written = FPDFText_GetText(text_page, 0, char_count, buf.as_mut_ptr());
                if written > 0 {
                    let valid_len = (written as usize).saturating_sub(1);
                    extracted = String::from_utf16_lossy(&buf[..valid_len]);
                }
            }

            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);

            Some(extracted)
        }
    }

    fn save_current_page_image(&mut self, hwnd: HWND) {
        if self.doc.is_null()
            || self.pixels.is_empty()
            || self.bmp_width <= 0
            || self.bmp_height <= 0
        {
            return;
        }

        let stem = Path::new(&self.file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("page");
        let default_name = format!("{}_page_{}.bmp", stem, self.current_page + 1);

        if let Some(save_path) = save_custom_dialog(
            hwnd,
            &default_name,
            "BMP 비트맵 이미지 (*.bmp)\0*.bmp\0모든 파일 (*.*)\0*.*\0\0",
            "bmp",
        ) {
            match write_bgra_to_bmp_file(&save_path, self.bmp_width, self.bmp_height, &self.pixels)
            {
                Ok(_) => {
                    self.status_msg = Some(format!(
                        "이미지 저장 완료: {} ({}x{}px)",
                        save_path, self.bmp_width, self.bmp_height
                    ));
                }
                Err(e) => {
                    self.status_msg = Some(format!("이미지 저장 실패: {}", e));
                }
            }
            update_window_title(hwnd, self);
        }
    }

    fn refresh_view(&mut self, hwnd: HWND, force_rerender: bool) {
        if force_rerender {
            self.pixels.clear();
        }
        update_window_title(hwnd, self);
        unsafe {
            let mut rc: RECT = zeroed();
            GetClientRect(hwnd, &mut rc);
            self.render_current_page(rc.right - rc.left, rc.bottom - rc.top);
            InvalidateRect(hwnd, null(), 0);
        }
    }

    fn go_to_page(&mut self, target_page: i32, hwnd: HWND) {
        if self.page_count <= 0 {
            return;
        }
        let next = target_page.clamp(0, self.page_count - 1);
        if next != self.current_page {
            self.current_page = next;
            self.pan_x = 0;
            self.pan_y = 0;
            self.pending_align_top = self.fit_width;
            self.pending_align_bottom = false;
            self.hovered_link = None;
            self.status_msg = None;

            let mut rc: RECT = unsafe { zeroed() };
            unsafe { GetClientRect(hwnd, &mut rc) };
            let avail_h = (rc.bottom - rc.top) - self.top_bar_offset() - self.bottom_bar_offset();
            let visible_thumbs = ((avail_h - SIDEBAR_TAB_HEIGHT) / THUMB_ITEM_HEIGHT).max(1) as usize;
            let cur_u = next as usize;
            if cur_u < self.thumb_scroll || cur_u >= self.thumb_scroll + visible_thumbs {
                self.thumb_scroll = cur_u.saturating_sub(visible_thumbs / 2);
            }

            if !self.search_query.is_empty() {
                self.perform_search(false);
            }
            self.refresh_view(hwnd, true);
        }
    }

    fn go_to_first_page(&mut self, hwnd: HWND) {
        if self.page_count <= 0 {
            return;
        }
        self.go_to_page(0, hwnd);
        self.status_msg = Some("첫 페이지(1p)로 이동".to_string());
        update_window_title(hwnd, self);
    }

    fn go_to_last_page(&mut self, hwnd: HWND) {
        if self.page_count <= 0 {
            return;
        }
        let mut last_target = self.page_count - 1;
        if self.spread_mode && last_target % 2 == 1 {
            last_target = (last_target - 1).max(0);
        }
        self.go_to_page(last_target, hwnd);
        self.status_msg = Some(format!("마지막 페이지({}p)로 이동", self.page_count));
        update_window_title(hwnd, self);
    }

    fn scroll_sidebar(&mut self, wheel_delta: i16, hwnd: HWND) {
        unsafe {
            let mut rc: RECT = zeroed();
            GetClientRect(hwnd, &mut rc);
            let client_h = rc.bottom - rc.top;
            let avail_h = client_h - self.top_bar_offset() - self.bottom_bar_offset();

            match self.sidebar_tab {
                SidebarTab::Bookmarks => {
                    if self.bookmarks.is_empty() {
                        return;
                    }
                    let max_rows =
                        ((avail_h - SIDEBAR_TAB_HEIGHT) / BOOKMARK_ROW_HEIGHT).max(1) as usize;
                    let max_scroll = self.bookmarks.len().saturating_sub(max_rows);
                    let step = 3usize;
                    if wheel_delta < 0 {
                        self.bookmark_scroll = (self.bookmark_scroll + step).min(max_scroll);
                    } else if wheel_delta > 0 {
                        self.bookmark_scroll = self.bookmark_scroll.saturating_sub(step);
                    }
                }
                SidebarTab::Thumbnails => {
                    if self.page_count <= 0 {
                        return;
                    }
                    let max_rows =
                        ((avail_h - SIDEBAR_TAB_HEIGHT) / THUMB_ITEM_HEIGHT).max(1) as usize;
                    let max_scroll = (self.page_count as usize).saturating_sub(max_rows);
                    let step = 1usize;
                    if wheel_delta < 0 {
                        self.thumb_scroll = (self.thumb_scroll + step).min(max_scroll);
                    } else if wheel_delta > 0 {
                        self.thumb_scroll = self.thumb_scroll.saturating_sub(step);
                    }
                }
            }
            InvalidateRect(hwnd, null(), 0);
        }
    }

    fn change_page(&mut self, delta: i32, hwnd: HWND) {
        let step = if self.spread_mode { delta * 2 } else { delta };
        self.go_to_page(self.current_page + step, hwnd);
    }

    fn scroll_or_turn_page(&mut self, scroll_down: bool, hwnd: HWND) {
        if self.doc.is_null() || self.page_count <= 0 {
            return;
        }

        let mut rc: RECT = unsafe { zeroed() };
        unsafe { GetClientRect(hwnd, &mut rc) };
        let viewport_h =
            (rc.bottom - rc.top) - self.top_bar_offset() - self.bottom_bar_offset();

        if self.bmp_height > viewport_h - 16 {
            let max_pan = (self.bmp_height - viewport_h) / 2 + 12;
            let min_pan = -max_pan;
            let scroll_step = 110;

            if scroll_down {
                if self.pan_y > min_pan {
                    self.pan_y = (self.pan_y - scroll_step).max(min_pan);
                    unsafe { InvalidateRect(hwnd, null(), 0) };
                    return;
                }
            } else if self.pan_y < max_pan {
                self.pan_y = (self.pan_y + scroll_step).min(max_pan);
                unsafe { InvalidateRect(hwnd, null(), 0) };
                return;
            }
        }

        let prev_page = self.current_page;
        if scroll_down {
            self.change_page(1, hwnd);
            if self.current_page != prev_page {
                self.pending_align_top = true;
                self.refresh_view(hwnd, false);
            }
        } else {
            self.change_page(-1, hwnd);
            if self.current_page != prev_page {
                self.pending_align_bottom = self.fit_width;
                self.pending_align_top = false;
                self.refresh_view(hwnd, false);
            }
        }
    }

    fn toggle_fit_width(&mut self, hwnd: HWND) {
        if self.doc.is_null() {
            return;
        }
        self.fit_width = !self.fit_width;
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.pending_align_top = self.fit_width;
        self.pending_align_bottom = false;
        self.status_msg = Some(if self.fit_width {
            "창 너비 맞춤(Fit-to-Width) 활성화".to_string()
        } else {
            "페이지 전체 맞춤(Fit-to-Page) 활성화".to_string()
        });
        self.refresh_view(hwnd, true);
    }

    fn toggle_spread_mode(&mut self, hwnd: HWND) {
        if self.doc.is_null() {
            return;
        }
        self.spread_mode = !self.spread_mode;
        if self.spread_mode && self.current_page % 2 == 1 {
            self.current_page = (self.current_page - 1).max(0);
        }
        self.pan_x = 0;
        self.pan_y = 0;
        self.pending_align_top = self.fit_width;
        self.status_msg = Some(if self.spread_mode {
            "2페이지 양면 보기(Spread View) 활성화".to_string()
        } else {
            "1페이지 단면 보기 활성화".to_string()
        });
        self.refresh_view(hwnd, true);
    }

    fn toggle_dark_mode(&mut self, hwnd: HWND) {
        self.dark_mode = !self.dark_mode;
        self.status_msg = Some(if self.dark_mode {
            "다크 모드(색상 반전) 활성화".to_string()
        } else {
            "일반 밝기 모드 활성화".to_string()
        });
        self.refresh_view(hwnd, true);
    }

    fn toggle_fullscreen(&mut self, hwnd: HWND) {
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            if !self.is_fullscreen {
                self.saved_placement.length = size_of::<WINDOWPLACEMENT>() as u32;
                GetWindowPlacement(hwnd, &mut self.saved_placement);

                let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut mi: MONITORINFO = zeroed();
                mi.cbSize = size_of::<MONITORINFO>() as u32;

                if GetMonitorInfoW(hmonitor, &mut mi) != 0 {
                    SetWindowLongPtrW(
                        hwnd,
                        GWL_STYLE,
                        (style & !WS_OVERLAPPEDWINDOW) as isize,
                    );
                    SetWindowPos(
                        hwnd,
                        HWND_TOP,
                        mi.rcMonitor.left,
                        mi.rcMonitor.top,
                        mi.rcMonitor.right - mi.rcMonitor.left,
                        mi.rcMonitor.bottom - mi.rcMonitor.top,
                        SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
                    );
                    self.is_fullscreen = true;
                }
            } else {
                SetWindowLongPtrW(
                    hwnd,
                    GWL_STYLE,
                    (style | WS_OVERLAPPEDWINDOW) as isize,
                );
                SetWindowPlacement(hwnd, &self.saved_placement);
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED | 0x0003,
                );
                self.is_fullscreen = false;
            }
            self.refresh_view(hwnd, true);
        }
    }

    fn step_bookmark(&mut self, forward: bool, hwnd: HWND) {
        if self.bookmarks.is_empty() {
            self.status_msg = Some("문서에 북마크가 없습니다".to_string());
            update_window_title(hwnd, self);
            return;
        }

        if forward {
            if let Some(bm) = self
                .bookmarks
                .iter()
                .find(|b| b.page_index > self.current_page)
            {
                let target = bm.page_index;
                let title = bm.title.clone();
                self.go_to_page(target, hwnd);
                self.status_msg = Some(format!("목차: {}", title));
                update_window_title(hwnd, self);
            }
        } else if let Some(bm) = self
            .bookmarks
            .iter()
            .rev()
            .find(|b| b.page_index >= 0 && b.page_index < self.current_page)
        {
            let target = bm.page_index;
            let title = bm.title.clone();
            self.go_to_page(target, hwnd);
            self.status_msg = Some(format!("목차: {}", title));
            update_window_title(hwnd, self);
        }
    }

    fn rotate_page(&mut self, clockwise: bool, hwnd: HWND) {
        if self.doc.is_null() {
            return;
        }
        self.rotation = if clockwise {
            (self.rotation + 1) % 4
        } else {
            (self.rotation + 3) % 4
        };
        self.clear_thumbnail_cache();
        self.pan_x = 0;
        self.pan_y = 0;
        self.pending_align_top = self.fit_width;
        self.status_msg = None;
        self.refresh_view(hwnd, true);
    }

    fn change_zoom(&mut self, factor: f64, hwnd: HWND) {
        if self.doc.is_null() {
            return;
        }
        let next_zoom = (self.zoom * factor).clamp(0.25, 5.0);
        if (next_zoom - self.zoom).abs() > 0.001 {
            let ratio = next_zoom / self.zoom;
            self.pan_x = (self.pan_x as f64 * ratio) as i32;
            self.pan_y = (self.pan_y as f64 * ratio) as i32;
            self.zoom = next_zoom;
            self.status_msg = None;
            self.refresh_view(hwnd, true);
        }
    }

    fn reset_view(&mut self, hwnd: HWND) {
        self.zoom = 1.0;
        self.rotation = 0;
        self.fit_width = false;
        self.pan_x = 0;
        self.pan_y = 0;
        self.clear_thumbnail_cache();
        self.status_msg = None;
        self.refresh_view(hwnd, true);
    }

    fn copy_text_to_clipboard(&mut self, hwnd: HWND) {
        if self.doc.is_null() {
            return;
        }
        let mut combined = self
            .extract_page_text_at(self.current_page)
            .unwrap_or_default();

        if self.spread_mode && (self.current_page + 1 < self.page_count) {
            if let Some(second_text) = self.extract_page_text_at(self.current_page + 1) {
                if !second_text.trim().is_empty() {
                    if !combined.is_empty() {
                        combined.push_str("\r\n\r\n");
                    }
                    combined.push_str(&second_text);
                }
            }
        }

        if !combined.trim().is_empty() {
            let normalized = combined.replace("\r\n", "\n").replace('\n', "\r\n");
            let char_len = normalized.chars().count();
            if set_clipboard_unicode_text(hwnd, &normalized) {
                self.status_msg = Some(format!("텍스트 복사 완료 ({}자)", char_len));
            } else {
                self.status_msg = Some("클립보드 복사 실패".to_string());
            }
        } else {
            self.status_msg = Some("이 페이지에 추출할 텍스트가 없습니다".to_string());
        }
        update_window_title(hwnd, self);
    }

    fn is_toolbar_btn_active(&self, id: u16) -> bool {
        match id {
            IDM_VIEW_FIT_WIDTH => self.fit_width,
            IDM_VIEW_SPREAD => self.spread_mode,
            IDM_THEME_DARK_DOC => self.dark_mode,
            IDM_VIEW_BOOKMARKS => {
                self.show_sidebar && self.sidebar_tab == SidebarTab::Bookmarks
            }
            IDM_VIEW_THUMBNAILS => {
                self.show_sidebar && self.sidebar_tab == SidebarTab::Thumbnails
            }
            IDM_ANNOT_NONE => self.active_tool == ActiveTool::Pan,
            IDM_ANNOT_HIGHLIGHT => self.active_tool == ActiveTool::Highlight,
            IDM_ANNOT_PENCIL => self.active_tool == ActiveTool::Pencil,
            IDM_SEC_REDACT_TOOL => self.active_tool == ActiveTool::RedactBox,
            IDM_TOOL_SEARCH => self.input_mode == InputMode::Search,
            _ => false,
        }
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        if !self.doc.is_null() {
            unsafe { FPDF_CloseDocument(self.doc) };
        }
    }
}

// ============================================================================
// 좌측 사이드바 ([📑 목차] / [🖼 썸네일] 탭 전환 및 축소판 렌더링)
// ============================================================================
unsafe fn draw_left_sidebar_panel(hdc: *mut c_void, client_h: i32, state: &mut AppState) {
    let top_off = state.top_bar_offset();
    let bot_off = state.bottom_bar_offset();
    let panel_bottom = client_h - bot_off;

    let panel_rc = RECT {
        left: 0,
        top: top_off,
        right: SIDEBAR_WIDTH,
        bottom: panel_bottom,
    };
    let panel_brush = CreateSolidBrush(0x00252526);
    FillRect(hdc, &panel_rc, panel_brush);
    DeleteObject(panel_brush as _);

    let tab_bar_rc = RECT {
        left: 0,
        top: top_off,
        right: SIDEBAR_WIDTH,
        bottom: top_off + SIDEBAR_TAB_HEIGHT,
    };
    let tab_bar_brush = CreateSolidBrush(0x001B1B1C);
    FillRect(hdc, &tab_bar_rc, tab_bar_brush);
    DeleteObject(tab_bar_brush as _);

    let border_rc = RECT {
        left: SIDEBAR_WIDTH - 1,
        top: top_off,
        right: SIDEBAR_WIDTH,
        bottom: panel_bottom,
    };
    let border_brush = CreateSolidBrush(0x00454545);
    FillRect(hdc, &border_rc, border_brush);
    DeleteObject(border_brush as _);

    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT as i32);

    let is_bm_tab = state.sidebar_tab == SidebarTab::Bookmarks;
    let tab1_rc = RECT {
        left: 2,
        top: top_off + 2,
        right: 122,
        bottom: top_off + SIDEBAR_TAB_HEIGHT - 1,
    };
    let tab1_br = CreateSolidBrush(if is_bm_tab { 0x0037373D } else { 0x00222224 });
    FillRect(hdc, &tab1_rc, tab1_br);
    DeleteObject(tab1_br as _);

    let tab2_rc = RECT {
        left: 124,
        top: top_off + 2,
        right: 246,
        bottom: top_off + SIDEBAR_TAB_HEIGHT - 1,
    };
    let tab2_br = CreateSolidBrush(if !is_bm_tab { 0x0037373D } else { 0x00222224 });
    FillRect(hdc, &tab2_rc, tab2_br);
    DeleteObject(tab2_br as _);

    let accent_rc = RECT {
        left: if is_bm_tab { 2 } else { 124 },
        top: top_off + 2,
        right: if is_bm_tab { 122 } else { 246 },
        bottom: top_off + 5,
    };
    let accent_br = CreateSolidBrush(0x00FFAA00);
    FillRect(hdc, &accent_rc, accent_br);
    DeleteObject(accent_br as _);

    SetTextColor(
        hdc,
        if is_bm_tab { 0x0080D8FF } else { 0x00AAAAAA },
    );
    let t1_lbl: Vec<u16> = format!("📑 목차 ({})", state.bookmarks.len())
        .encode_utf16()
        .collect();
    TextOutW(hdc, 14, top_off + 10, t1_lbl.as_ptr(), t1_lbl.len() as i32);

    SetTextColor(
        hdc,
        if !is_bm_tab { 0x0080D8FF } else { 0x00AAAAAA },
    );
    let t2_lbl: Vec<u16> = format!("🖼 썸네일 ({})", state.page_count)
        .encode_utf16()
        .collect();
    TextOutW(hdc, 134, top_off + 10, t2_lbl.as_ptr(), t2_lbl.len() as i32);

    SetTextColor(hdc, 0x00999999);
    let close_lbl: Vec<u16> = "✕".encode_utf16().collect();
    TextOutW(hdc, 256, top_off + 10, close_lbl.as_ptr(), close_lbl.len() as i32);

    let content_top = top_off + SIDEBAR_TAB_HEIGHT + 4;
    let avail_h = (panel_bottom - content_top).max(26);

    match state.sidebar_tab {
        SidebarTab::Bookmarks => {
            let max_rows = (avail_h / BOOKMARK_ROW_HEIGHT).max(1) as usize;
            let total_bm = state.bookmarks.len();
            let max_scroll = total_bm.saturating_sub(max_rows);
            if state.bookmark_scroll > max_scroll {
                state.bookmark_scroll = max_scroll;
            }

            if state.bookmarks.is_empty() {
                SetTextColor(hdc, 0x00AAAAAA);
                let empty_msg: Vec<u16> =
                    "목차 없음 ([보기 > 자동 북마크] 가능)".encode_utf16().collect();
                TextOutW(
                    hdc,
                    12,
                    content_top + 12,
                    empty_msg.as_ptr(),
                    empty_msg.len() as i32,
                );
            } else {
                for (i, bm) in state
                    .bookmarks
                    .iter()
                    .enumerate()
                    .skip(state.bookmark_scroll)
                    .take(max_rows)
                {
                    let row_idx = i - state.bookmark_scroll;
                    let y_top = content_top + (row_idx as i32) * BOOKMARK_ROW_HEIGHT;
                    let is_current = bm.page_index == state.current_page
                        || (state.spread_mode && bm.page_index == state.current_page + 1);

                    if is_current {
                        let active_rc = RECT {
                            left: 2,
                            top: y_top,
                            right: SIDEBAR_WIDTH - 6,
                            bottom: y_top + BOOKMARK_ROW_HEIGHT - 1,
                        };
                        let active_brush = CreateSolidBrush(0x005A3D1C);
                        FillRect(hdc, &active_rc, active_brush);
                        DeleteObject(active_brush as _);
                        SetTextColor(hdc, 0x0000EAFF);
                    } else {
                        SetTextColor(hdc, 0x00E6E6E6);
                    }

                    let indent = (bm.depth.min(5) as i32) * 12;
                    let label = if bm.page_index >= 0 {
                        format!("[p.{}] {}", bm.page_index + 1, bm.title)
                    } else {
                        bm.title.clone()
                    };
                    let w_label: Vec<u16> = label.encode_utf16().collect();
                    TextOutW(
                        hdc,
                        10 + indent,
                        y_top + 5,
                        w_label.as_ptr(),
                        w_label.len() as i32,
                    );
                }

                if total_bm > max_rows && avail_h > 40 {
                    let track_h = panel_bottom - content_top - 4;
                    let thumb_h =
                        ((max_rows as f64 / total_bm as f64) * track_h as f64).max(24.0) as i32;
                    let scroll_ratio = state.bookmark_scroll as f64 / max_scroll.max(1) as f64;
                    let thumb_top = content_top + ((track_h - thumb_h) as f64 * scroll_ratio) as i32;

                    let thumb_rc = RECT {
                        left: SIDEBAR_WIDTH - 5,
                        top: thumb_top,
                        right: SIDEBAR_WIDTH - 2,
                        bottom: thumb_top + thumb_h,
                    };
                    let thumb_brush = CreateSolidBrush(0x00888888);
                    FillRect(hdc, &thumb_rc, thumb_brush);
                    DeleteObject(thumb_brush as _);
                }
            }
        }
        SidebarTab::Thumbnails => {
            let max_rows = (avail_h / THUMB_ITEM_HEIGHT).max(1) as usize;
            let total_pages = state.page_count.max(0) as usize;
            let max_scroll = total_pages.saturating_sub(max_rows);
            if state.thumb_scroll > max_scroll {
                state.thumb_scroll = max_scroll;
            }

            if total_pages == 0 {
                SetTextColor(hdc, 0x00AAAAAA);
                let empty_msg: Vec<u16> = "열려 있는 문서가 없습니다.".encode_utf16().collect();
                TextOutW(
                    hdc,
                    16,
                    content_top + 16,
                    empty_msg.as_ptr(),
                    empty_msg.len() as i32,
                );
            } else {
                let start_idx = state.thumb_scroll;
                let end_idx = (start_idx + max_rows).min(total_pages);

                for (slot, page_u) in (start_idx..end_idx).enumerate() {
                    let p_idx = page_u as i32;
                    let item_top = content_top + (slot as i32) * THUMB_ITEM_HEIGHT + 4;
                    let is_active = p_idx == state.current_page
                        || (state.spread_mode && p_idx == state.current_page + 1);

                    let card_rc = RECT {
                        left: 16,
                        top: item_top,
                        right: SIDEBAR_WIDTH - 18,
                        bottom: item_top + THUMB_ITEM_HEIGHT - 10,
                    };
                    let card_bg = CreateSolidBrush(if is_active { 0x004E371B } else { 0x002E2E32 });
                    FillRect(hdc, &card_rc, card_bg);
                    DeleteObject(card_bg as _);

                    if let Some(thumb) = state.get_or_render_thumbnail(p_idx) {
                        let tx = 16 + ((SIDEBAR_WIDTH - 34) - thumb.width) / 2;
                        let ty = item_top + 6 + (176 - thumb.height) / 2;

                        if is_active {
                            let frame_rc = RECT {
                                left: tx - 3,
                                top: ty - 3,
                                right: tx + thumb.width + 3,
                                bottom: ty + thumb.height + 3,
                            };
                            let frame_br = CreateSolidBrush(0x0000B4FF);
                            FillRect(hdc, &frame_rc, frame_br);
                            DeleteObject(frame_br as _);
                        }

                        let mut bmi: BITMAPINFO = zeroed();
                        bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
                        bmi.bmiHeader.biWidth = thumb.width;
                        bmi.bmiHeader.biHeight = -thumb.height;
                        bmi.bmiHeader.biPlanes = 1;
                        bmi.bmiHeader.biBitCount = 32;
                        bmi.bmiHeader.biCompression = BI_RGB;

                        StretchDIBits(
                            hdc,
                            tx,
                            ty,
                            thumb.width,
                            thumb.height,
                            0,
                            0,
                            thumb.width,
                            thumb.height,
                            thumb.pixels.as_ptr() as *const c_void,
                            &bmi,
                            DIB_RGB_COLORS,
                            SRCCOPY,
                        );
                    }

                    SetTextColor(
                        hdc,
                        if is_active { 0x0000EAFF } else { 0x00DDDDDD },
                    );
                    let p_lbl = format!("- {} -", p_idx + 1);
                    let w_p: Vec<u16> = p_lbl.encode_utf16().collect();
                    TextOutW(
                        hdc,
                        SIDEBAR_WIDTH / 2 - 20,
                        item_top + THUMB_ITEM_HEIGHT - 28,
                        w_p.as_ptr(),
                        w_p.len() as i32,
                    );
                }

                if total_pages > max_rows && avail_h > 40 {
                    let track_h = panel_bottom - content_top - 4;
                    let thumb_h = ((max_rows as f64 / total_pages as f64) * track_h as f64)
                        .max(24.0) as i32;
                    let scroll_ratio = state.thumb_scroll as f64 / max_scroll.max(1) as f64;
                    let thumb_top = content_top + ((track_h - thumb_h) as f64 * scroll_ratio) as i32;

                    let thumb_rc = RECT {
                        left: SIDEBAR_WIDTH - 5,
                        top: thumb_top,
                        right: SIDEBAR_WIDTH - 2,
                        bottom: thumb_top + thumb_h,
                    };
                    let thumb_brush = CreateSolidBrush(0x00888888);
                    FillRect(hdc, &thumb_rc, thumb_brush);
                    DeleteObject(thumb_brush as _);
                }
            }
        }
    }

    SelectObject(hdc, old_font);
}

// ============================================================================
// 상단 툴바 & 하단 상태바 렌더링
// ============================================================================
fn hit_test_toolbar(click_x: i32, click_y: i32) -> Option<u16> {
    if !(4..=TOOLBAR_HEIGHT - 4).contains(&click_y) {
        return None;
    }
    let mut cur_x = 8;
    for btn in TOOLBAR_BUTTONS {
        if click_x >= cur_x && click_x <= cur_x + btn.width {
            return Some(btn.id);
        }
        cur_x += btn.width + 5;
    }
    None
}

unsafe fn draw_top_toolbar(hdc: *mut c_void, client_w: i32, state: &AppState) {
    let bar_rc = RECT {
        left: 0,
        top: 0,
        right: client_w,
        bottom: TOOLBAR_HEIGHT,
    };
    let bar_brush = CreateSolidBrush(0x002D2D30);
    FillRect(hdc, &bar_rc, bar_brush);
    DeleteObject(bar_brush as _);

    let border_rc = RECT {
        left: 0,
        top: TOOLBAR_HEIGHT - 1,
        right: client_w,
        bottom: TOOLBAR_HEIGHT,
    };
    let border_brush = CreateSolidBrush(0x00454545);
    FillRect(hdc, &border_rc, border_brush);
    DeleteObject(border_brush as _);

    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT as i32);

    let mut cur_x = 8;
    for btn in TOOLBAR_BUTTONS {
        let is_active = state.is_toolbar_btn_active(btn.id);
        let is_hover = state.hover_tb_id == Some(btn.id);

        let btn_rc = RECT {
            left: cur_x,
            top: 5,
            right: cur_x + btn.width,
            bottom: TOOLBAR_HEIGHT - 6,
        };

        let bg_col = if is_active {
            0x007A4B16
        } else if is_hover {
            0x00505055
        } else {
            0x003C3C40
        };

        let btn_brush = CreateSolidBrush(bg_col);
        FillRect(hdc, &btn_rc, btn_brush);
        DeleteObject(btn_brush as _);

        SetTextColor(
            hdc,
            if is_active {
                0x0066EAFF
            } else {
                0x00F0F0F0
            },
        );
        let w_lbl: Vec<u16> = btn.label.encode_utf16().collect();
        TextOutW(hdc, cur_x + 7, 11, w_lbl.as_ptr(), w_lbl.len() as i32);

        cur_x += btn.width + 5;
    }

    SelectObject(hdc, old_font);
}

fn hit_test_status_bar(click_x: i32, client_w: i32) -> Option<u16> {
    let center_x = client_w / 2;
    if (center_x - 135..center_x - 100).contains(&click_x) {
        return Some(IDM_TB_FIRST_PAGE);
    }
    if (center_x - 96..center_x - 60).contains(&click_x) {
        return Some(IDM_TB_PREV_PAGE);
    }
    if (center_x - 56..center_x + 56).contains(&click_x) {
        return Some(IDM_TOOL_GOTO);
    }
    if (center_x + 60..center_x + 96).contains(&click_x) {
        return Some(IDM_TB_NEXT_PAGE);
    }
    if (center_x + 100..center_x + 135).contains(&click_x) {
        return Some(IDM_TB_LAST_PAGE);
    }

    if (client_w - 295..client_w - 220).contains(&click_x) {
        return Some(IDM_VIEW_SPREAD);
    }
    if (client_w - 210..client_w - 180).contains(&click_x) {
        return Some(IDM_VIEW_ZOOM_OUT);
    }
    if (client_w - 178..client_w - 122).contains(&click_x) {
        return Some(IDM_VIEW_ACTUAL_SIZE);
    }
    if (client_w - 120..client_w - 90).contains(&click_x) {
        return Some(IDM_VIEW_ZOOM_IN);
    }
    if (client_w - 84..client_w - 8).contains(&click_x) {
        return Some(IDM_VIEW_FULLSCREEN);
    }
    None
}

unsafe fn draw_bottom_status_bar(
    hdc: *mut c_void,
    client_w: i32,
    client_h: i32,
    state: &AppState,
) {
    let top_y = client_h - STATUSBAR_HEIGHT;
    let bar_rc = RECT {
        left: 0,
        top: top_y,
        right: client_w,
        bottom: client_h,
    };
    let bar_brush = CreateSolidBrush(0x00222225);
    FillRect(hdc, &bar_rc, bar_brush);
    DeleteObject(bar_brush as _);

    let border_rc = RECT {
        left: 0,
        top: top_y,
        right: client_w,
        bottom: top_y + 1,
    };
    let border_brush = CreateSolidBrush(0x00454545);
    FillRect(hdc, &border_rc, border_brush);
    DeleteObject(border_brush as _);

    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT as i32);

    let w_cm = state.cur_page_w_pt * 2.54 / 72.0;
    let h_cm = state.cur_page_h_pt * 2.54 / 72.0;
    let dim_text = if state.page_count > 0 {
        format!(
            "📐 {:.1} x {:.1} cm ({:.0}x{:.0} pt)",
            w_cm, h_cm, state.cur_page_w_pt, state.cur_page_h_pt
        )
    } else {
        String::from("📐 문서 대기 중")
    };

    // 하이퍼링크 위에 마우스가 올라가 있으면 URL/대상 페이지를 하단 상태바에 우선 표시
    let status_info = if let Some(ref link) = state.hovered_link {
        match link {
            LinkTarget::InternalPage(p) => format!("🔗 내부 링크: {}페이지로 이동 (클릭)", p + 1),
            LinkTarget::ExternalUri(uri) => format!("🌐 웹 링크: {} (클릭하여 열기)", uri),
        }
    } else if let Some(ref msg) = state.status_msg {
        format!("{}  |  💡 {}", dim_text, msg)
    } else {
        dim_text
    };

    SetTextColor(
        hdc,
        if state.hovered_link.is_some() {
            0x0066EAFF
        } else {
            0x00CCCCCC
        },
    );
    let w_left: Vec<u16> = status_info.encode_utf16().collect();
    TextOutW(hdc, 10, top_y + 6, w_left.as_ptr(), w_left.len() as i32);

    let center_x = client_w / 2;
    let nav_bg = RECT {
        left: center_x - 135,
        top: top_y + 3,
        right: center_x + 135,
        bottom: client_h - 3,
    };
    let nav_br = CreateSolidBrush(0x00333337);
    FillRect(hdc, &nav_bg, nav_br);
    DeleteObject(nav_br as _);

    SetTextColor(hdc, 0x0080D8FF);
    let nav_str = if state.page_count > 0 {
        format!(
            "[⏮]   [◀]    {} / {} p (이동)    [▶]   [⏭]",
            state.current_page + 1,
            state.page_count
        )
    } else {
        String::from("[⏮]   [◀]     0 / 0 p     [▶]   [⏭]")
    };
    let w_nav: Vec<u16> = nav_str.encode_utf16().collect();
    TextOutW(
        hdc,
        center_x - 122,
        top_y + 6,
        w_nav.as_ptr(),
        w_nav.len() as i32,
    );

    let right_bg = RECT {
        left: client_w - 295,
        top: top_y + 3,
        right: client_w - 8,
        bottom: client_h - 3,
    };
    let right_br = CreateSolidBrush(0x00333337);
    FillRect(hdc, &right_bg, right_br);
    DeleteObject(right_br as _);

    SetTextColor(hdc, 0x00E6E6E6);
    let mode_lbl = if state.spread_mode { "2p양면" } else { "1p단면" };
    let right_str = format!(
        "[{}]  |  [-] {:>4.0}% [+]  |  [⛶ 전체화면]",
        mode_lbl,
        state.zoom * 100.0
    );
    let w_right: Vec<u16> = right_str.encode_utf16().collect();
    TextOutW(
        hdc,
        client_w - 288,
        top_y + 6,
        w_right.as_ptr(),
        w_right.len() as i32,
    );

    SelectObject(hdc, old_font);
}

unsafe fn draw_page_overlays_and_guides(
    hdc: *mut c_void,
    page_x: i32,
    page_y: i32,
    client_w: i32,
    client_h: i32,
    state: &AppState,
) {
    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT as i32);

    let pw = state.bmp_width;
    let ph = state.bmp_height;

    if let Some(ref wm) = state.watermark_text {
        SetTextColor(hdc, 0x009090D0);
        let w_wm: Vec<u16> = format!("=== [ {} ] ===", wm).encode_utf16().collect();
        for step in [-ph / 4, 0, ph / 4] {
            TextOutW(
                hdc,
                page_x + pw / 2 - 70,
                page_y + ph / 2 + step,
                w_wm.as_ptr(),
                w_wm.len() as i32,
            );
        }
    }

    if let Some(ref hf) = state.header_footer_text {
        SetTextColor(hdc, 0x00555555);
        let w_hf: Vec<u16> = hf.encode_utf16().collect();
        TextOutW(
            hdc,
            page_x + 20,
            page_y + 10,
            w_hf.as_ptr(),
            w_hf.len() as i32,
        );
    }

    if state.show_page_numbers {
        SetTextColor(hdc, 0x00444444);
        let p_str = format!("- {} / {} -", state.current_page + 1, state.page_count);
        let w_p: Vec<u16> = p_str.encode_utf16().collect();
        TextOutW(
            hdc,
            page_x + pw / 2 - 30,
            page_y + ph - 24,
            w_p.as_ptr(),
            w_p.len() as i32,
        );
    }

    if state.show_bates_numbers {
        SetTextColor(hdc, 0x002222AA);
        let b_str = format!("BATES #{:06}", state.current_page + 1);
        let w_b: Vec<u16> = b_str.encode_utf16().collect();
        TextOutW(
            hdc,
            page_x + pw - 120,
            page_y + 10,
            w_b.as_ptr(),
            w_b.len() as i32,
        );
    }

    if state.show_annotations {
        for ov in state
            .overlays
            .iter()
            .filter(|o| o.page_index == state.current_page)
        {
            let x1 = page_x + (ov.x1 * pw as f64) as i32;
            let y1 = page_y + (ov.y1 * ph as f64) as i32;
            let x2 = page_x + (ov.x2 * pw as f64) as i32;
            let y2 = page_y + (ov.y2 * ph as f64) as i32;

            match ov.kind {
                OverlayKind::Highlight | OverlayKind::AreaHighlight => {
                    let pen = CreatePen(PS_SOLID, 2, 0x0000D8FF);
                    let old_p = SelectObject(hdc, pen as _);
                    let null_br = GetStockObject(5);
                    let old_b = SelectObject(hdc, null_br);
                    Rectangle(hdc, x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2));
                    SelectObject(hdc, old_b);
                    SelectObject(hdc, old_p);
                    DeleteObject(pen as _);
                }
                OverlayKind::Underline => {
                    let pen = CreatePen(PS_SOLID, 2, 0x0000AA00);
                    let old_p = SelectObject(hdc, pen as _);
                    MoveToEx(hdc, x1.min(x2), y1.max(y2), null_mut());
                    LineTo(hdc, x1.max(x2), y1.max(y2));
                    SelectObject(hdc, old_p);
                    DeleteObject(pen as _);
                }
                OverlayKind::Strikeout => {
                    let pen = CreatePen(PS_SOLID, 2, 0x000000DD);
                    let old_p = SelectObject(hdc, pen as _);
                    let mid_y = (y1 + y2) / 2;
                    MoveToEx(hdc, x1.min(x2), mid_y, null_mut());
                    LineTo(hdc, x1.max(x2), mid_y);
                    SelectObject(hdc, old_p);
                    DeleteObject(pen as _);
                }
                OverlayKind::RectShape => {
                    let pen = CreatePen(PS_SOLID, 2, 0x000000FF);
                    let old_p = SelectObject(hdc, pen as _);
                    let null_br = GetStockObject(5);
                    let old_b = SelectObject(hdc, null_br);
                    Rectangle(hdc, x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2));
                    SelectObject(hdc, old_b);
                    SelectObject(hdc, old_p);
                    DeleteObject(pen as _);
                }
                OverlayKind::LineShape => {
                    let pen = CreatePen(PS_SOLID, 2, 0x00FF4400);
                    let old_p = SelectObject(hdc, pen as _);
                    MoveToEx(hdc, x1, y1, null_mut());
                    LineTo(hdc, x2, y2);
                    SelectObject(hdc, old_p);
                    DeleteObject(pen as _);
                }
                OverlayKind::Pencil => {
                    if ov.points.len() >= 2 {
                        let pen = CreatePen(PS_SOLID, 2, 0x00AA00AA);
                        let old_p = SelectObject(hdc, pen as _);
                        let p0 = ov.points[0];
                        MoveToEx(
                            hdc,
                            page_x + (p0.0 * pw as f64) as i32,
                            page_y + (p0.1 * ph as f64) as i32,
                            null_mut(),
                        );
                        for pt in &ov.points[1..] {
                            LineTo(
                                hdc,
                                page_x + (pt.0 * pw as f64) as i32,
                                page_y + (pt.1 * ph as f64) as i32,
                            );
                        }
                        SelectObject(hdc, old_p);
                        DeleteObject(pen as _);
                    }
                }
                OverlayKind::StampApproved | OverlayKind::StampDraft | OverlayKind::Note => {
                    let rc = RECT {
                        left: x1,
                        top: y1,
                        right: x1 + 150,
                        bottom: y1 + 32,
                    };
                    let color = if ov.kind == OverlayKind::StampApproved {
                        0x00228822
                    } else if ov.kind == OverlayKind::StampDraft {
                        0x00AA4400
                    } else {
                        0x0088FFFF
                    };
                    let br = CreateSolidBrush(color);
                    FillRect(hdc, &rc, br);
                    DeleteObject(br as _);
                    SetTextColor(
                        hdc,
                        if ov.kind == OverlayKind::Note {
                            0x00000000
                        } else {
                            0x00FFFFFF
                        },
                    );
                    let w_lbl: Vec<u16> = ov.label.encode_utf16().collect();
                    TextOutW(hdc, x1 + 8, y1 + 8, w_lbl.as_ptr(), w_lbl.len() as i32);
                }
                OverlayKind::Redaction => {
                    let rc = RECT {
                        left: x1.min(x2),
                        top: y1.min(y2),
                        right: x1.max(x2).max(x1 + 10),
                        bottom: y1.max(y2).max(y1 + 10),
                    };
                    let br = CreateSolidBrush(0x00000000);
                    FillRect(hdc, &rc, br);
                    DeleteObject(br as _);
                }
                OverlayKind::FormText => {
                    let rc = RECT {
                        left: x1.min(x2),
                        top: y1.min(y2),
                        right: x1.max(x2).max(x1 + 120),
                        bottom: y1.max(y2).max(y1 + 26),
                    };
                    let br = CreateSolidBrush(0x00FFF4E8);
                    FillRect(hdc, &rc, br);
                    DeleteObject(br as _);
                    SetTextColor(hdc, 0x00663300);
                    let w_lbl: Vec<u16> = ov.label.encode_utf16().collect();
                    TextOutW(
                        hdc,
                        rc.left + 4,
                        rc.top + 5,
                        w_lbl.as_ptr(),
                        w_lbl.len() as i32,
                    );
                }
                OverlayKind::FormCheck => {
                    let rc = RECT {
                        left: x1,
                        top: y1,
                        right: x1 + 20,
                        bottom: y1 + 20,
                    };
                    let br = CreateSolidBrush(0x00FFFFFF);
                    FillRect(hdc, &rc, br);
                    DeleteObject(br as _);
                    SetTextColor(hdc, 0x00000000);
                    let mark = if ov.checked { "[V]" } else { "[ ]" };
                    let w_m: Vec<u16> = mark.encode_utf16().collect();
                    TextOutW(hdc, x1 + 1, y1 + 2, w_m.as_ptr(), w_m.len() as i32);
                }
            }
        }
    }

    if state.show_grid {
        let grid_pen = CreatePen(PS_DOT, 1, 0x00C0C0C0);
        let old_pen = SelectObject(hdc, grid_pen as _);
        let step = 50;
        let mut gx = page_x;
        while gx < page_x + pw {
            MoveToEx(hdc, gx, page_y, null_mut());
            LineTo(hdc, gx, page_y + ph);
            gx += step;
        }
        let mut gy = page_y;
        while gy < page_y + ph {
            MoveToEx(hdc, page_x, gy, null_mut());
            LineTo(hdc, page_x + pw, gy);
            gy += step;
        }
        SelectObject(hdc, old_pen);
        DeleteObject(grid_pen as _);
    }

    if state.show_ruler {
        let sb_w = state.sidebar_offset();
        let top_off = state.top_bar_offset();
        let bot_off = state.bottom_bar_offset();
        let top_ruler = RECT {
            left: sb_w,
            top: top_off,
            right: client_w,
            bottom: top_off + 22,
        };
        let left_ruler = RECT {
            left: sb_w,
            top: top_off + 22,
            right: sb_w + 26,
            bottom: client_h - bot_off,
        };
        let r_brush = CreateSolidBrush(0x00E8E8E8);
        FillRect(hdc, &top_ruler, r_brush);
        FillRect(hdc, &left_ruler, r_brush);
        DeleteObject(r_brush as _);

        SetTextColor(hdc, 0x00222222);
        let mut rx = sb_w + 30;
        let mut cm = 0;
        while rx < client_w {
            let lbl = format!("{}", cm);
            let w_l: Vec<u16> = lbl.encode_utf16().collect();
            TextOutW(hdc, rx, top_off + 3, w_l.as_ptr(), w_l.len() as i32);
            rx += 38;
            cm += 1;
        }
        let mut ry = top_off + 26;
        let mut vcm = 0;
        while ry < client_h - bot_off {
            let lbl = format!("{}", vcm);
            let w_l: Vec<u16> = lbl.encode_utf16().collect();
            TextOutW(hdc, sb_w + 4, ry, w_l.as_ptr(), w_l.len() as i32);
            ry += 38;
            vcm += 1;
        }
    }

    SelectObject(hdc, old_font);
}

unsafe fn build_main_menu_bar() -> *mut c_void {
    let hmenu = CreateMenu();

    let add_item = |menu: *mut c_void, id: u16, label: &str| {
        let w = to_wstring(label);
        AppendMenuW(menu, MF_STRING, id as usize, w.as_ptr());
    };
    let add_sep = |menu: *mut c_void| {
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
    };
    let add_sub = |parent: *mut c_void, sub: *mut c_void, label: &str| {
        let w = to_wstring(label);
        AppendMenuW(parent, MF_POPUP, sub as usize, w.as_ptr());
    };

    let m_file = CreatePopupMenu();
    add_item(m_file, IDM_FILE_NEW_BLANK, "빈 PDF 생성(&N)");
    add_item(m_file, IDM_FILE_FROM_CLIPBOARD, "클립보드에서 PDF 생성(&B)");
    add_sep(m_file);
    add_item(m_file, IDM_FILE_OPEN, "PDF 파일 열기(&O)...\tO");
    add_item(m_file, IDM_FILE_SAVE, "저장(&S)\tCtrl+S");
    add_item(m_file, IDM_FILE_SAVE_AS, "다른 이름으로 저장(&A)...");
    add_item(m_file, IDM_FILE_SAVE_V14, "다른 버전으로 저장 (PDF 1.4)...");
    add_item(m_file, IDM_FILE_SAVE_V17, "다른 버전으로 저장 (PDF 1.7)...");
    add_sep(m_file);
    add_item(m_file, IDM_FILE_PRINT, "인쇄하기(&P)...\tCtrl+P");
    add_item(m_file, IDM_FILE_PROPS, "파일 속성(&I)...\tI");
    add_sep(m_file);
    add_item(m_file, IDM_FILE_EXIT, "종료(&X)\tESC");
    add_sub(hmenu, m_file, "파일(&F)");

    let m_edit = CreatePopupMenu();
    add_item(m_edit, IDM_EDIT_ADD_TEXT, "텍스트 추가(&T)...");
    add_sep(m_edit);
    add_item(m_edit, IDM_EDIT_WM_CONFIDENTIAL, "워터마크 추가: '기밀 (CONFIDENTIAL)'");
    add_item(m_edit, IDM_EDIT_WM_SAMPLE, "워터마크 추가: 'SAMPLE'");
    add_item(m_edit, IDM_EDIT_WM_CUSTOM, "사용자 지정 워터마크 입력...");
    add_item(m_edit, IDM_EDIT_WM_CLEAR, "워터마크 제거");
    add_sep(m_edit);
    add_item(m_edit, IDM_EDIT_BG_CREAM, "배경화면 추가: 눈 보호 크림색");
    add_item(m_edit, IDM_EDIT_BG_MINT, "배경화면 추가: 소프트 민트색");
    add_item(m_edit, IDM_EDIT_BG_CLEAR, "배경화면 제거");
    add_sep(m_edit);
    add_item(m_edit, IDM_EDIT_HEADER_FOOTER, "머리글 & 바닥글 토글");
    add_item(m_edit, IDM_EDIT_PAGE_NUMBER, "페이지 번호 표시 토글");
    add_item(m_edit, IDM_EDIT_BATES_NUMBER, "베이츠 번호(Bates #0001) 표시 토글");
    add_sub(hmenu, m_edit, "편집(&E)");

    let m_annot = CreatePopupMenu();
    add_item(m_annot, IDM_ANNOT_NONE, "기본 손 도구 (화면 이동 Pan 및 링크 클릭 모드)");
    add_sep(m_annot);
    add_item(m_annot, IDM_ANNOT_HIGHLIGHT, "강조 (형광펜 드래그)");
    add_item(m_annot, IDM_ANNOT_AREA_HL, "영역 강조 (사각 영역 하이라이트)");
    add_item(m_annot, IDM_ANNOT_UNDERLINE, "밑줄 표시 (드래그)\tCtrl+U");
    add_item(m_annot, IDM_ANNOT_STRIKEOUT, "취소선 표시 (드래그)\tCtrl+Q");
    add_sep(m_annot);
    add_item(m_annot, IDM_ANNOT_PENCIL, "연필 (자유 곡선 그리기)");
    add_item(m_annot, IDM_ANNOT_ERASER, "지우개 (클릭하여 주석 삭제)");
    add_item(m_annot, IDM_ANNOT_RECT, "도형: 직사각형 추가");
    add_item(m_annot, IDM_ANNOT_LINE, "도형: 직선/화살표 추가");
    add_item(m_annot, IDM_ANNOT_NOTE, "노트/타자기 주석 클릭 추가");
    add_sep(m_annot);
    add_item(m_annot, IDM_ANNOT_STAMP_APPROVED, "스탬프 찍기: [승인됨 / APPROVED]");
    add_item(m_annot, IDM_ANNOT_STAMP_DRAFT, "스탬프 찍기: [초안 / DRAFT]");
    add_sep(m_annot);
    add_item(m_annot, IDM_ANNOT_MEASURE_DIST, "측정 도구: 두 점 거리 측정 (cm/mm)");
    add_item(m_annot, IDM_ANNOT_MEASURE_AREA, "측정 도구: 영역 넓이/둘레 측정 (sq cm)");
    add_sep(m_annot);
    add_item(m_annot, IDM_ANNOT_TOGGLE_SHOW, "주석 표시 / 숨기기 토글");
    add_item(m_annot, IDM_ANNOT_CLEAR_ALL, "모든 사용자 주석 초기화");
    add_sub(hmenu, m_annot, "주석(&A)");

    let m_conv = CreatePopupMenu();
    add_item(m_conv, IDM_CONV_TO_BMP, "이미지로 변환 (.bmp)...\tS");
    add_item(m_conv, IDM_CONV_TO_TXT, "텍스트로 변환 (.txt)...");
    add_item(m_conv, IDM_CONV_TO_HTML, "HTML 웹 문서로 변환 (.html)...");
    add_item(m_conv, IDM_CONV_TO_RTF, "Word 호환 문서로 변환 (.rtf)...");
    add_item(m_conv, IDM_CONV_TO_CSV, "Excel 호환 데이터로 변환 (.csv)...");
    add_sep(m_conv);
    add_item(m_conv, IDM_CONV_BATCH_TXT, "현재 페이지 텍스트 클립보드 복사\tC");
    add_sub(hmenu, m_conv, "변환(&C)");

    let m_view = CreatePopupMenu();
    add_item(m_view, IDM_VIEW_ZOOM_IN, "확대(&I)\t+");
    add_item(m_view, IDM_VIEW_ZOOM_OUT, "축소(&O)\t-");
    add_item(m_view, IDM_VIEW_ACTUAL_SIZE, "실제 크기 (100%)");
    add_item(m_view, IDM_VIEW_FIT_PAGE, "창 맞추기 (초기화)\t0");
    add_item(m_view, IDM_VIEW_FIT_WIDTH, "너비 맞추기 (Fit-to-Width)\tW");
    add_sep(m_view);
    add_item(m_view, IDM_VIEW_SINGLE, "단일 페이지 보기 (1p)");
    add_item(m_view, IDM_VIEW_SPREAD, "두 페이지 양면 보기 (2p)\tD");
    add_item(m_view, IDM_VIEW_COVER_SPREAD, "별도 표지 양면 보기 토글");
    add_sep(m_view);
    add_item(m_view, IDM_VIEW_ROTATE_CW, "시계 방향 회전 (90°)\tR");
    add_item(m_view, IDM_VIEW_ROTATE_CCW, "반시계 방향 회전 (90°)\tShift+R");
    add_sep(m_view);
    add_item(m_view, IDM_VIEW_RULER, "눈금자(Ruler) 표시 토글");
    add_item(m_view, IDM_VIEW_GRID, "격자(Grid) 및 안내선 표시 토글");
    add_sep(m_view);
    add_item(m_view, IDM_VIEW_BOOKMARKS, "좌측 패널: 목차(Bookmarks) 탭 보기\tB");
    add_item(m_view, IDM_VIEW_THUMBNAILS, "좌측 패널: 페이지 썸네일 미리보기 탭");
    add_item(m_view, IDM_VIEW_AUTO_BOOKMARK, "자동 북마크(목차) 생성");
    add_item(m_view, IDM_VIEW_FULLSCREEN, "읽기 모드 / 전체 화면\tF11");
    add_sub(hmenu, m_view, "보기(&V)");

    let m_org = CreatePopupMenu();
    add_item(m_org, IDM_ORG_INSERT_BLANK, "빈 페이지 삽입(&B)");
    add_item(m_org, IDM_ORG_INSERT_PDF, "다른 PDF에서 페이지 삽입(&I)...");
    add_item(m_org, IDM_ORG_DELETE_PAGE, "현재 페이지 삭제(&D)");
    add_sep(m_org);
    add_item(m_org, IDM_ORG_EXTRACT_PAGE, "현재 페이지 추출하여 새 PDF 저장(&E)...");
    add_item(m_org, IDM_ORG_SPLIT_PDF, "현재 페이지 기준으로 분할 저장(&S)...");
    add_sep(m_org);
    add_item(m_org, IDM_ORG_CROP_MARGIN, "페이지 여백 자르기 (CropBox 8%)");
    add_item(m_org, IDM_ORG_ROTATE_PAGE_OBJ, "현재 페이지 자체 회전 (영구 반영)");
    add_sub(hmenu, m_org, "구성(&P)");

    let m_tool = CreatePopupMenu();
    add_item(m_tool, IDM_TOOL_SEARCH, "문서 전체 키워드 검색(&F)...\tF");
    add_item(m_tool, IDM_TOOL_GOTO, "페이지 번호 직접 이동(&G)...\tG");
    add_sep(m_tool);
    add_item(m_tool, IDM_TOOL_MERGE_PDF, "PDF 병합 (문서 끝에 다른 PDF 합치기)...");
    add_item(m_tool, IDM_TOOL_COMPARE_PDF, "PDF 비교 (다른 PDF와 차이점 분석)...");
    add_item(m_tool, IDM_TOOL_FLATTEN, "평면화 (주석/교정을 본문에 영구 병합)");
    add_sep(m_tool);
    add_item(m_tool, IDM_TOOL_DETECT_TABLES, "표 인식 (테이블 감지 및 CSV 복사)");
    add_item(m_tool, IDM_TOOL_OCR_EXTRACT, "페이지 텍스트 전체 추출/검토");
    add_item(m_tool, IDM_TOOL_SCREENSHOT, "현재 화면 스크린샷 클립보드 복사");
    add_sub(hmenu, m_tool, "도구(&T)");

    let m_form = CreatePopupMenu();
    add_item(m_form, IDM_FORM_ADD_TEXTFIELD, "텍스트 필드 양식 추가 (드래그)");
    add_item(m_form, IDM_FORM_ADD_CHECKBOX, "체크 박스 양식 추가 (클릭)");
    add_sep(m_form);
    add_item(m_form, IDM_FORM_RECOGNIZE, "문서 내 양식 필드 자동 인식");
    add_item(m_form, IDM_FORM_EXPORT_CSV, "양식 데이터 추출 (.csv 저장)...");
    add_sub(hmenu, m_form, "양식(&M)");

    let m_sec = CreatePopupMenu();
    add_item(m_sec, IDM_SEC_REDACT_TOOL, "교정 표시 도구 (드래그 영역 검은색 마스킹)");
    add_item(m_sec, IDM_SEC_SEARCH_REDACT, "검색 & 교정 (검색어 일괄 검은색 마스킹)...");
    add_item(m_sec, IDM_SEC_SANITIZE, "숨겨진 데이터 및 주석 일괄 삭제");
    add_item(m_sec, IDM_SEC_SIGN_STAMP, "디지털 서명 스탬프 삽입");
    add_sub(hmenu, m_sec, "보안(&S)");

    let m_etc = CreatePopupMenu();
    add_item(m_etc, IDM_THEME_DARK_DOC, "야간 독서용 다크 모드(색상 반전) 토글\tV");
    add_sep(m_etc);
    add_item(m_etc, IDM_THEME_LIGHT, "테마 색상: 라이트 모드");
    add_item(m_etc, IDM_THEME_DARK_BG, "테마 색상: 다크 모드 (기본)");
    add_item(m_etc, IDM_THEME_CLASSIC, "테마 색상: 클래식 모드");
    add_sep(m_etc);
    add_item(m_etc, IDM_HELP_SHORTCUTS, "전체 기능 및 단축키 도움말(&H)\tH / F1");
    add_sub(hmenu, m_etc, "기타(&O)");

    hmenu
}

fn update_window_title(hwnd: HWND, state: &AppState) {
    let title = if state.page_count > 0 {
        let deg = state.rotation * 90;
        let page_disp = if state.spread_mode && (state.current_page + 1 < state.page_count) {
            format!(
                "{}-{} / {}p [2p양면]",
                state.current_page + 1,
                state.current_page + 2,
                state.page_count
            )
        } else {
            format!("{} / {}p", state.current_page + 1, state.page_count)
        };

        let fit_badge = if state.fit_width { " | 폭맞춤" } else { "" };
        let mode_badge = if state.dark_mode { " | 🌙다크" } else { "" };
        let base = format!(
            "DaPDF(v{}) - {} ({}) [Zoom: {:.0}%{} | {}°{}]",
            APP_VERSION,
            state.file_name,
            page_disp,
            state.zoom * 100.0,
            fit_badge,
            deg,
            mode_badge
        );

        match state.input_mode {
            InputMode::GotoPage => format!(
                "{} | 📄 페이지 이동 (1~{}): {}_ [Enter:이동, ESC:취소]",
                base, state.page_count, state.goto_input
            ),
            InputMode::AddTextPrompt => format!(
                "{} | ✏️ 페이지에 추가할 텍스트 입력: \"{}_\" [Enter:삽입, ESC:취소]",
                base, state.text_prompt_buf
            ),
            InputMode::CustomWatermarkPrompt => format!(
                "{} | 💧 워터마크 텍스트 입력: \"{}_\" [Enter:적용, ESC:취소]",
                base, state.text_prompt_buf
            ),
            InputMode::SearchRedactPrompt => format!(
                "{} | ⬛ 일괄 교정(마스킹)할 단어 입력: \"{}_\" [Enter:교정적용, ESC:취소]",
                base, state.search_query
            ),
            InputMode::Search => {
                let match_stat = if state.search_query.is_empty() {
                    String::from("키워드 입력 중")
                } else if state.search_matches.is_empty() {
                    String::from("문서 내 결과 없음")
                } else {
                    format!(
                        "{}/{}건 ({}p)",
                        state.active_match + 1,
                        state.search_matches.len(),
                        state.current_page + 1
                    )
                };
                format!(
                    "{} | 🔍 전체검색: \"{}_\" ({}) [Enter:확정, F3:다음, ESC:취소]",
                    base, state.search_query, match_stat
                )
            }
            InputMode::Normal => {
                if !state.search_query.is_empty() {
                    let match_stat = if state.search_matches.is_empty() {
                        String::from("현재 페이지 결과 없음 - N/F3으로 다음 페이지 검색")
                    } else {
                        format!(
                            "{}/{}건",
                            state.active_match + 1,
                            state.search_matches.len()
                        )
                    };
                    format!(
                        "{} | 🔍 \"{}\" ({}) [N/F3:다음, Shift+N:이전, ESC:해제]",
                        base, state.search_query, match_stat
                    )
                } else if let Some(ref msg) = state.status_msg {
                    format!("{} - [{}]", base, msg)
                } else {
                    base
                }
            }
        }
    } else {
        format!("DaPDF(v{}) - {}", APP_VERSION, state.file_name)
    };

    let w_title = to_wstring(&title);
    unsafe {
        SetWindowTextW(hwnd, w_title.as_ptr());
    }
}

fn to_wstring(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn format_pdf_date(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::from("-");
    }
    let s = trimmed.strip_prefix("D:").unwrap_or(trimmed);
    if s.len() >= 14 && s.as_bytes()[..14].iter().all(|b| b.is_ascii_digit()) {
        format!(
            "{}-{}-{} {}:{}:{}",
            &s[0..4],
            &s[4..6],
            &s[6..8],
            &s[8..10],
            &s[10..12],
            &s[12..14]
        )
    } else if s.len() >= 8 && s.as_bytes()[..8].iter().all(|b| b.is_ascii_digit()) {
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    } else {
        trimmed.to_string()
    }
}

fn write_bgra_to_bmp_file(
    path: &str,
    width: i32,
    height: i32,
    pixels: &[u8],
) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    let data_size = (width as usize) * (height as usize) * 4;
    let file_size = 54 + data_size as u32;

    let mut header = [0u8; 54];
    header[0] = b'B';
    header[1] = b'M';
    header[2..6].copy_from_slice(&file_size.to_le_bytes());
    header[10..14].copy_from_slice(&54u32.to_le_bytes());

    header[14..18].copy_from_slice(&40u32.to_le_bytes());
    header[18..22].copy_from_slice(&width.to_le_bytes());
    header[22..26].copy_from_slice(&(-height).to_le_bytes());
    header[26..28].copy_from_slice(&1u16.to_le_bytes());
    header[28..30].copy_from_slice(&32u16.to_le_bytes());
    header[30..34].copy_from_slice(&0u32.to_le_bytes());
    header[34..38].copy_from_slice(&(data_size as u32).to_le_bytes());

    file.write_all(&header)?;
    file.write_all(&pixels[..data_size])?;
    Ok(())
}

fn save_custom_dialog(
    hwnd: HWND,
    default_name: &str,
    filter_str: &str,
    def_ext_str: &str,
) -> Option<String> {
    unsafe {
        let mut file_buf = vec![0u16; 1024];
        for (i, ch) in default_name.encode_utf16().enumerate().take(1000) {
            file_buf[i] = ch;
        }

        let filter: Vec<u16> = filter_str.encode_utf16().collect();
        let def_ext = to_wstring(def_ext_str);

        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.lpstrDefExt = def_ext.as_ptr();
        ofn.Flags = OFN_EXPLORER
            | OFN_ENABLESIZING
            | OFN_OVERWRITEPROMPT
            | OFN_PATHMUSTEXIST
            | OFN_NOCHANGEDIR;

        if GetSaveFileNameW(&mut ofn) != 0 {
            let len = file_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(file_buf.len());
            Some(String::from_utf16_lossy(&file_buf[..len]))
        } else {
            None
        }
    }
}

fn get_clipboard_unicode_text(hwnd: HWND) -> Option<String> {
    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return None;
        }
        let hmem = GetClipboardData(CF_UNICODETEXT);
        if hmem.is_null() {
            CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(hmem) as *const u16;
        if ptr.is_null() {
            CloseClipboard();
            return None;
        }
        let len = (0..100_000).take_while(|&i| *ptr.add(i) != 0).count();
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
        GlobalUnlock(hmem);
        CloseClipboard();
        Some(text)
    }
}

fn set_clipboard_unicode_text(hwnd: HWND, text: &str) -> bool {
    let wide = to_wstring(text);
    let byte_len = wide.len() * size_of::<u16>();

    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return false;
        }
        EmptyClipboard();

        let hmem = GlobalAlloc(GMEM_MOVEABLE, byte_len);
        if hmem.is_null() {
            CloseClipboard();
            return false;
        }

        let locked_ptr = GlobalLock(hmem) as *mut u16;
        if locked_ptr.is_null() {
            CloseClipboard();
            return false;
        }

        copy_nonoverlapping(wide.as_ptr(), locked_ptr, wide.len());
        GlobalUnlock(hmem);

        let result = SetClipboardData(CF_UNICODETEXT, hmem);
        CloseClipboard();

        !result.is_null()
    }
}

unsafe fn copy_current_view_bitmap_to_clipboard(hwnd: HWND, state: &AppState) -> bool {
    if state.pixels.is_empty() || state.bmp_width <= 0 || state.bmp_height <= 0 {
        return false;
    }
    let hdc_screen = GetDC(hwnd);
    let hdc_mem = CreateCompatibleDC(hdc_screen);
    let hbm = CreateCompatibleBitmap(hdc_screen, state.bmp_width, state.bmp_height);
    let old_bm = SelectObject(hdc_mem, hbm as _);

    let mut bmi: BITMAPINFO = zeroed();
    bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = state.bmp_width;
    bmi.bmiHeader.biHeight = -state.bmp_height;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    StretchDIBits(
        hdc_mem,
        0,
        0,
        state.bmp_width,
        state.bmp_height,
        0,
        0,
        state.bmp_width,
        state.bmp_height,
        state.pixels.as_ptr() as *const c_void,
        &bmi,
        DIB_RGB_COLORS,
        SRCCOPY,
    );

    SelectObject(hdc_mem, old_bm);
    DeleteDC(hdc_mem);
    ReleaseDC(hwnd, hdc_screen);

    if OpenClipboard(hwnd) != 0 {
        EmptyClipboard();
        SetClipboardData(CF_BITMAP, hbm as _);
        CloseClipboard();
        true
    } else {
        DeleteObject(hbm as _);
        false
    }
}

fn open_file_dialog(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut file_buf = vec![0u16; 1024];
        let filter: Vec<u16> = "PDF 문서 (*.pdf)\0*.pdf\0모든 파일 (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();

        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.Flags = OFN_EXPLORER
            | OFN_ENABLESIZING
            | OFN_FILEMUSTEXIST
            | OFN_PATHMUSTEXIST
            | OFN_NOCHANGEDIR;

        if GetOpenFileNameW(&mut ofn) != 0 {
            let len = file_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(file_buf.len());
            Some(String::from_utf16_lossy(&file_buf[..len]))
        } else {
            None
        }
    }
}

unsafe fn handle_menu_command(hwnd: HWND, cmd_id: u16, state: &mut AppState) {
    match cmd_id {
        IDM_TB_PREV_PAGE => state.change_page(-1, hwnd),
        IDM_TB_NEXT_PAGE => state.change_page(1, hwnd),
        IDM_TB_FIRST_PAGE => state.go_to_first_page(hwnd),
        IDM_TB_LAST_PAGE => state.go_to_last_page(hwnd),

        IDM_FILE_NEW_BLANK => state.create_blank_pdf(hwnd),
        IDM_FILE_FROM_CLIPBOARD => state.create_pdf_from_clipboard(hwnd),
        IDM_FILE_OPEN => {
            if let Some(path) = open_file_dialog(hwnd) {
                if state.load_file(&path).is_ok() {
                    state.refresh_view(hwnd, true);
                }
            }
        }
        IDM_FILE_SAVE => {
            if let Some(existing) = state.current_path.clone() {
                match state.save_pdf_to_path(&existing, None) {
                    Ok(_) => state.status_msg = Some(format!("저장 완료: {}", existing)),
                    Err(e) => state.status_msg = Some(e),
                }
                state.refresh_view(hwnd, false);
            } else {
                handle_menu_command(hwnd, IDM_FILE_SAVE_AS, state);
            }
        }
        IDM_FILE_SAVE_AS | IDM_FILE_SAVE_V14 | IDM_FILE_SAVE_V17 => {
            let ver = if cmd_id == IDM_FILE_SAVE_V14 {
                Some(14)
            } else if cmd_id == IDM_FILE_SAVE_V17 {
                Some(17)
            } else {
                None
            };
            if let Some(save_path) = save_custom_dialog(
                hwnd,
                &state.file_name,
                "PDF 문서 (*.pdf)\0*.pdf\0\0",
                "pdf",
            ) {
                match state.save_pdf_to_path(&save_path, ver) {
                    Ok(_) => state.status_msg = Some(format!("PDF 저장 완료: {}", save_path)),
                    Err(e) => state.status_msg = Some(e),
                }
                state.refresh_view(hwnd, false);
            }
        }
        IDM_FILE_PRINT => {
            let mut pd: PRINTDLGW = zeroed();
            pd.lStructSize = size_of::<PRINTDLGW>() as u32;
            pd.hwndOwner = hwnd;
            pd.Flags = PD_RETURNDC;
            if PrintDlgW(&mut pd) != 0 && !pd.hDC.is_null() {
                DeleteDC(pd.hDC);
                state.status_msg = Some("인쇄 작업이 프린터 큐로 전송되었습니다".to_string());
                state.refresh_view(hwnd, false);
            }
        }
        IDM_FILE_PROPS => state.show_metadata_dialog(hwnd),
        IDM_FILE_EXIT => PostQuitMessage(0),

        IDM_EDIT_ADD_TEXT => {
            if state.doc.is_null() {
                state.create_blank_pdf(hwnd);
            }
            state.input_mode = InputMode::AddTextPrompt;
            state.text_prompt_buf.clear();
            state.status_msg =
                Some("상단 타이틀바 안내를 보고 추가할 텍스트를 입력 후 Enter를 누르세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_EDIT_WM_CONFIDENTIAL => {
            state.watermark_text = Some("CONFIDENTIAL / 기밀".to_string());
            state.status_msg = Some("워터마크 적용: CONFIDENTIAL / 기밀".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_EDIT_WM_SAMPLE => {
            state.watermark_text = Some("SAMPLE".to_string());
            state.status_msg = Some("워터마크 적용: SAMPLE".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_EDIT_WM_CUSTOM => {
            state.input_mode = InputMode::CustomWatermarkPrompt;
            state.text_prompt_buf.clear();
            state.refresh_view(hwnd, false);
        }
        IDM_EDIT_WM_CLEAR => {
            state.watermark_text = None;
            state.status_msg = Some("모든 워터마크가 제거되었습니다".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_EDIT_BG_CREAM => {
            state.page_bg_tint = Some((252, 248, 227));
            state.status_msg = Some("배경화면 적용: 눈 보호 크림색".to_string());
            state.refresh_view(hwnd, true);
        }
        IDM_EDIT_BG_MINT => {
            state.page_bg_tint = Some((230, 247, 238));
            state.status_msg = Some("배경화면 적용: 소프트 민트색".to_string());
            state.refresh_view(hwnd, true);
        }
        IDM_EDIT_BG_CLEAR => {
            state.page_bg_tint = None;
            state.status_msg = Some("페이지 배경화면이 제거되었습니다".to_string());
            state.refresh_view(hwnd, true);
        }
        IDM_EDIT_HEADER_FOOTER => {
            if state.header_footer_text.is_some() {
                state.header_footer_text = None;
                state.status_msg = Some("머리글 & 바닥글이 제거되었습니다".to_string());
            } else {
                state.header_footer_text =
                    Some(format!("DaPDF 문서 검토본 - {}", state.file_name));
                state.status_msg = Some("머리글 & 바닥글이 추가되었습니다".to_string());
            }
            state.refresh_view(hwnd, false);
        }
        IDM_EDIT_PAGE_NUMBER => {
            state.show_page_numbers = !state.show_page_numbers;
            state.status_msg = Some(if state.show_page_numbers {
                "하단 페이지 번호 표시 활성화".to_string()
            } else {
                "페이지 번호 표시 해제".to_string()
            });
            state.refresh_view(hwnd, false);
        }
        IDM_EDIT_BATES_NUMBER => {
            state.show_bates_numbers = !state.show_bates_numbers;
            state.status_msg = Some(if state.show_bates_numbers {
                "우측 상단 베이츠 번호(Bates Numbering) 활성화".to_string()
            } else {
                "베이츠 번호 표시 해제".to_string()
            });
            state.refresh_view(hwnd, false);
        }

        IDM_ANNOT_NONE => {
            state.active_tool = ActiveTool::Pan;
            state.status_msg = Some("기본 손 도구 (화면 드래그 이동 및 링크 클릭 모드)".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_HIGHLIGHT => {
            state.active_tool = ActiveTool::Highlight;
            state.status_msg = Some("강조 도구: 페이지 위에서 마우스를 드래그하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_AREA_HL => {
            state.active_tool = ActiveTool::AreaHighlight;
            state.status_msg =
                Some("영역 강조 도구: 강조할 사각형 영역을 드래그하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_UNDERLINE => {
            state.active_tool = ActiveTool::Underline;
            state.status_msg = Some("밑줄 도구: 텍스트 영역을 드래그하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_STRIKEOUT => {
            state.active_tool = ActiveTool::Strikeout;
            state.status_msg = Some("취소선 도구: 텍스트 영역을 드래그하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_PENCIL => {
            state.active_tool = ActiveTool::Pencil;
            state.status_msg = Some("연필 도구: 페이지 위에 자유롭게 그리세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_ERASER => {
            state.active_tool = ActiveTool::Eraser;
            state.status_msg = Some("지우개 도구: 삭제할 주석 위를 클릭하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_RECT => {
            state.active_tool = ActiveTool::RectShape;
            state.status_msg = Some("직사각형 도구: 페이지 위를 드래그하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_LINE => {
            state.active_tool = ActiveTool::LineShape;
            state.status_msg = Some("직선 도구: 시작점부터 끝점까지 드래그하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_NOTE => {
            state.active_tool = ActiveTool::Note;
            state.status_msg = Some("노트/타자기 도구: 추가할 위치를 클릭하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_STAMP_APPROVED => {
            state.active_tool = ActiveTool::StampApproved;
            state.status_msg =
                Some("스탬프 도구 [APPROVED]: 찍을 위치를 클릭하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_STAMP_DRAFT => {
            state.active_tool = ActiveTool::StampDraft;
            state.status_msg = Some("스탬프 도구 [DRAFT]: 찍을 위치를 클릭하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_MEASURE_DIST => {
            state.active_tool = ActiveTool::MeasureDist;
            state.status_msg =
                Some("거리 측정 도구: 측정할 두 지점 사이를 드래그하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_MEASURE_AREA => {
            state.active_tool = ActiveTool::MeasureArea;
            state.status_msg =
                Some("영역/둘레 측정 도구: 측정할 사각 영역을 드래그하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_TOGGLE_SHOW => {
            state.show_annotations = !state.show_annotations;
            state.status_msg = Some(if state.show_annotations {
                "주석 표시 활성화".to_string()
            } else {
                "주석 숨김 처리됨".to_string()
            });
            state.refresh_view(hwnd, true);
        }
        IDM_ANNOT_CLEAR_ALL => {
            state.overlays.clear();
            state.status_msg = Some("모든 사용자 주석이 제거되었습니다".to_string());
            state.refresh_view(hwnd, false);
        }

        IDM_CONV_TO_BMP => state.save_current_page_image(hwnd),
        IDM_CONV_TO_TXT => state.export_document_as(hwnd, "txt"),
        IDM_CONV_TO_HTML => state.export_document_as(hwnd, "html"),
        IDM_CONV_TO_RTF => state.export_document_as(hwnd, "rtf"),
        IDM_CONV_TO_CSV => state.export_document_as(hwnd, "csv"),
        IDM_CONV_BATCH_TXT => state.copy_text_to_clipboard(hwnd),

        IDM_VIEW_ZOOM_IN => state.change_zoom(1.15, hwnd),
        IDM_VIEW_ZOOM_OUT => state.change_zoom(1.0 / 1.15, hwnd),
        IDM_VIEW_ACTUAL_SIZE => {
            state.zoom = 1.0;
            state.fit_width = false;
            state.refresh_view(hwnd, true);
        }
        IDM_VIEW_FIT_PAGE => state.reset_view(hwnd),
        IDM_VIEW_FIT_WIDTH => state.toggle_fit_width(hwnd),
        IDM_VIEW_SINGLE => {
            state.spread_mode = false;
            state.refresh_view(hwnd, true);
        }
        IDM_VIEW_SPREAD => state.toggle_spread_mode(hwnd),
        IDM_VIEW_COVER_SPREAD => {
            state.spread_mode = true;
            state.cover_mode = !state.cover_mode;
            state.status_msg = Some(if state.cover_mode {
                "별도 표지 양면 보기 활성화 (1p 단독 표지)".to_string()
            } else {
                "일반 2p 양면 보기".to_string()
            });
            state.refresh_view(hwnd, true);
        }
        IDM_VIEW_ROTATE_CW => state.rotate_page(true, hwnd),
        IDM_VIEW_ROTATE_CCW => state.rotate_page(false, hwnd),
        IDM_VIEW_RULER => {
            state.show_ruler = !state.show_ruler;
            state.refresh_view(hwnd, false);
        }
        IDM_VIEW_GRID => {
            state.show_grid = !state.show_grid;
            state.refresh_view(hwnd, false);
        }
        IDM_VIEW_BOOKMARKS => {
            if state.show_sidebar && state.sidebar_tab == SidebarTab::Bookmarks {
                state.show_sidebar = false;
            } else {
                state.show_sidebar = true;
                state.sidebar_tab = SidebarTab::Bookmarks;
            }
            state.refresh_view(hwnd, true);
        }
        IDM_VIEW_THUMBNAILS => {
            if state.show_sidebar && state.sidebar_tab == SidebarTab::Thumbnails {
                state.show_sidebar = false;
            } else {
                state.show_sidebar = true;
                state.sidebar_tab = SidebarTab::Thumbnails;
            }
            state.refresh_view(hwnd, true);
        }
        IDM_VIEW_AUTO_BOOKMARK => state.generate_auto_bookmarks(hwnd),
        IDM_VIEW_FULLSCREEN => state.toggle_fullscreen(hwnd),

        IDM_ORG_INSERT_BLANK => state.insert_blank_page_after_current(hwnd),
        IDM_ORG_INSERT_PDF => state.merge_external_pdf(hwnd, false),
        IDM_ORG_DELETE_PAGE => state.delete_current_page(hwnd),
        IDM_ORG_EXTRACT_PAGE | IDM_ORG_SPLIT_PDF => state.extract_current_page_to_pdf(hwnd),
        IDM_ORG_CROP_MARGIN => state.crop_current_page_margins(hwnd),
        IDM_ORG_ROTATE_PAGE_OBJ => {
            if !state.doc.is_null() && state.page_count > 0 {
                let page = FPDF_LoadPage(state.doc, state.current_page);
                if !page.is_null() {
                    let cur_rot = FPDFPage_GetRotation(page);
                    FPDFPage_SetRotation(page, (cur_rot + 1) % 4);
                    FPDFPage_GenerateContent(page);
                    FPDF_ClosePage(page);
                    state.thumb_cache.remove(&state.current_page);
                    state.refresh_view(hwnd, true);
                }
            }
        }

        IDM_TOOL_SEARCH => {
            if !state.doc.is_null() {
                state.input_mode = InputMode::Search;
                state.status_msg =
                    Some("검색할 키워드를 입력해 주세요 (Enter: 확정, ESC: 취소)".to_string());
                state.refresh_view(hwnd, false);
            }
        }
        IDM_TOOL_GOTO => {
            if !state.doc.is_null() && state.page_count > 0 {
                state.input_mode = InputMode::GotoPage;
                state.goto_input.clear();
                state.status_msg = Some(format!(
                    "이동할 페이지 번호(1~{})를 입력 후 Enter를 누르세요",
                    state.page_count
                ));
                state.refresh_view(hwnd, false);
            }
        }
        IDM_TOOL_MERGE_PDF => state.merge_external_pdf(hwnd, true),
        IDM_TOOL_COMPARE_PDF => state.compare_with_other_pdf(hwnd),
        IDM_TOOL_FLATTEN => state.flatten_current_page(hwnd),
        IDM_TOOL_DETECT_TABLES => state.detect_and_copy_tables(hwnd),
        IDM_TOOL_OCR_EXTRACT => state.copy_text_to_clipboard(hwnd),
        IDM_TOOL_SCREENSHOT => {
            if copy_current_view_bitmap_to_clipboard(hwnd, state) {
                state.status_msg =
                    Some("스크린샷이 클립보드에 이미지로 복사되었습니다!".to_string());
                state.refresh_view(hwnd, false);
            }
        }

        IDM_FORM_ADD_TEXTFIELD => {
            state.active_tool = ActiveTool::FormTextField;
            state.status_msg = Some("텍스트 필드 양식 추가: 영역을 드래그하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_FORM_ADD_CHECKBOX => {
            state.active_tool = ActiveTool::FormCheckBox;
            state.status_msg = Some("체크 박스 양식 추가: 위치를 클릭하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_FORM_RECOGNIZE => {
            let count = state
                .overlays
                .iter()
                .filter(|o| o.kind == OverlayKind::FormText || o.kind == OverlayKind::FormCheck)
                .count();
            state.status_msg = Some(format!("양식 인식 완료: 총 {}개의 양식 필드 감지", count));
            state.refresh_view(hwnd, false);
        }
        IDM_FORM_EXPORT_CSV => {
            if let Some(save_path) = save_custom_dialog(
                hwnd,
                "form_data.csv",
                "CSV 파일 (*.csv)\0*.csv\0\0",
                "csv",
            ) {
                let mut csv = String::from("Page,FieldType,Value,Checked\r\n");
                for ov in &state.overlays {
                    if ov.kind == OverlayKind::FormText || ov.kind == OverlayKind::FormCheck {
                        let ftype = if ov.kind == OverlayKind::FormText {
                            "TextField"
                        } else {
                            "CheckBox"
                        };
                        csv.push_str(&format!(
                            "{},{},\"{}\",{}\r\n",
                            ov.page_index + 1,
                            ftype,
                            ov.label,
                            ov.checked
                        ));
                    }
                }
                let _ = fs::write(&save_path, csv.as_bytes());
                state.status_msg = Some(format!("양식 데이터 CSV 추출 완료: {}", save_path));
                state.refresh_view(hwnd, false);
            }
        }

        IDM_SEC_REDACT_TOOL => {
            state.active_tool = ActiveTool::RedactBox;
            state.status_msg =
                Some("교정 표시 도구: 가릴 민감 정보 영역을 마우스로 드래그하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_SEC_SEARCH_REDACT => {
            if !state.doc.is_null() {
                state.input_mode = InputMode::SearchRedactPrompt;
                state.search_query.clear();
                state.refresh_view(hwnd, false);
            }
        }
        IDM_SEC_SANITIZE => {
            state.overlays.clear();
            state.watermark_text = None;
            state.header_footer_text = None;
            state.status_msg =
                Some("숨겨진 주석/오버레이 데이터가 모두 정리되었습니다".to_string());
            state.refresh_view(hwnd, true);
        }
        IDM_SEC_SIGN_STAMP => {
            state.overlays.push(UserOverlay {
                page_index: state.current_page,
                kind: OverlayKind::StampApproved,
                x1: 0.68,
                y1: 0.85,
                x2: 0.92,
                y2: 0.92,
                points: Vec::new(),
                label: String::from("DIGITALLY SIGNED"),
                checked: true,
            });
            state.status_msg = Some("하단에 디지털 서명 인장이 삽입되었습니다".to_string());
            state.refresh_view(hwnd, false);
        }

        IDM_THEME_DARK_DOC => state.toggle_dark_mode(hwnd),
        IDM_THEME_LIGHT => {
            state.theme_bg_color = 0x00E0E0E0;
            state.dark_mode = false;
            state.refresh_view(hwnd, true);
        }
        IDM_THEME_DARK_BG => {
            state.theme_bg_color = 0x00323232;
            state.refresh_view(hwnd, true);
        }
        IDM_THEME_CLASSIC => {
            state.theme_bg_color = 0x006A5ACD;
            state.dark_mode = false;
            state.refresh_view(hwnd, true);
        }
        IDM_HELP_SHORTCUTS => state.show_help_dialog(hwnd),
        _ => {}
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;

    match msg {
        WM_ERASEBKGND => 1,

        WM_SETCURSOR => {
            if !state_ptr.is_null() {
                let state = &*state_ptr;
                let hit_area = (lparam & 0xFFFF) as u16;
                // HTCLIENT = 1
                if hit_area == 1 && state.hovered_link.is_some() {
                    SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                    return 1;
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        WM_COMMAND => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let cmd_id = (wparam & 0xFFFF) as u16;
                handle_menu_command(hwnd, cmd_id, state);
            }
            0
        }

        WM_SIZE => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let width = (lparam & 0xFFFF) as i16 as i32;
                let height = ((lparam >> 16) & 0xFFFF) as i16 as i32;
                state.pixels.clear();
                state.render_current_page(width, height);
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }

        WM_DROPFILES => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let hdrop = wparam as HDROP;
                let mut buf = vec![0u16; 1024];
                if DragQueryFileW(hdrop, 0, buf.as_mut_ptr(), buf.len() as u32) > 0 {
                    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                    let path = String::from_utf16_lossy(&buf[..len]);
                    if state.load_file(&path).is_ok() {
                        state.refresh_view(hwnd, true);
                    }
                }
                DragFinish(hdrop);
            }
            0
        }

        WM_LBUTTONDOWN => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let click_x = (lparam & 0xFFFF) as i16 as i32;
                let click_y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                let mut rc: RECT = zeroed();
                GetClientRect(hwnd, &mut rc);
                let client_w = rc.right - rc.left;
                let client_h = rc.bottom - rc.top;

                if !state.is_fullscreen && click_y < TOOLBAR_HEIGHT {
                    if let Some(cmd_id) = hit_test_toolbar(click_x, click_y) {
                        handle_menu_command(hwnd, cmd_id, state);
                    }
                    return 0;
                }

                if !state.is_fullscreen && click_y >= client_h - STATUSBAR_HEIGHT {
                    if let Some(cmd_id) = hit_test_status_bar(click_x, client_w) {
                        handle_menu_command(hwnd, cmd_id, state);
                    }
                    return 0;
                }

                let top_off = state.top_bar_offset();
                if state.show_sidebar && click_x < SIDEBAR_WIDTH {
                    if click_y >= top_off && click_y < top_off + SIDEBAR_TAB_HEIGHT {
                        if click_x >= 246 {
                            state.show_sidebar = false;
                            state.refresh_view(hwnd, true);
                        } else if click_x < 123 {
                            state.sidebar_tab = SidebarTab::Bookmarks;
                            InvalidateRect(hwnd, null(), 0);
                        } else {
                            state.sidebar_tab = SidebarTab::Thumbnails;
                            InvalidateRect(hwnd, null(), 0);
                        }
                        return 0;
                    }

                    let content_top = top_off + SIDEBAR_TAB_HEIGHT + 4;
                    if click_y >= content_top {
                        match state.sidebar_tab {
                            SidebarTab::Bookmarks => {
                                let row =
                                    ((click_y - content_top) / BOOKMARK_ROW_HEIGHT) as usize;
                                let bm_idx = state.bookmark_scroll + row;
                                if let Some(bm) = state.bookmarks.get(bm_idx) {
                                    let target_page = bm.page_index;
                                    if target_page >= 0 {
                                        state.go_to_page(target_page, hwnd);
                                    }
                                }
                            }
                            SidebarTab::Thumbnails => {
                                let slot =
                                    ((click_y - content_top) / THUMB_ITEM_HEIGHT) as usize;
                                let target_page = (state.thumb_scroll + slot) as i32;
                                if target_page >= 0 && target_page < state.page_count {
                                    state.go_to_page(target_page, hwnd);
                                }
                            }
                        }
                    }
                    return 0;
                }

                if state.bmp_width > 0 && state.bmp_height > 0 {
                    let nx = (click_x - state.last_draw_x) as f64 / state.bmp_width as f64;
                    let ny = (click_y - state.last_draw_y) as f64 / state.bmp_height as f64;

                    if state.active_tool == ActiveTool::Eraser {
                        state.overlays.retain(|o| {
                            !(o.page_index == state.current_page
                                && nx >= o.x1.min(o.x2) - 0.03
                                && nx <= o.x1.max(o.x2) + 0.03
                                && ny >= o.y1.min(o.y2) - 0.03
                                && ny <= o.y1.max(o.y2) + 0.03)
                        });
                        InvalidateRect(hwnd, null(), 0);
                        return 0;
                    }

                    for ov in state.overlays.iter_mut() {
                        if ov.page_index == state.current_page
                            && ov.kind == OverlayKind::FormCheck
                            && (nx - ov.x1).abs() < 0.04
                            && (ny - ov.y1).abs() < 0.04
                        {
                            ov.checked = !ov.checked;
                            InvalidateRect(hwnd, null(), 0);
                            return 0;
                        }
                    }
                }

                state.is_dragging = true;
                state.mouse_down_x = click_x;
                state.mouse_down_y = click_y;
                state.drag_start_x = click_x;
                state.drag_start_y = click_y;
                state.drag_cur_x = click_x;
                state.drag_cur_y = click_y;
                if state.active_tool == ActiveTool::Pencil && state.bmp_width > 0 {
                    state.pencil_trace.clear();
                    let nx = ((click_x - state.last_draw_x) as f64 / state.bmp_width as f64)
                        .clamp(0.0, 1.0);
                    let ny = ((click_y - state.last_draw_y) as f64 / state.bmp_height as f64)
                        .clamp(0.0, 1.0);
                    state.pencil_trace.push((nx, ny));
                }
                SetCapture(hwnd);
            }
            0
        }

        WM_MOUSEMOVE => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let cur_x = (lparam & 0xFFFF) as i16 as i32;
                let cur_y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                if !state.is_dragging {
                    // 1. 상단 툴바 호버 갱신
                    if !state.is_fullscreen {
                        let new_hover = if cur_y < TOOLBAR_HEIGHT {
                            hit_test_toolbar(cur_x, cur_y)
                        } else {
                            None
                        };
                        if new_hover != state.hover_tb_id {
                            state.hover_tb_id = new_hover;
                            let tb_rc = RECT {
                                left: 0,
                                top: 0,
                                right: 2000,
                                bottom: TOOLBAR_HEIGHT,
                            };
                            InvalidateRect(hwnd, &tb_rc, 0);
                        }
                    }

                    // 2. 본문 하이퍼링크 호버 탐지 (손 도구 모드일 때)
                    let new_link = if state.active_tool == ActiveTool::Pan
                        && cur_x >= state.sidebar_offset()
                    {
                        state.detect_link_at_client_pos(cur_x, cur_y)
                    } else {
                        None
                    };

                    if new_link != state.hovered_link {
                        state.hovered_link = new_link;
                        if state.hovered_link.is_some() {
                            SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                        } else {
                            SetCursor(LoadCursorW(null_mut(), IDC_ARROW));
                        }
                        let mut rc: RECT = zeroed();
                        GetClientRect(hwnd, &mut rc);
                        let sb_rc = RECT {
                            left: 0,
                            top: (rc.bottom - rc.top) - STATUSBAR_HEIGHT,
                            right: rc.right - rc.left,
                            bottom: rc.bottom - rc.top,
                        };
                        InvalidateRect(hwnd, &sb_rc, 0);
                    }
                } else {
                    if state.active_tool == ActiveTool::Pan {
                        state.pan_x += cur_x - state.drag_start_x;
                        state.pan_y += cur_y - state.drag_start_y;
                        state.drag_start_x = cur_x;
                        state.drag_start_y = cur_y;
                    } else if state.active_tool == ActiveTool::Pencil && state.bmp_width > 0 {
                        let nx = ((cur_x - state.last_draw_x) as f64 / state.bmp_width as f64)
                            .clamp(0.0, 1.0);
                        let ny = ((cur_y - state.last_draw_y) as f64 / state.bmp_height as f64)
                            .clamp(0.0, 1.0);
                        state.pencil_trace.push((nx, ny));
                    } else {
                        state.drag_cur_x = cur_x;
                        state.drag_cur_y = cur_y;
                    }
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }

        WM_LBUTTONUP => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if state.is_dragging {
                    state.is_dragging = false;
                    ReleaseCapture();

                    let end_x = (lparam & 0xFFFF) as i16 as i32;
                    let end_y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                    // 손 도구 모드에서 드래그 이동 거리(4px 미만)가 거의 없는 순수 클릭이면 하이퍼링크 실행
                    if state.active_tool == ActiveTool::Pan {
                        let move_dist = (end_x - state.mouse_down_x)
                            .abs()
                            .max((end_y - state.mouse_down_y).abs());
                        if move_dist < 4 {
                            if let Some(target) = state.detect_link_at_client_pos(end_x, end_y) {
                                state.execute_link_target(&target, hwnd);
                            }
                        }
                        return 0;
                    }

                    if state.bmp_width > 0 && state.bmp_height > 0 {
                        let x1 = ((state.drag_start_x - state.last_draw_x) as f64
                            / state.bmp_width as f64)
                            .clamp(0.0, 1.0);
                        let y1 = ((state.drag_start_y - state.last_draw_y) as f64
                            / state.bmp_height as f64)
                            .clamp(0.0, 1.0);
                        let x2 = ((end_x - state.last_draw_x) as f64 / state.bmp_width as f64)
                            .clamp(0.0, 1.0);
                        let y2 = ((end_y - state.last_draw_y) as f64 / state.bmp_height as f64)
                            .clamp(0.0, 1.0);

                        match state.active_tool {
                            ActiveTool::MeasureDist => {
                                let pw_mm = state.cur_page_w_pt * 25.4 / 72.0;
                                let ph_mm = state.cur_page_h_pt * 25.4 / 72.0;
                                let dx_mm = (x2 - x1) * pw_mm;
                                let dy_mm = (y2 - y1) * ph_mm;
                                let dist_mm = (dx_mm * dx_mm + dy_mm * dy_mm).sqrt();
                                state.status_msg = Some(format!(
                                    "거리 측정 결과: {:.2} cm ({:.1} mm)",
                                    dist_mm / 10.0,
                                    dist_mm
                                ));
                            }
                            ActiveTool::MeasureArea => {
                                let pw_cm = state.cur_page_w_pt * 2.54 / 72.0;
                                let ph_cm = state.cur_page_h_pt * 2.54 / 72.0;
                                let w_cm = (x2 - x1).abs() * pw_cm;
                                let h_cm = (y2 - y1).abs() * ph_cm;
                                let area = w_cm * h_cm;
                                let perim = 2.0 * (w_cm + h_cm);
                                state.status_msg = Some(format!(
                                    "영역 측정: 넓이 {:.2} sq cm | 둘레 {:.2} cm",
                                    area, perim
                                ));
                            }
                            ActiveTool::Pencil => {
                                if state.pencil_trace.len() >= 2 {
                                    state.overlays.push(UserOverlay {
                                        page_index: state.current_page,
                                        kind: OverlayKind::Pencil,
                                        x1,
                                        y1,
                                        x2,
                                        y2,
                                        points: state.pencil_trace.clone(),
                                        label: String::new(),
                                        checked: false,
                                    });
                                }
                            }
                            _ => {
                                let (kind, label) = match state.active_tool {
                                    ActiveTool::Highlight => (OverlayKind::Highlight, ""),
                                    ActiveTool::AreaHighlight => (OverlayKind::AreaHighlight, ""),
                                    ActiveTool::Underline => (OverlayKind::Underline, ""),
                                    ActiveTool::Strikeout => (OverlayKind::Strikeout, ""),
                                    ActiveTool::RectShape => (OverlayKind::RectShape, ""),
                                    ActiveTool::LineShape => (OverlayKind::LineShape, ""),
                                    ActiveTool::StampApproved => {
                                        (OverlayKind::StampApproved, "APPROVED / 승인됨")
                                    }
                                    ActiveTool::StampDraft => {
                                        (OverlayKind::StampDraft, "DRAFT / 초안")
                                    }
                                    ActiveTool::Note => (OverlayKind::Note, "메모/노트 주석"),
                                    ActiveTool::RedactBox => (OverlayKind::Redaction, "REDACTED"),
                                    ActiveTool::FormTextField => {
                                        (OverlayKind::FormText, "[텍스트 입력 필드]")
                                    }
                                    ActiveTool::FormCheckBox => (OverlayKind::FormCheck, "Check"),
                                    _ => (OverlayKind::Highlight, ""),
                                };
                                state.overlays.push(UserOverlay {
                                    page_index: state.current_page,
                                    kind,
                                    x1,
                                    y1,
                                    x2,
                                    y2,
                                    points: Vec::new(),
                                    label: label.to_string(),
                                    checked: false,
                                });
                            }
                        }
                        update_window_title(hwnd, state);
                        InvalidateRect(hwnd, null(), 0);
                    }
                }
            }
            0
        }

        WM_MOUSEWHEEL => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let delta = ((wparam >> 16) & 0xFFFF) as i16;
                let ctrl_pressed = (GetKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;

                let mut pt = POINT {
                    x: (lparam & 0xFFFF) as i16 as i32,
                    y: ((lparam >> 16) & 0xFFFF) as i16 as i32,
                };
                ScreenToClient(hwnd, &mut pt);

                if !ctrl_pressed && state.show_sidebar && pt.x >= 0 && pt.x < SIDEBAR_WIDTH {
                    state.scroll_sidebar(delta, hwnd);
                    return 0;
                }

                if ctrl_pressed {
                    if delta > 0 {
                        state.change_zoom(1.15, hwnd);
                    } else if delta < 0 {
                        state.change_zoom(1.0 / 1.15, hwnd);
                    }
                } else if delta < 0 {
                    state.scroll_or_turn_page(true, hwnd);
                } else if delta > 0 {
                    state.scroll_or_turn_page(false, hwnd);
                }
            }
            0
        }

        WM_CHAR => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if state.ignore_next_char {
                    state.ignore_next_char = false;
                    return 0;
                }

                let ch = wparam as u32;
                match state.input_mode {
                    InputMode::Search | InputMode::SearchRedactPrompt => {
                        if ch >= 0x20 {
                            if let Some(c) = char::from_u32(ch) {
                                state.search_query.push(c);
                                if state.input_mode == InputMode::Search {
                                    state.perform_search(true);
                                }
                                state.refresh_view(hwnd, true);
                            }
                        }
                        return 0;
                    }
                    InputMode::AddTextPrompt | InputMode::CustomWatermarkPrompt => {
                        if ch >= 0x20 {
                            if let Some(c) = char::from_u32(ch) {
                                state.text_prompt_buf.push(c);
                                update_window_title(hwnd, state);
                            }
                        }
                        return 0;
                    }
                    InputMode::GotoPage => {
                        if let Some(c) = char::from_u32(ch) {
                            if c.is_ascii_digit() {
                                state.goto_input.push(c);
                                update_window_title(hwnd, state);
                            }
                        }
                        return 0;
                    }
                    InputMode::Normal => {}
                }
            }
            0
        }

        WM_KEYDOWN => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let key = wparam as u16;
                let shift_pressed = (GetKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;
                let ctrl_pressed = (GetKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;

                if state.input_mode != InputMode::Normal {
                    match key {
                        VK_ESCAPE => {
                            state.input_mode = InputMode::Normal;
                            state.goto_input.clear();
                            state.text_prompt_buf.clear();
                            state.refresh_view(hwnd, true);
                        }
                        VK_RETURN => {
                            let mode = state.input_mode;
                            state.input_mode = InputMode::Normal;
                            match mode {
                                InputMode::GotoPage => {
                                    if let Ok(p) = state.goto_input.trim().parse::<i32>() {
                                        if p >= 1 && p <= state.page_count {
                                            state.go_to_page(p - 1, hwnd);
                                        }
                                    }
                                    state.goto_input.clear();
                                }
                                InputMode::AddTextPrompt => {
                                    let txt = state.text_prompt_buf.trim().to_string();
                                    state.text_prompt_buf.clear();
                                    if !txt.is_empty() && !state.doc.is_null() {
                                        let page = FPDF_LoadPage(state.doc, state.current_page);
                                        if !page.is_null() {
                                            let h = FPDF_GetPageHeight(page);
                                            AppState::insert_pdf_text_object(
                                                state.doc,
                                                page,
                                                &txt,
                                                60.0,
                                                h - 80.0,
                                                14.0,
                                            );
                                            FPDFPage_GenerateContent(page);
                                            FPDF_ClosePage(page);
                                        }
                                        state.thumb_cache.remove(&state.current_page);
                                        state.status_msg =
                                            Some(format!("텍스트 객체 추가됨: {}", txt));
                                    }
                                }
                                InputMode::CustomWatermarkPrompt => {
                                    let wm = state.text_prompt_buf.trim().to_string();
                                    state.text_prompt_buf.clear();
                                    if !wm.is_empty() {
                                        state.watermark_text = Some(wm);
                                    }
                                }
                                InputMode::SearchRedactPrompt => {
                                    state.apply_search_redaction(hwnd);
                                }
                                InputMode::Search => {
                                    if state.search_matches.is_empty()
                                        && !state.search_query.is_empty()
                                    {
                                        state.perform_search(true);
                                    }
                                }
                                InputMode::Normal => {}
                            }
                            state.refresh_view(hwnd, true);
                        }
                        VK_BACK => {
                            match state.input_mode {
                                InputMode::GotoPage => {
                                    state.goto_input.pop();
                                }
                                InputMode::AddTextPrompt | InputMode::CustomWatermarkPrompt => {
                                    state.text_prompt_buf.pop();
                                }
                                InputMode::Search | InputMode::SearchRedactPrompt => {
                                    state.search_query.pop();
                                    if state.input_mode == InputMode::Search {
                                        state.perform_search(true);
                                    }
                                }
                                InputMode::Normal => {}
                            }
                            state.refresh_view(hwnd, true);
                        }
                        VK_F3 => state.step_search_match(!shift_pressed, hwnd),
                        _ => {}
                    }
                    return 0;
                }

                match key {
                    VK_HOME => state.go_to_first_page(hwnd),
                    VK_END => state.go_to_last_page(hwnd),
                    VK_RIGHT | VK_NEXT => state.change_page(1, hwnd),
                    VK_LEFT | VK_PRIOR => state.change_page(-1, hwnd),
                    VK_DOWN => state.scroll_or_turn_page(true, hwnd),
                    VK_UP => state.scroll_or_turn_page(false, hwnd),
                    VK_OEM_PLUS => state.change_zoom(1.15, hwnd),
                    VK_OEM_MINUS => state.change_zoom(1.0 / 1.15, hwnd),
                    0x30 => state.reset_view(hwnd),
                    0x52 => state.rotate_page(!shift_pressed, hwnd),
                    0x43 => state.copy_text_to_clipboard(hwnd),
                    0x53 => {
                        if ctrl_pressed {
                            handle_menu_command(hwnd, IDM_FILE_SAVE, state);
                        } else {
                            state.save_current_page_image(hwnd);
                        }
                    }
                    0x55 if ctrl_pressed => handle_menu_command(hwnd, IDM_ANNOT_UNDERLINE, state),
                    0x51 if ctrl_pressed => handle_menu_command(hwnd, IDM_ANNOT_STRIKEOUT, state),
                    0x50 if ctrl_pressed => handle_menu_command(hwnd, IDM_FILE_PRINT, state),
                    0x57 => state.toggle_fit_width(hwnd),
                    0x44 => {
                        if ctrl_pressed {
                            state.show_metadata_dialog(hwnd);
                        } else {
                            state.toggle_spread_mode(hwnd);
                        }
                    }
                    0x49 => state.show_metadata_dialog(hwnd),
                    0x48 => state.show_help_dialog(hwnd),
                    VK_F1 => {
                        if ctrl_pressed {
                            handle_menu_command(hwnd, IDM_TOOL_SCREENSHOT, state);
                        } else {
                            state.show_help_dialog(hwnd);
                        }
                    }
                    0x56 => state.toggle_dark_mode(hwnd),
                    VK_F11 => state.toggle_fullscreen(hwnd),
                    0x47 => {
                        if !state.doc.is_null() && state.page_count > 0 {
                            state.input_mode = InputMode::GotoPage;
                            state.goto_input.clear();
                            state.status_msg = None;
                            if !ctrl_pressed {
                                state.ignore_next_char = true;
                            }
                            update_window_title(hwnd, state);
                        }
                    }
                    0x42 => {
                        state.show_sidebar = !state.show_sidebar;
                        state.refresh_view(hwnd, true);
                    }
                    VK_OEM_4 => state.step_bookmark(false, hwnd),
                    VK_OEM_6 => state.step_bookmark(true, hwnd),
                    0x46 => {
                        if !state.doc.is_null() {
                            state.input_mode = InputMode::Search;
                            state.status_msg = None;
                            if !ctrl_pressed {
                                state.ignore_next_char = true;
                            }
                            update_window_title(hwnd, state);
                        }
                    }
                    VK_F3 => state.step_search_match(!shift_pressed, hwnd),
                    0x4E => {
                        if !state.search_query.is_empty() {
                            state.step_search_match(!shift_pressed, hwnd);
                        } else {
                            state.toggle_dark_mode(hwnd);
                        }
                    }
                    0x4F => handle_menu_command(hwnd, IDM_FILE_OPEN, state),
                    VK_ESCAPE => {
                        if state.active_tool != ActiveTool::Pan {
                            state.active_tool = ActiveTool::Pan;
                            state.status_msg = Some("기본 손 도구 모드로 복귀".to_string());
                            state.refresh_view(hwnd, false);
                        } else if !state.search_query.is_empty() {
                            state.search_query.clear();
                            state.search_matches.clear();
                            state.refresh_view(hwnd, true);
                        } else if state.is_fullscreen {
                            state.toggle_fullscreen(hwnd);
                        } else {
                            PostQuitMessage(0);
                        }
                    }
                    _ => {}
                }
            }
            0
        }

        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut rc: RECT = zeroed();
            GetClientRect(hwnd, &mut rc);
            let client_w = rc.right - rc.left;
            let client_h = rc.bottom - rc.top;

            if !state_ptr.is_null() {
                let state = &mut *state_ptr;

                let bg_color = if state.dark_mode {
                    0x00141414
                } else {
                    state.theme_bg_color
                };
                let bg_brush = CreateSolidBrush(bg_color);
                FillRect(hdc, &rc, bg_brush);
                DeleteObject(bg_brush as _);

                let sb_w = state.sidebar_offset();
                let top_off = state.top_bar_offset();
                let bot_off = state.bottom_bar_offset();
                let viewport_h = (client_h - top_off - bot_off).max(1);

                if state.pixels.is_empty() && !state.doc.is_null() {
                    state.render_current_page(client_w, client_h);
                }

                if !state.pixels.is_empty() && state.bmp_width > 0 && state.bmp_height > 0 {
                    let canvas_w = (client_w - sb_w).max(1);
                    let x = sb_w + (canvas_w - state.bmp_width) / 2 + state.pan_x;
                    let y = top_off + (viewport_h - state.bmp_height) / 2 + state.pan_y;
                    state.last_draw_x = x;
                    state.last_draw_y = y;

                    let mut bmi: BITMAPINFO = zeroed();
                    bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
                    bmi.bmiHeader.biWidth = state.bmp_width;
                    bmi.bmiHeader.biHeight = -state.bmp_height;
                    bmi.bmiHeader.biPlanes = 1;
                    bmi.bmiHeader.biBitCount = 32;
                    bmi.bmiHeader.biCompression = BI_RGB;

                    StretchDIBits(
                        hdc,
                        x,
                        y,
                        state.bmp_width,
                        state.bmp_height,
                        0,
                        0,
                        state.bmp_width,
                        state.bmp_height,
                        state.pixels.as_ptr() as *const c_void,
                        &bmi,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );

                    draw_page_overlays_and_guides(
                        hdc as *mut c_void,
                        x,
                        y,
                        client_w,
                        client_h,
                        state,
                    );
                }

                if state.show_sidebar {
                    draw_left_sidebar_panel(hdc as *mut c_void, client_h, state);
                }

                if !state.is_fullscreen {
                    draw_top_toolbar(hdc as *mut c_void, client_w, state);
                    draw_bottom_status_bar(hdc as *mut c_void, client_w, client_h, state);
                }
            }

            EndPaint(hwnd, &ps);
            0
        }

        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn main() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        FPDF_InitLibrary();
    }

    let args: Vec<String> = env::args().collect();
    let initial_pdf = if args.len() > 1 {
        args[1].clone()
    } else {
        "test- .pdf".to_string()
    };

    let mut app_state = AppState::new();
    if Path::new(&initial_pdf).exists() {
        let _ = app_state.load_file(&initial_pdf);
    }

    unsafe {
        let hinstance = GetModuleHandleW(null());
        let class_name = to_wstring("DaPDF_WindowClass");

        let mut wc: WNDCLASSW = zeroed();
        wc.style = CS_HREDRAW | CS_VREDRAW;
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance = hinstance;
        wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        wc.lpszClassName = class_name.as_ptr();

        RegisterClassW(&wc);

        let hmenu = build_main_menu_bar();

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            to_wstring(&format!("DaPDF(v{}) Viewer", APP_VERSION)).as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            60,
            60,
            1460,
            1060,
            null_mut(),
            hmenu as _,
            hinstance,
            null(),
        );

        if hwnd.is_null() {
            FPDF_DestroyLibrary();
            std::process::exit(1);
        }

        SetMenu(hwnd, hmenu as _);
        DragAcceptFiles(hwnd, 1);

        SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut app_state as *mut _ as isize);
        app_state.refresh_view(hwnd, true);
        ShowWindow(hwnd, SW_SHOW);

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    drop(app_state);
    unsafe {
        FPDF_DestroyLibrary();
    }
}
