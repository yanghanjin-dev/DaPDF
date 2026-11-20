#![windows_subsystem = "windows"]
#![allow(dead_code, non_snake_case, non_camel_case_types)]

use std::collections::HashMap;
use std::env;
use std::ffi::{c_void, CStr, CString};
use std::fs::{self, File};
use std::io::Write;
use std::mem::{size_of, zeroed};
use std::os::raw::{c_char, c_double, c_float, c_int, c_ulong, c_ushort};
use std::path::Path;
use std::ptr::{copy_nonoverlapping, null, null_mut};

#[cfg(test)]
mod regression_tests;
mod annotations;
mod printing;
mod history;
mod pages;
mod conversion;

use windows_sys::Win32::{
    Foundation::{GlobalFree, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
    Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen,
        CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetDC, GetDIBits,
        GetMonitorInfoW, GetStockObject, GetTextExtentPoint32W, InvalidateRect, LineTo, MonitorFromWindow, MoveToEx,
        Rectangle, ReleaseDC, ScreenToClient, SelectObject, SetBkMode, SetTextColor, StretchDIBits,
        TextOutW, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DEFAULT_GUI_FONT, DIB_RGB_COLORS,
        MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, PS_DOT, PS_SOLID, SRCCOPY, TRANSPARENT,
    },
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
        },
        LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryA},
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    },
    UI::{
        Controls::Dialogs::{
            GetOpenFileNameW, GetSaveFileNameW, PrintDlgW, OFN_ENABLESIZING, OFN_EXPLORER,
            OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST,
            OPENFILENAMEW, PD_RETURNDC, PRINTDLGW,
        },
        HiDpi::{GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
        Input::KeyboardAndMouse::{
            GetKeyState, ReleaseCapture, SetCapture, VK_BACK, VK_CONTROL, VK_DOWN, VK_END,
            VK_ESCAPE, VK_F1, VK_F11, VK_F3, VK_HOME, VK_LEFT, VK_NEXT, VK_OEM_4, VK_OEM_6,
            VK_OEM_MINUS, VK_OEM_PLUS, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_TAB, VK_UP,
        },
        Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, ShellExecuteW, HDROP},
        WindowsAndMessaging::{
            AppendMenuW, CreateMenu, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
            DispatchMessageW, GetClientRect, GetCursorPos, GetMessageW, GetWindowLongPtrW,
            GetWindowPlacement, LoadCursorW, LoadIconW, MessageBoxW, PostQuitMessage, RegisterClassW,
            SetCursor, SetMenu, SetWindowLongPtrW, SetWindowPlacement, SetWindowPos,
            SetWindowTextW, ShowWindow, TrackPopupMenu, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
            GWLP_USERDATA, GWL_STYLE, HWND_TOP, IDC_ARROW, IDC_HAND, IDC_IBEAM,
            MB_ICONINFORMATION, MB_OK, MF_POPUP, MF_SEPARATOR, MF_STRING, MSG, SWP_FRAMECHANGED,
            SWP_NOOWNERZORDER, SWP_NOZORDER, SW_SHOWNORMAL, SW_SHOW, TPM_NONOTIFY, TPM_RETURNCMD,
            WINDOWPLACEMENT, WM_CHAR, WM_COMMAND, WM_DESTROY, WM_DROPFILES, WM_ERASEBKGND,
            WM_CLOSE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT,
            WM_RBUTTONDOWN, WM_SETCURSOR, WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
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

const PDFACTION_GOTO: c_ulong = 1;
const PDFACTION_URI: c_ulong = 3;

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
const IDM_EDIT_UNDO: u16 = 2098;
const IDM_EDIT_REDO: u16 = 2099;
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
const IDM_ANNOT_TEXT_SELECT: u16 = 3016;

const IDM_CONV_TO_BMP: u16 = 4001;
const IDM_CONV_TO_TXT: u16 = 4002;
const IDM_CONV_TO_HTML: u16 = 4003;
const IDM_CONV_TO_RTF: u16 = 4004;
const IDM_CONV_TO_CSV: u16 = 4005;
const IDM_CONV_BATCH_TXT: u16 = 4006;
const IDM_CONV_TO_DOCX: u16 = 4007;

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
const IDM_VIEW_AUTO_BOOKMARK: u16 = 5014;
const IDM_VIEW_FULLSCREEN: u16 = 5015;
const IDM_VIEW_THUMBNAILS: u16 = 5016;

const IDM_ORG_INSERT_BLANK: u16 = 6001;
const IDM_ORG_INSERT_PDF: u16 = 6002;
const IDM_ORG_DELETE_PAGE: u16 = 6003;
const IDM_ORG_EXTRACT_PAGE: u16 = 6004;
const IDM_ORG_SPLIT_PDF: u16 = 6005;
const IDM_ORG_CROP_MARGIN: u16 = 6006;
const IDM_ORG_ROTATE_PAGE_OBJ: u16 = 6007;
const IDM_ORG_EXTRACT_RANGE: u16 = 6010;
const IDM_ORG_REORDER: u16 = 6011;

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
const IDM_FORM_ADD_RADIO: u16 = 8005;
const IDM_FORM_ADD_DROPDOWN: u16 = 8006;
const IDM_FORM_ADD_DATE: u16 = 8007;
const IDM_FORM_EXPORT_FDF: u16 = 8008;
const IDM_FORM_IMPORT_FDF: u16 = 8009;
const IDM_FORM_RESET_DATA: u16 = 8010;

const IDM_CTX_COPY: u16 = 8901;
const IDM_CTX_HIGHLIGHT: u16 = 8902;
const IDM_CTX_SEARCH: u16 = 8903;

const IDM_SEC_REDACT_TOOL: u16 = 9001;
const IDM_SEC_SEARCH_REDACT: u16 = 9002;
const IDM_SEC_SANITIZE: u16 = 9003;
const IDM_SEC_SIGN_STAMP: u16 = 9004;

const IDM_SIGN_UPLOAD_IMAGE: u16 = 9101;
const IDM_SIGN_DRAW_HAND: u16 = 9102;
const IDM_SIGN_CREATE_SEAL: u16 = 9103;
const IDM_SIGN_PLACE_SAVED: u16 = 9104;

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
#[derive(Clone, Copy)]
struct BLENDFUNCTION {
    BlendOp: u8,
    BlendFlags: u8,
    SourceConstantAlpha: u8,
    AlphaFormat: u8,
}

#[link(name = "msimg32")]
extern "system" {
    fn AlphaBlend(
        hdcDest: *mut c_void,
        xoriginDest: i32,
        yoriginDest: i32,
        wDest: i32,
        hDest: i32,
        hdcSrc: *mut c_void,
        xoriginSrc: i32,
        yoriginSrc: i32,
        wSrc: i32,
        hSrc: i32,
        ftn: BLENDFUNCTION,
    ) -> i32;
}

#[repr(C)]
struct GdiplusStartupInput {
    GdiplusVersion: u32,
    DebugEventCallback: *const c_void,
    SuppressBackgroundThread: i32,
    SuppressExternalCodecs: i32,
}

#[repr(C)]
struct BitmapData {
    Width: u32,
    Height: u32,
    Stride: i32,
    PixelFormat: i32,
    Scan0: *mut u8,
    Reserved: usize,
}

#[repr(C)]
struct GpRect {
    X: i32,
    Y: i32,
    Width: i32,
    Height: i32,
}

unsafe fn gdiplus_proc(name: &[u8]) -> Option<unsafe extern "system" fn() -> isize> {
    let mut hmod = GetModuleHandleW(to_wstring("gdiplus.dll").as_ptr());
    if hmod.is_null() {
        hmod = LoadLibraryA(b"gdiplus.dll\0".as_ptr());
    }
    if hmod.is_null() {
        None
    } else {
        GetProcAddress(hmod, name.as_ptr())
    }
}

unsafe fn GdiplusStartup(token: *mut usize, input: *const GdiplusStartupInput, output: *mut c_void) -> i32 {
    type F = unsafe extern "system" fn(*mut usize, *const GdiplusStartupInput, *mut c_void) -> i32;
    if let Some(p) = gdiplus_proc(b"GdiplusStartup\0") { (std::mem::transmute::<_, F>(p))(token, input, output) } else { 1 }
}

unsafe fn GdipCreateBitmapFromFile(filename: *const u16, bitmap: *mut *mut c_void) -> i32 {
    type F = unsafe extern "system" fn(*const u16, *mut *mut c_void) -> i32;
    if let Some(p) = gdiplus_proc(b"GdipCreateBitmapFromFile\0") { (std::mem::transmute::<_, F>(p))(filename, bitmap) } else { 1 }
}

unsafe fn GdipGetImageWidth(image: *mut c_void, width: *mut u32) -> i32 {
    type F = unsafe extern "system" fn(*mut c_void, *mut u32) -> i32;
    if let Some(p) = gdiplus_proc(b"GdipGetImageWidth\0") { (std::mem::transmute::<_, F>(p))(image, width) } else { 1 }
}

unsafe fn GdipGetImageHeight(image: *mut c_void, height: *mut u32) -> i32 {
    type F = unsafe extern "system" fn(*mut c_void, *mut u32) -> i32;
    if let Some(p) = gdiplus_proc(b"GdipGetImageHeight\0") { (std::mem::transmute::<_, F>(p))(image, height) } else { 1 }
}

unsafe fn GdipBitmapLockBits(bitmap: *mut c_void, rect: *const GpRect, flags: u32, format: i32, locked_data: *mut BitmapData) -> i32 {
    type F = unsafe extern "system" fn(*mut c_void, *const GpRect, u32, i32, *mut BitmapData) -> i32;
    if let Some(p) = gdiplus_proc(b"GdipBitmapLockBits\0") { (std::mem::transmute::<_, F>(p))(bitmap, rect, flags, format, locked_data) } else { 1 }
}

unsafe fn GdipBitmapUnlockBits(bitmap: *mut c_void, locked_data: *mut BitmapData) -> i32 {
    type F = unsafe extern "system" fn(*mut c_void, *mut BitmapData) -> i32;
    if let Some(p) = gdiplus_proc(b"GdipBitmapUnlockBits\0") { (std::mem::transmute::<_, F>(p))(bitmap, locked_data) } else { 1 }
}

unsafe fn GdipDisposeImage(image: *mut c_void) -> i32 {
    type F = unsafe extern "system" fn(*mut c_void) -> i32;
    if let Some(p) = gdiplus_proc(b"GdipDisposeImage\0") { (std::mem::transmute::<_, F>(p))(image) } else { 1 }
}

#[derive(Clone)]
struct SignatureStampData {
    width: i32,
    height: i32,
    pm_bgra: Vec<u8>,
    raw_bgra: Vec<u8>,
}

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
    fn FPDFText_GetCharIndexAtPos(
        text_page: *mut c_void,
        x: c_double,
        y: c_double,
        x_tolerance: c_double,
        y_tolerance: c_double,
    ) -> c_int;
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

    fn FPDFBookmark_GetFirstChild(document: *mut c_void, bookmark: *mut c_void) -> *mut c_void;
    fn FPDFBookmark_GetNextSibling(document: *mut c_void, bookmark: *mut c_void) -> *mut c_void;
    fn FPDFBookmark_GetTitle(
        bookmark: *mut c_void,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;
    fn FPDFBookmark_GetDest(document: *mut c_void, bookmark: *mut c_void) -> *mut c_void;
    fn FPDFBookmark_GetAction(bookmark: *mut c_void) -> *mut c_void;

    fn FPDFLink_GetLinkAtPoint(page: *mut c_void, x: c_double, y: c_double) -> *mut c_void;
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
        R: u32,
        G: u32,
        B: u32,
        A: u32,
    ) -> c_int;
    fn FPDFPageObj_CreateNewRect(x: c_float, y: c_float, w: c_float, h: c_float) -> *mut c_void;
    fn FPDFPath_SetDrawMode(path: *mut c_void, fillmode: c_int, stroke: c_int) -> c_int;
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

#[derive(Clone)]
struct TextSelection {
    page_index: i32,
    anchor_char: i32,
    rects: Vec<PdfRect>,
    text: String,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum SidebarTab {
    Bookmarks,
    Thumbnails,
    Search,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum InputMode {
    Normal,
    Search,
    GotoPage,
    AddTextPrompt,
    CustomWatermarkPrompt,
    SearchRedactPrompt,
    EditFormField(usize),
    CreateSealPrompt,
    PageOperation(u16),
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum ActiveTool {
    Pan,
    TextSelect,
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
    FormRadio,
    FormDropdown,
    FormDate,
    PlaceSignature,
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
    FormRadio,
    FormDropdown,
    SignatureImage(usize),
}

#[derive(Clone, PartialEq)]
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
    ToolbarButton { id: IDM_FILE_OPEN, label: "📂 열기", width: 60 },
    ToolbarButton { id: IDM_FILE_SAVE, label: "💾 저장", width: 60 },
    ToolbarButton { id: IDM_VIEW_ZOOM_OUT, label: "🔍-", width: 40 },
    ToolbarButton { id: IDM_VIEW_ZOOM_IN, label: "🔍+", width: 40 },
    ToolbarButton { id: IDM_VIEW_FIT_WIDTH, label: "↔ 폭맞춤", width: 68 },
    ToolbarButton { id: IDM_VIEW_SPREAD, label: "📖 2p양면", width: 70 },
    ToolbarButton { id: IDM_THEME_DARK_DOC, label: "🌙 다크", width: 60 },
    ToolbarButton { id: IDM_VIEW_THUMBNAILS, label: "🖼 썸네일", width: 70 },
    ToolbarButton { id: IDM_ANNOT_NONE, label: "✋ 이동", width: 60 },
    ToolbarButton { id: IDM_ANNOT_TEXT_SELECT, label: "🔤 선택", width: 60 },
    ToolbarButton { id: IDM_ANNOT_HIGHLIGHT, label: "🖍 강조", width: 60 },
    ToolbarButton { id: IDM_ANNOT_PENCIL, label: "✏ 연필", width: 60 },
    ToolbarButton { id: IDM_SEC_REDACT_TOOL, label: "⬛ 교정", width: 60 },
    ToolbarButton { id: IDM_SIGN_UPLOAD_IMAGE, label: "✒ 서명", width: 60 },
    ToolbarButton { id: IDM_EDIT_ADD_TEXT, label: "➕ 텍스트", width: 70 },
    ToolbarButton { id: IDM_TOOL_SEARCH, label: "🔎 검색", width: 60 },
    ToolbarButton { id: IDM_TB_PREV_PAGE, label: "◀", width: 32 },
    ToolbarButton { id: IDM_TB_NEXT_PAGE, label: "▶", width: 32 },
];

struct AppState {
    sidebar_width: i32,
    resizing_sidebar: bool,
    search_caret_visible: bool,
    window_focused: bool,
    status_controls: Vec<StatusControl>,
    status_info_rect: RECT,
    statusbar_height: i32,
    history: history::EditHistory,
    toolbar_rects: Vec<RECT>,
    toolbar_height: i32,
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
    text_selection: Option<TextSelection>,

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
    signatures: Vec<SignatureStampData>,
    active_sig_idx: Option<usize>,
    selected_overlay_idx: Option<usize>,
    is_resizing_overlay: bool,
    is_moving_overlay: bool,
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
            sidebar_width: SIDEBAR_WIDTH,
            resizing_sidebar: false,
            search_caret_visible: true,
            window_focused: true,
            status_controls: Vec::new(),
            status_info_rect: unsafe { zeroed() },
            statusbar_height: STATUSBAR_HEIGHT,
            history: history::EditHistory::default(),
            toolbar_rects: Vec::new(),
            toolbar_height: TOOLBAR_HEIGHT,
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
            p1_rect_in_bmp: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            p2_rect_in_bmp: None,
            last_draw_x: 0,
            last_draw_y: 0,
            status_msg: None,
            hover_tb_id: None,
            hovered_link: None,
            text_selection: None,
            input_mode: InputMode::Normal,
            ignore_next_char: false,
            text_prompt_buf: String::new(),
            search_query: String::new(),
            search_matches: Vec::new(),
            active_match: 0,
            goto_input: String::new(),
            bookmarks: Vec::new(),
            show_sidebar: true,
            sidebar_tab: SidebarTab::Thumbnails,
            bookmark_scroll: 0,
            thumb_scroll: 0,
            thumb_cache: HashMap::new(),
            active_tool: ActiveTool::TextSelect,
            show_annotations: true,
            overlays: Vec::new(),
            signatures: Vec::new(),
            active_sig_idx: None,
            selected_overlay_idx: None,
            is_resizing_overlay: false,
            is_moving_overlay: false,
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
        if self.is_fullscreen { 0 } else { self.toolbar_height }
    }

    fn bottom_bar_offset(&self) -> i32 {
        if self.is_fullscreen { 0 } else { self.statusbar_height }
    }

    fn hit_test_page_area(&self, client_x: i32, client_y: i32) -> Option<(i32, i32, i32, i32, i32)> {
        if self.doc.is_null() || self.page_count <= 0 || self.bmp_width <= 0 {
            return None;
        }
        let rel_x = client_x - self.last_draw_x;
        let rel_y = client_y - self.last_draw_y;

        if rel_x >= self.p1_rect_in_bmp.left
            && rel_x < self.p1_rect_in_bmp.right
            && rel_y >= self.p1_rect_in_bmp.top
            && rel_y < self.p1_rect_in_bmp.bottom
        {
            return Some((
                self.current_page,
                rel_x - self.p1_rect_in_bmp.left,
                rel_y - self.p1_rect_in_bmp.top,
                self.p1_rect_in_bmp.right - self.p1_rect_in_bmp.left,
                self.p1_rect_in_bmp.bottom - self.p1_rect_in_bmp.top,
            ));
        }

        if let Some(p2_rc) = self.p2_rect_in_bmp {
            if self.current_page + 1 < self.page_count
                && rel_x >= p2_rc.left
                && rel_x < p2_rc.right
                && rel_y >= p2_rc.top
                && rel_y < p2_rc.bottom
            {
                return Some((
                    self.current_page + 1,
                    rel_x - p2_rc.left,
                    rel_y - p2_rc.top,
                    p2_rc.right - p2_rc.left,
                    p2_rc.bottom - p2_rc.top,
                ));
            }
        }
        None
    }

    fn get_char_index_at_client_pos(&self, client_x: i32, client_y: i32) -> Option<(i32, i32)> {
        let (page_idx, dev_x, dev_y, size_x, size_y) = self.hit_test_page_area(client_x, client_y)?;
        if size_x <= 0 || size_y <= 0 {
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
            let mut page_x: c_double = 0.0;
            let mut page_y: c_double = 0.0;
            FPDF_DeviceToPage(page, 0, 0, size_x, size_y, self.rotation, dev_x, dev_y, &mut page_x, &mut page_y);
            let char_idx = FPDFText_GetCharIndexAtPos(text_page, page_x, page_y, 12.0, 12.0);
            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
            if char_idx >= 0 { Some((page_idx, char_idx)) } else { None }
        }
    }

    fn build_text_selection(&self, page_idx: i32, anchor_char: i32, cur_char: i32) -> Option<TextSelection> {
        if self.doc.is_null() || page_idx < 0 || page_idx >= self.page_count {
            return None;
        }
        let s_idx = anchor_char.min(cur_char);
        let e_idx = anchor_char.max(cur_char);
        let count = e_idx - s_idx + 1;
        if count <= 0 {
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
            let rect_count = FPDFText_CountRects(text_page, s_idx, count);
            let mut rects = Vec::new();
            for r in 0..rect_count {
                let (mut left, mut top, mut right, mut bottom) = (0.0, 0.0, 0.0, 0.0);
                if FPDFText_GetRect(text_page, r, &mut left, &mut top, &mut right, &mut bottom) != 0 {
                    rects.push(PdfRect { left, top, right, bottom });
                }
            }
            let mut buf = vec![0u16; (count + 2) as usize];
            let written = FPDFText_GetText(text_page, s_idx, count, buf.as_mut_ptr());
            let selected_text = if written > 0 {
                let valid_len = (written as usize).saturating_sub(1);
                String::from_utf16_lossy(&buf[..valid_len])
            } else {
                String::new()
            };
            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
            if rects.is_empty() && selected_text.is_empty() {
                None
            } else {
                Some(TextSelection { page_index: page_idx, anchor_char, rects, text: selected_text })
            }
        }
    }

    fn detect_link_at_client_pos(&self, client_x: i32, client_y: i32) -> Option<LinkTarget> {
        let (page_idx, dev_x, dev_y, size_x, size_y) = self.hit_test_page_area(client_x, client_y)?;
        self.query_link_on_page(page_idx, dev_x, dev_y, size_x, size_y)
    }

    fn query_link_on_page(&self, page_idx: i32, dev_x: i32, dev_y: i32, size_x: i32, size_y: i32) -> Option<LinkTarget> {
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
            FPDF_DeviceToPage(page, 0, 0, size_x, size_y, self.rotation, dev_x, dev_y, &mut page_x, &mut page_y);
            let link = FPDFLink_GetLinkAtPoint(page, page_x, page_y);
            let mut result = None;
            if !link.is_null() {
                let dest = FPDFLink_GetDest(self.doc, link);
                if !dest.is_null() {
                    let target_p = FPDFDest_GetDestPageIndex(self.doc, dest);
                    if target_p >= 0 {
                        result = Some(LinkTarget::InternalPage(target_p));
                    }
                }
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
                                FPDFAction_GetURIPath(self.doc, action, buf.as_mut_ptr() as *mut c_void, len);
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
                    ShellExecuteW(hwnd, w_open.as_ptr(), w_uri.as_ptr(), null(), null(), SW_SHOWNORMAL);
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
            let (pw, ph) = if self.rotation % 2 == 1 { (raw_h, raw_w) } else { (raw_w, raw_h) };
            let scale = (156.0 / pw.max(1.0)).min(176.0 / ph.max(1.0));
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
            let entry = ThumbnailCache { width: tw, height: th, pixels };
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
            self.history = history::EditHistory::default();
            self.pdf_bytes.clear();
            self.current_path = None;
            self.file_name = String::from("제목 없음-1.pdf");
            self.page_count = FPDF_GetPageCount(self.doc);
            self.current_page = 0;
            self.overlays.clear();
            self.bookmarks.clear();
            self.text_selection = None;
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

    unsafe fn insert_pdf_text_object(doc: *mut c_void, page: *mut c_void, text: &str, x: f64, y: f64, size: f32) {
        let Ok(font_name) = CString::new("Helvetica") else { return };
        let dll_name = to_wstring("pdfium.dll");
        let hmod = GetModuleHandleW(dll_name.as_ptr());
        if hmod.is_null() {
            return;
        }
        type CreateTextObjFn = unsafe extern "C" fn(*mut c_void, *const c_char, c_float) -> *mut c_void;
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
            // Save a separate document so retries cannot duplicate annotations or
            // mutate the editable in-memory document on an I/O failure.
            let document_bytes = annotations::serialize_document(self.doc, None)?;
            let copy = annotations::PdfDocument::from_bytes(&document_bytes)?;
            annotations::write_overlays(copy.0, &self.overlays)?;
            let mut ctx = FileWriteContext {
                fw: FPDF_FILEWRITE { version: 1, WriteBlock: Some(write_block_cb) },
                buf: Vec::new(),
            };
            let ok = if let Some(ver) = version {
                FPDF_SaveWithVersion(copy.0, &mut ctx.fw, 0, ver)
            } else {
                FPDF_SaveAsCopy(copy.0, &mut ctx.fw, 0)
            };
            if ok == 0 || ctx.buf.is_empty() {
                return Err("PDFium 문서 저장 실패".to_string());
            }
            annotations::atomic_write(Path::new(path), &ctx.buf)?;
            self.current_path = Some(path.to_string());
            self.file_name = Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path).to_string();
            self.mark_saved()?;
            Ok(())
        }
    }

    fn load_file(&mut self, path: &str) -> Result<(), String> {
        let bytes = fs::read(path).map_err(|e| format!("파일 읽기 실패: {}", e))?;
        if bytes.is_empty() {
            return Err("빈 파일입니다.".to_string());
        }
        let new_doc = unsafe { FPDF_LoadMemDocument(bytes.as_ptr() as *const c_void, bytes.len() as c_int, null()) };
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
        self.text_selection = None;
        self.status_msg = None;
        self.input_mode = InputMode::Normal;
        self.active_tool = ActiveTool::TextSelect;
        self.search_query.clear();
        self.search_matches.clear();
        self.active_match = 0;
        self.goto_input.clear();
        self.bookmark_scroll = 0;
        self.thumb_scroll = 0;
        self.file_name = Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or(path).to_string();
        self.load_bookmarks();
        self.show_sidebar = true;
        self.sidebar_tab = SidebarTab::Thumbnails;
        self.overlays = unsafe { annotations::take_managed_overlays(self.doc) };
        self.history = history::EditHistory::default();
        self.mark_saved()?;
        Ok(())
    }

    fn generate_auto_bookmarks(&mut self, hwnd: HWND) {
        if self.doc.is_null() || self.page_count <= 0 {
            return;
        }
        self.bookmarks.clear();
        for p in 0..self.page_count {
            if let Some(txt) = self.extract_page_text_at(p) {
                if let Some(first_line) = txt.lines().map(|l| l.trim()).find(|l| l.chars().count() >= 2) {
                    let snippet: String = first_line.chars().take(28).collect();
                    self.bookmarks.push(BookmarkItem { title: snippet, page_index: p, depth: 0 });
                }
            }
        }
        self.show_sidebar = true;
        self.sidebar_tab = SidebarTab::Bookmarks;
        self.bookmark_scroll = 0;
        self.status_msg = Some(format!("자동 북마크 생성 완료 ({}개 항목)", self.bookmarks.len()));
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
                for overlay in &mut self.overlays { if overlay.page_index >= insert_idx { overlay.page_index += 1; } }
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
            self.overlays.retain(|o|o.page_index!=self.current_page);
            for overlay in &mut self.overlays { if overlay.page_index>self.current_page { overlay.page_index-=1; } }
            self.page_count = FPDF_GetPageCount(self.doc);
            if self.current_page >= self.page_count {
                self.current_page = (self.page_count - 1).max(0);
            }
            self.clear_thumbnail_cache();
            self.status_msg = Some(format!("페이지 삭제 완료 (현재 총 {}페이지)", self.page_count));
            self.refresh_view(hwnd, true);
        }
    }

    fn merge_external_pdf(&mut self, hwnd: HWND, at_end: bool) {
        if self.doc.is_null() {
            self.create_blank_pdf(hwnd);
        }
        let Some(ext_path) = open_file_dialog(hwnd) else { return };
        let Ok(bytes) = fs::read(&ext_path) else { return };
        unsafe {
            let src_doc = FPDF_LoadMemDocument(bytes.as_ptr() as *const c_void, bytes.len() as c_int, null());
            if src_doc.is_null() {
                self.status_msg = Some("병합할 PDF 파일을 열 수 없습니다".to_string());
                update_window_title(hwnd, self);
                return;
            }
            let insert_idx = if at_end { self.page_count } else { (self.current_page + 1).min(self.page_count) };
            let added = FPDF_GetPageCount(src_doc);
            let ok = FPDF_ImportPages(self.doc, src_doc, null(), insert_idx);
            FPDF_CloseDocument(src_doc);
            if ok != 0 {
                for overlay in &mut self.overlays { if overlay.page_index>=insert_idx { overlay.page_index+=added; } }
                self.overlays.extend(annotations::take_managed_overlays(self.doc));
                self.page_count = FPDF_GetPageCount(self.doc);
                self.clear_thumbnail_cache();
                self.status_msg = Some(format!("PDF 병합/삽입 완료! (총 {}페이지)", self.page_count));
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
        let Some(save_path) = save_custom_dialog(hwnd, &default_name, "PDF 문서 (*.pdf)\0*.pdf\0\0", "pdf") else { return };
        unsafe {
            let new_doc = FPDF_CreateNewDocument();
            if new_doc.is_null() {
                return;
            }
            let range_str = CString::new(format!("{}", self.current_page + 1)).unwrap();
            if FPDF_ImportPages(new_doc, self.doc, range_str.as_ptr(), 0) != 0 {
                let mut ctx = FileWriteContext {
                    fw: FPDF_FILEWRITE { version: 1, WriteBlock: Some(write_block_cb) },
                    buf: Vec::new(),
                };
                if FPDF_SaveAsCopy(new_doc, &mut ctx.fw, 0) != 0 && !ctx.buf.is_empty() {
                    let _ = fs::write(&save_path, &ctx.buf);
                    self.status_msg = Some(format!("{}페이지 추출 저장 완료: {}", self.current_page + 1, save_path));
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
                for ov in self.overlays.iter().filter(|o| o.page_index == self.current_page && o.kind == OverlayKind::Redaction) {
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
                for ov in self.overlays.iter().filter(|o| o.page_index == self.current_page) {
                    if matches!(ov.kind, OverlayKind::FormText | OverlayKind::FormDropdown) && !ov.label.is_empty() {
                        let tx = (ov.x1.min(ov.x2) as f64) * (w as f64) + 4.0;
                        let ty = (1.0 - ov.y1.max(ov.y2) as f64) * (h as f64) + 6.0;
                        Self::insert_pdf_text_object(self.doc, page, &ov.label, tx, ty, 11.0);
                    }
                }
                let dll_name = to_wstring("pdfium.dll");
                let hmod = GetModuleHandleW(dll_name.as_ptr());
                if !hmod.is_null() {
                    type NewImgObjFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
                    type SetBmpFn = unsafe extern "C" fn(*mut *mut c_void, c_int, *mut c_void, *mut c_void) -> c_int;
                    let s_new = CString::new("FPDFPageObj_NewImageObj").unwrap();
                    let s_set = CString::new("FPDFImageObj_SetBitmap").unwrap();
                    if let (Some(f_new), Some(f_set)) = (
                        GetProcAddress(hmod, s_new.as_ptr() as *const u8),
                        GetProcAddress(hmod, s_set.as_ptr() as *const u8),
                    ) {
                        let new_img_obj: NewImgObjFn = std::mem::transmute(f_new);
                        let set_bmp: SetBmpFn = std::mem::transmute(f_set);
                        for ov in self.overlays.iter().filter(|o| o.page_index == self.current_page) {
                            if let OverlayKind::SignatureImage(sig_idx) = ov.kind {
                                if let Some(sig) = self.signatures.get(sig_idx) {
                                    let fpdf_bmp = FPDFBitmap_Create(sig.width, sig.height, 1);
                                    if !fpdf_bmp.is_null() {
                                        let dst_buf = FPDFBitmap_GetBuffer(fpdf_bmp);
                                        let stride = FPDFBitmap_GetStride(fpdf_bmp) as usize;
                                        let row_bytes = (sig.width * 4) as usize;
                                        for y in 0..(sig.height as usize) {
                                            let src_ptr = sig.raw_bgra.as_ptr().add(y * row_bytes);
                                            let dst_ptr = dst_buf.add(y * stride);
                                            copy_nonoverlapping(src_ptr, dst_ptr, row_bytes);
                                        }
                                        let img_obj = new_img_obj(self.doc);
                                        if !img_obj.is_null() {
                                            let mut p_page = page;
                                            set_bmp(&mut p_page, 1, img_obj, fpdf_bmp);
                                            let rx = (ov.x1.min(ov.x2) as f64) * (w as f64);
                                            let rw = ((ov.x2 - ov.x1).abs() as f64 * (w as f64)).max(24.0);
                                            let rh = ((ov.y2 - ov.y1).abs() as f64 * (h as f64)).max(18.0);
                                            let ry = (1.0 - ov.y1.max(ov.y2) as f64) * (h as f64);
                                            FPDFPageObj_Transform(img_obj, rw, 0.0, 0.0, rh, rx, ry);
                                            FPDFPage_InsertObject(page, img_obj);
                                        }
                                        FPDFBitmap_Destroy(fpdf_bmp);
                                    }
                                }
                            }
                        }
                    }
                }
                FPDFPage_GenerateContent(page);
                FPDFPage_Flatten(page, 0);
                FPDF_ClosePage(page);
                self.thumb_cache.remove(&self.current_page);
                self.status_msg = Some("현재 페이지 주석/양식/서명 평면화(Flatten) 완료".to_string());
                self.refresh_view(hwnd, true);
            }
        }
    }

    fn compare_with_other_pdf(&mut self, hwnd: HWND) {
        if self.doc.is_null() {
            return;
        }
        let Some(other_path) = open_file_dialog(hwnd) else { return };
        let Ok(bytes) = fs::read(&other_path) else { return };
        unsafe {
            let doc2 = FPDF_LoadMemDocument(bytes.as_ptr() as *const c_void, bytes.len() as c_int, null());
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
                "[PDF 문서 비교 결과]\n\n• 기준 문서: {} ({} 페이지)\n• 비교 문서: {} ({} 페이지)\n• 텍스트 차이 감지 페이지 수: {}개\n• 차이 발생 페이지: {:?}",
                self.file_name, self.page_count, other_path, count2, diff_pages.len(), diff_pages
            );
            let w_rep = to_wstring(&report);
            let w_cap = to_wstring("DaPDF - PDF 비교 결과 (VS)");
            MessageBoxW(hwnd, w_rep.as_ptr(), w_cap.as_ptr(), MB_OK | MB_ICONINFORMATION);
        }
    }

    fn detect_and_copy_tables(&mut self, hwnd: HWND) {
        let Some(txt) = self.extract_page_text_at(self.current_page) else { return };
        let mut table_rows = Vec::new();
        for line in txt.lines() {
            if line.contains('|') || line.contains('\t') || line.contains("  ") {
                let cols: Vec<&str> = line.split(|c| c == '|' || c == '\t').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
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
                table_rows.iter().take(10).cloned().collect::<Vec<_>>().join("\n")
            )
        };
        let w_msg = to_wstring(&msg);
        let w_cap = to_wstring("DaPDF - 표 인식 결과");
        unsafe { MessageBoxW(hwnd, w_msg.as_ptr(), w_cap.as_ptr(), MB_OK | MB_ICONINFORMATION) };
    }

    fn export_document_as(&mut self, hwnd: HWND, ext: &str) {
        if self.doc.is_null() || self.page_count <= 0 {
            return;
        }
        let stem = Path::new(&self.file_name).file_stem().and_then(|s| s.to_str()).unwrap_or("document");
        let default_name = format!("{}.{}", stem, ext);
        let filter = match ext {
            "html" => "HTML 웹 문서 (*.html)\0*.html\0\0",
            "rtf" => "Word 호환 서식 문서 (*.rtf)\0*.rtf\0\0",
            "csv" => "Excel CSV 데이터 (*.csv)\0*.csv\0\0",
            _ => "텍스트 문서 (*.txt)\0*.txt\0\0",
        };
        let Some(save_path) = save_custom_dialog(hwnd, &default_name, filter, ext) else { return };
        let mut all_pages = Vec::new();
        for p in 0..self.page_count {
            if let Some(t) = self.extract_page_text_at(p) {
                all_pages.push((p + 1, t));
            }
        }
        let output = match ext {
            "html" => {
                let mut h = format!(
                    "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body>",
                    self.file_name
                );
                for (pno, txt) in &all_pages {
                    h.push_str(&format!("<div><h3>Page {}</h3><pre>{}</pre></div>", pno, txt.replace('&', "&amp;").replace('<', "&lt;")));
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
                        c.push_str(&format!("{},{},\"{}\"\r\n", pno, lno + 1, line.replace('"', "\"\"")));
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
            self.status_msg = Some(format!("{} 포맷 변환 완료: {}", ext.to_uppercase(), save_path));
        }
        update_window_title(hwnd, self);
    }

    fn get_meta_text(&self, tag: &str) -> String {
        if self.doc.is_null() {
            return String::new();
        }
        let Ok(c_tag) = CString::new(tag) else { return String::new() };
        unsafe {
            let byte_len = FPDF_GetMetaText(self.doc, c_tag.as_ptr(), null_mut(), 0);
            if byte_len <= 2 {
                return String::new();
            }
            let u16_len = (byte_len as usize) / 2;
            let mut buf = vec![0u16; u16_len];
            FPDF_GetMetaText(self.doc, c_tag.as_ptr(), buf.as_mut_ptr() as *mut c_void, byte_len);
            let actual_len = buf.iter().position(|&c| c == 0).unwrap_or(u16_len);
            String::from_utf16_lossy(&buf[..actual_len]).trim().to_string()
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
            let info_text = format!(
                "파일 이름: {}\n파일 크기: {:.1} KB\nPDF 버전: PDF {}\n총 페이지 수: {} 페이지\n현재 페이지 크기: {:.1} x {:.1} pt ({:.0} x {:.0} mm)\n제목: {}\n저자: {}",
                self.file_name,
                (self.pdf_bytes.len() as f64) / 1024.0,
                ver_str,
                self.page_count,
                self.cur_page_w_pt,
                self.cur_page_h_pt,
                w_mm,
                h_mm,
                self.get_meta_text("Title"),
                self.get_meta_text("Author"),
            );
            let w_info = to_wstring(&info_text);
            let w_caption = to_wstring(&format!("DaPDF(v{}) - 문서 메타데이터 정보", APP_VERSION));
            MessageBoxW(hwnd, w_info.as_ptr(), w_caption.as_ptr(), MB_OK | MB_ICONINFORMATION);
        }
    }

    fn show_help_dialog(&self, hwnd: HWND) {
        let help_text = "\
[서명 및 인장(도장) 이미지 날인 ([✒ 서명] 도구)]
• 상단 툴바의 [✒ 서명] 버튼 또는 [주석 > 서명/인장 이미지 업로드]로 .png/.jpg/.bmp 파일을 불러오면 흰 배경이 자동 투명화됩니다.
• 본문 위를 드래그하여 원하는 크기로 날인하고, 날인 후에도 드래그로 위치 이동 및 우측 하단(■) 핸들로 크기 조절이 가능합니다.

[인터랙티브 양식(Form) 직접 타이핑]
• [양식 > 텍스트 필드 추가] 후 클릭하여 키보드로 직접 타이핑(Tab: 다음 필드, Enter: 확정)할 수 있습니다.

[텍스트 드래그 선택 & 우클릭 팝업 메뉴]
• [🔤 선택] 도구로 텍스트를 드래그한 뒤 우클릭하면 [복사(Ctrl+C)], [형광펜 강조 추가], [문서에서 이 단어 검색] 메뉴가 실행됩니다.";
        let w_help = to_wstring(help_text);
        let w_caption = to_wstring(&format!("DaPDF(v{}) Viewer - 도움말", APP_VERSION));
        unsafe { MessageBoxW(hwnd, w_help.as_ptr(), w_caption.as_ptr(), MB_OK | MB_ICONINFORMATION) };
    }

    fn load_bookmarks(&mut self) {
        self.bookmarks.clear();
        if self.doc.is_null() {
            return;
        }
        unsafe { Self::collect_bookmarks_recursive(self.doc, null_mut(), 0, &mut self.bookmarks) };
    }

    unsafe fn collect_bookmarks_recursive(doc: *mut c_void, parent: *mut c_void, depth: usize, out: &mut Vec<BookmarkItem>) {
        let mut bm = FPDFBookmark_GetFirstChild(doc, parent);
        while !bm.is_null() {
            let byte_len = FPDFBookmark_GetTitle(bm, null_mut(), 0);
            let mut title = String::from("(제목 없음)");
            if byte_len >= 2 {
                let u16_len = (byte_len as usize) / 2;
                let mut buf = vec![0u16; u16_len];
                FPDFBookmark_GetTitle(bm, buf.as_mut_ptr() as *mut c_void, byte_len);
                let actual_len = buf.iter().position(|&c| c == 0).unwrap_or(u16_len);
                let decoded = String::from_utf16_lossy(&buf[..actual_len]).trim().to_string();
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
            let page_index = if !dest.is_null() { FPDFDest_GetDestPageIndex(doc, dest) } else { -1 };
            out.push(BookmarkItem { title, page_index, depth });
            Self::collect_bookmarks_recursive(doc, bm, depth + 1, out);
            bm = FPDFBookmark_GetNextSibling(doc, bm);
        }
    }

    fn collect_matches_on_page(&self, page_idx: i32) -> Vec<SearchMatch> {
        let mut matches = Vec::new();
        if self.doc.is_null() || page_idx < 0 || page_idx >= self.page_count || self.search_query.is_empty() {
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
                                let (mut left, mut top, mut right, mut bottom) = (0.0, 0.0, 0.0, 0.0);
                                if FPDFText_GetRect(text_page, r, &mut left, &mut top, &mut right, &mut bottom) != 0 {
                                    rects.push(PdfRect { left, top, right, bottom });
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
                self.status_msg = Some(format!("'{}' 검색 결과 {}건 교정(검은색 마스킹) 완료", self.search_query, count));
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
                self.refresh_view(hwnd, true);
                return;
            } else if !forward && self.active_match > 0 {
                self.active_match -= 1;
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
                self.current_page = next_page;
                self.pan_x = 0;
                self.pan_y = 0;
                self.pending_align_top = self.fit_width;
                self.active_match = if forward { 0 } else { matches.len() - 1 };
                self.search_matches = matches;
                self.refresh_view(hwnd, true);
                return;
            }
        }
    }

    fn sidebar_offset(&self) -> i32 {
        if self.show_sidebar { self.sidebar_width } else { 0 }
    }

    fn align_vertical_pan_if_needed(&mut self, viewport_h: i32) {
        if self.pending_align_top {
            self.pending_align_top = false;
            self.pan_y = if self.bmp_height > viewport_h - 24 { (self.bmp_height - viewport_h) / 2 + 12 } else { 0 };
        } else if self.pending_align_bottom {
            self.pending_align_bottom = false;
            self.pan_y = if self.bmp_height > viewport_h - 24 { -((self.bmp_height - viewport_h) / 2 + 12) } else { 0 };
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
            let (w1, h1) = if self.rotation % 2 == 1 { (raw_h1, raw_w1) } else { (raw_w1, raw_h1) };

            let show_second = self.spread_mode
                && !(self.cover_mode && self.current_page == 0)
                && (self.current_page + 1 < self.page_count);
            let page2 = if show_second { FPDF_LoadPage(self.doc, self.current_page + 1) } else { null_mut() };
            let (w2, h2) = if !page2.is_null() {
                let rw2 = FPDF_GetPageWidth(page2);
                let rh2 = FPDF_GetPageHeight(page2);
                if self.rotation % 2 == 1 { (rh2, rw2) } else { (rw2, rh2) }
            } else {
                (0.0, 0.0)
            };

            let gap = if !page2.is_null() { SPREAD_GAP } else { 0 };
            let avail_w = (canvas_w - 32 - gap).max(50) as f64;
            let avail_h = (viewport_h - 24).max(50) as f64;
            let total_pdf_w = w1 + w2;
            let max_pdf_h = h1.max(h2);
            let base_scale = if self.fit_width { avail_w / total_pdf_w } else { (avail_w / total_pdf_w).min(avail_h / max_pdf_h) };
            let final_scale = base_scale * self.zoom;

            let target_w1 = ((w1 * final_scale) as i32).clamp(1, 4096);
            let target_h1 = ((h1 * final_scale) as i32).clamp(1, 4096);
            let (target_w2, target_h2) = if !page2.is_null() {
                (((w2 * final_scale) as i32).clamp(1, 4096), ((h2 * final_scale) as i32).clamp(1, 4096))
            } else {
                (0, 0)
            };

            let total_w = if !page2.is_null() { target_w1 + gap + target_w2 } else { target_w1 };
            let total_h = target_h1.max(target_h2);
            let y_off1 = (total_h - target_h1) / 2;
            self.p1_rect_in_bmp = RECT { left: 0, top: y_off1, right: target_w1, bottom: y_off1 + target_h1 };

            if !page2.is_null() {
                let x_off2 = target_w1 + gap;
                let y_off2 = (total_h - target_h2) / 2;
                self.p2_rect_in_bmp = Some(RECT { left: x_off2, top: y_off2, right: x_off2 + target_w2, bottom: y_off2 + target_h2 });
            } else {
                self.p2_rect_in_bmp = None;
            }

            if self.bmp_width == total_w && self.bmp_height == total_h && !self.pixels.is_empty() {
                self.align_vertical_pan_if_needed(viewport_h);
                if !page2.is_null() { FPDF_ClosePage(page2); }
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
            self.draw_highlights_for_page(page1, 0, y_off1, target_w1, target_h1, &matches_p1, Some(self.active_match));

            if !page2.is_null() {
                let x_off2 = target_w1 + gap;
                let y_off2 = (total_h - target_h2) / 2;
                self.render_single_page_into_buffer(page2, x_off2, y_off2, target_w2, target_h2);
                if !self.search_query.is_empty() {
                    let matches_p2 = self.collect_matches_on_page(self.current_page + 1);
                    self.draw_highlights_for_page(page2, x_off2, y_off2, target_w2, target_h2, &matches_p2, None);
                }
                FPDF_ClosePage(page2);
            }
            FPDF_ClosePage(page1);
        }
    }

    unsafe fn render_single_page_into_buffer(&mut self, page: *mut c_void, dest_x: i32, dest_y: i32, w: i32, h: i32) {
        let bitmap = FPDFBitmap_Create(w, h, 1);
        if !bitmap.is_null() {
            let fill_color = if let Some((r, g, b)) = self.page_bg_tint {
                0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
            } else {
                0xFFFFFFFF
            };
            FPDFBitmap_FillRect(bitmap, 0, 0, w, h, fill_color);
            let render_flags = if self.show_annotations { FPDF_ANNOT | FPDF_LCD_TEXT } else { FPDF_LCD_TEXT };
            FPDF_RenderPageBitmap(bitmap, page, 0, 0, w, h, self.rotation, render_flags);

            let buffer = FPDFBitmap_GetBuffer(bitmap);
            let stride = FPDFBitmap_GetStride(bitmap) as usize;
            let copy_bytes = (w * 4) as usize;
            let total_w = self.bmp_width as usize;

            for y in 0..(h as usize) {
                let src_row = buffer.add(y * stride);
                let dst_offset = ((dest_y as usize + y) * total_w + (dest_x as usize)) * 4;
                let dst_slice = std::slice::from_raw_parts_mut(self.pixels.as_mut_ptr().add(dst_offset), copy_bytes);
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
            let (hl_r, hl_g, hl_b, alpha) = if is_active { (255u32, 130u32, 0u32, 125u32) } else { (255u32, 235u32, 0u32, 100u32) };
            let inv_alpha = 255 - alpha;
            for rect in &m.rects {
                let (mut dev_x1, mut dev_y1, mut dev_x2, mut dev_y2) = (0, 0, 0, 0);
                FPDF_PageToDevice(page, 0, 0, page_w, page_h, self.rotation, rect.left, rect.top, &mut dev_x1, &mut dev_y1);
                FPDF_PageToDevice(page, 0, 0, page_w, page_h, self.rotation, rect.right, rect.bottom, &mut dev_x2, &mut dev_y2);
                let x_min = (dev_x1.min(dev_x2) - 2).clamp(0, page_w - 1) + offset_x;
                let x_max = (dev_x1.max(dev_x2) + 2).clamp(0, page_w - 1) + offset_x;
                let y_min = (dev_y1.min(dev_y2) - 2).clamp(0, page_h - 1) + offset_y;
                let y_max = (dev_y1.max(dev_y2) + 2).clamp(0, page_h - 1) + offset_y;
                for y in y_min..=y_max {
                    if y < 0 || y >= total_h { continue; }
                    let row_offset = (y * total_w * 4) as usize;
                    for x in x_min..=x_max {
                        if x < 0 || x >= total_w { continue; }
                        let p = row_offset + (x * 4) as usize;
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

    fn extract_page_text_at(&self, page_idx: i32) -> Option<String> {
        if self.doc.is_null() || page_idx < 0 || page_idx >= self.page_count {
            return None;
        }
        unsafe {
            let page = FPDF_LoadPage(self.doc, page_idx);
            if page.is_null() { return None; }
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
                    extracted = String::from_utf16_lossy(&buf[..(written as usize).saturating_sub(1)]);
                }
            }
            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
            Some(extracted)
        }
    }

    fn save_current_page_image(&mut self, hwnd: HWND) {
        if self.doc.is_null() || self.pixels.is_empty() || self.bmp_width <= 0 || self.bmp_height <= 0 {
            return;
        }
        let stem = Path::new(&self.file_name).file_stem().and_then(|s| s.to_str()).unwrap_or("page");
        let default_name = format!("{}_page_{}.bmp", stem, self.current_page + 1);
        if let Some(save_path) = save_custom_dialog(hwnd, &default_name, "BMP 비트맵 이미지 (*.bmp)\0*.bmp\0\0", "bmp") {
            let _ = write_bgra_to_bmp_file(&save_path, self.bmp_width, self.bmp_height, &self.pixels);
            self.status_msg = Some(format!("이미지 저장 완료: {}", save_path));
            update_window_title(hwnd, self);
        }
    }

    fn refresh_view(&mut self, hwnd: HWND, force_rerender: bool) {
        unsafe { self.update_toolbar_layout(hwnd); }
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
            self.text_selection = None;
            self.status_msg = None;
            if !self.search_query.is_empty() {
                self.perform_search(false);
            }
            self.refresh_view(hwnd, true);
        }
    }

    fn go_to_first_page(&mut self, hwnd: HWND) {
        self.go_to_page(0, hwnd);
    }

    fn go_to_last_page(&mut self, hwnd: HWND) {
        if self.page_count > 0 {
            self.go_to_page(self.page_count - 1, hwnd);
        }
    }

    fn scroll_sidebar(&mut self, wheel_delta: i16, hwnd: HWND) {
        unsafe {
            let mut rc: RECT = zeroed();
            GetClientRect(hwnd, &mut rc);
            let avail_h = (rc.bottom - rc.top) - self.top_bar_offset() - self.bottom_bar_offset();
            match self.sidebar_tab {
                SidebarTab::Search => {},
                SidebarTab::Bookmarks => {
                    let max_rows = ((avail_h - SIDEBAR_TAB_HEIGHT) / BOOKMARK_ROW_HEIGHT).max(1) as usize;
                    let max_scroll = self.bookmarks.len().saturating_sub(max_rows);
                    if wheel_delta < 0 {
                        self.bookmark_scroll = (self.bookmark_scroll + 3).min(max_scroll);
                    } else if wheel_delta > 0 {
                        self.bookmark_scroll = self.bookmark_scroll.saturating_sub(3);
                    }
                }
                SidebarTab::Thumbnails => {
                    let max_rows = ((avail_h - SIDEBAR_TAB_HEIGHT) / THUMB_ITEM_HEIGHT).max(1) as usize;
                    let max_scroll = (self.page_count.max(0) as usize).saturating_sub(max_rows);
                    if wheel_delta < 0 {
                        self.thumb_scroll = (self.thumb_scroll + 1).min(max_scroll);
                    } else if wheel_delta > 0 {
                        self.thumb_scroll = self.thumb_scroll.saturating_sub(1);
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
        let viewport_h = (rc.bottom - rc.top) - self.top_bar_offset() - self.bottom_bar_offset();
        if self.bmp_height > viewport_h - 16 {
            let max_pan = (self.bmp_height - viewport_h) / 2 + 12;
            let min_pan = -max_pan;
            if scroll_down && self.pan_y > min_pan {
                self.pan_y = (self.pan_y - 110).max(min_pan);
                unsafe { InvalidateRect(hwnd, null(), 0) };
                return;
            } else if !scroll_down && self.pan_y < max_pan {
                self.pan_y = (self.pan_y + 110).min(max_pan);
                unsafe { InvalidateRect(hwnd, null(), 0) };
                return;
            }
        }
        self.change_page(if scroll_down { 1 } else { -1 }, hwnd);
    }

    fn toggle_fit_width(&mut self, hwnd: HWND) {
        self.fit_width = !self.fit_width;
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.pending_align_top = self.fit_width;
        self.refresh_view(hwnd, true);
    }

    fn toggle_spread_mode(&mut self, hwnd: HWND) {
        self.spread_mode = !self.spread_mode;
        if self.spread_mode && self.current_page % 2 == 1 {
            self.current_page = (self.current_page - 1).max(0);
        }
        self.pan_x = 0;
        self.pan_y = 0;
        self.refresh_view(hwnd, true);
    }

    fn toggle_dark_mode(&mut self, hwnd: HWND) {
        self.dark_mode = !self.dark_mode;
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
                    SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !WS_OVERLAPPEDWINDOW) as isize);
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
                SetWindowLongPtrW(hwnd, GWL_STYLE, (style | WS_OVERLAPPEDWINDOW) as isize);
                SetWindowPlacement(hwnd, &self.saved_placement);
                SetWindowPos(hwnd, null_mut(), 0, 0, 0, 0, SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED | 0x0003);
                self.is_fullscreen = false;
            }
            self.refresh_view(hwnd, true);
        }
    }

    fn step_bookmark(&mut self, forward: bool, hwnd: HWND) {
        if forward {
            if let Some(bm) = self.bookmarks.iter().find(|b| b.page_index > self.current_page) {
                self.go_to_page(bm.page_index, hwnd);
            }
        } else if let Some(bm) = self.bookmarks.iter().rev().find(|b| b.page_index >= 0 && b.page_index < self.current_page) {
            self.go_to_page(bm.page_index, hwnd);
        }
    }

    fn rotate_page(&mut self, clockwise: bool, hwnd: HWND) {
        self.rotation = if clockwise { (self.rotation + 1) % 4 } else { (self.rotation + 3) % 4 };
        self.clear_thumbnail_cache();
        self.pan_x = 0;
        self.pan_y = 0;
        self.refresh_view(hwnd, true);
    }

    fn change_zoom(&mut self, factor: f64, hwnd: HWND) {
        let next_zoom = (self.zoom * factor).clamp(0.25, 5.0);
        if (next_zoom - self.zoom).abs() > 0.001 {
            let ratio = next_zoom / self.zoom;
            self.pan_x = (self.pan_x as f64 * ratio) as i32;
            self.pan_y = (self.pan_y as f64 * ratio) as i32;
            self.zoom = next_zoom;
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
        self.refresh_view(hwnd, true);
    }

    fn copy_text_to_clipboard(&mut self, hwnd: HWND) {
        if self.doc.is_null() {
            return;
        }
        if let Some(ref sel) = self.text_selection {
            if !sel.text.is_empty() {
                let normalized = sel.text.replace("\r\n", "\n").replace('\n', "\r\n");
                let char_len = normalized.chars().count();
                if set_clipboard_unicode_text(hwnd, &normalized) {
                    self.status_msg = Some(format!("선택 텍스트 복사 완료 ({}자)", char_len));
                }
                self.refresh_view(hwnd, false);
                return;
            }
        }
        let combined = self.extract_page_text_at(self.current_page).unwrap_or_default();
        if !combined.trim().is_empty() {
            let normalized = combined.replace("\r\n", "\n").replace('\n', "\r\n");
            if set_clipboard_unicode_text(hwnd, &normalized) {
                self.status_msg = Some(format!("페이지 전체 텍스트 복사 완료 ({}자)", normalized.chars().count()));
            }
        }
        self.refresh_view(hwnd, false);
    }

    fn is_toolbar_btn_active(&self, id: u16) -> bool {
        match id {
            IDM_VIEW_FIT_WIDTH => self.fit_width,
            IDM_VIEW_SPREAD => self.spread_mode,
            IDM_THEME_DARK_DOC => self.dark_mode,
            IDM_VIEW_BOOKMARKS => self.show_sidebar && self.sidebar_tab == SidebarTab::Bookmarks,
            IDM_VIEW_THUMBNAILS => self.show_sidebar && self.sidebar_tab == SidebarTab::Thumbnails,
            IDM_ANNOT_NONE => self.active_tool == ActiveTool::Pan,
            IDM_ANNOT_TEXT_SELECT => self.active_tool == ActiveTool::TextSelect,
            IDM_ANNOT_HIGHLIGHT => self.active_tool == ActiveTool::Highlight,
            IDM_ANNOT_PENCIL => self.active_tool == ActiveTool::Pencil,
            IDM_SEC_REDACT_TOOL => self.active_tool == ActiveTool::RedactBox,
            IDM_SIGN_UPLOAD_IMAGE => self.active_tool == ActiveTool::PlaceSignature,
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

unsafe fn load_signature_image_from_file(path: &str) -> Option<SignatureStampData> {
    let mut token: usize = 0;
    let input = GdiplusStartupInput {
        GdiplusVersion: 1,
        DebugEventCallback: null(),
        SuppressBackgroundThread: 0,
        SuppressExternalCodecs: 0,
    };
    if GdiplusStartup(&mut token, &input, null_mut()) != 0 {
        return None;
    }
    let w_path = to_wstring(path);
    let mut gp_bmp: *mut c_void = null_mut();
    if GdipCreateBitmapFromFile(w_path.as_ptr(), &mut gp_bmp) != 0 || gp_bmp.is_null() {
        return None;
    }
    let (mut w, mut h) = (0u32, 0u32);
    GdipGetImageWidth(gp_bmp, &mut w);
    GdipGetImageHeight(gp_bmp, &mut h);
    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        GdipDisposeImage(gp_bmp);
        return None;
    }
    let rect = GpRect { X: 0, Y: 0, Width: w as i32, Height: h as i32 };
    let mut bmp_data: BitmapData = zeroed();
    if GdipBitmapLockBits(gp_bmp, &rect, 1, 0x26200A, &mut bmp_data) != 0 {
        GdipDisposeImage(gp_bmp);
        return None;
    }
    let width = w as i32;
    let height = h as i32;
    let mut raw_bgra = vec![0u8; (width * height * 4) as usize];
    let mut pm_bgra = vec![0u8; (width * height * 4) as usize];

    for y in 0..(height as usize) {
        let src_row = if bmp_data.Stride >= 0 {
            bmp_data.Scan0.add(y * (bmp_data.Stride as usize))
        } else {
            bmp_data.Scan0.offset((y as isize) * (bmp_data.Stride as isize))
        };
        for x in 0..(width as usize) {
            let p = src_row.add(x * 4);
            let b = *p;
            let g = *p.add(1);
            let r = *p.add(2);
            let mut a = *p.add(3);
            if a > 200 && r >= 242 && g >= 242 && b >= 242 {
                a = 0;
            } else if a > 200 && r >= 220 && g >= 220 && b >= 220 {
                let lum = ((r as u32 + g as u32 + b as u32) / 3).saturating_sub(200);
                a = ((255u32.saturating_sub(lum * 4)).min(255)) as u8;
            }
            let dst_idx = (y * (width as usize) + x) * 4;
            raw_bgra[dst_idx] = b;
            raw_bgra[dst_idx + 1] = g;
            raw_bgra[dst_idx + 2] = r;
            raw_bgra[dst_idx + 3] = a;
            let af = a as u32;
            pm_bgra[dst_idx] = ((b as u32 * af) / 255) as u8;
            pm_bgra[dst_idx + 1] = ((g as u32 * af) / 255) as u8;
            pm_bgra[dst_idx + 2] = ((r as u32 * af) / 255) as u8;
            pm_bgra[dst_idx + 3] = a;
        }
    }
    GdipBitmapUnlockBits(gp_bmp, &mut bmp_data);
    GdipDisposeImage(gp_bmp);
    Some(SignatureStampData { width, height, pm_bgra, raw_bgra })
}

unsafe fn create_red_seal_stamp(hwnd: HWND, text: &str) -> Option<SignatureStampData> {
    let w = 180;
    let h = 180;
    let screen_dc = GetDC(hwnd);
    let mem_dc = CreateCompatibleDC(screen_dc);
    let hbm = CreateCompatibleBitmap(screen_dc, w, h);
    let old_bm = SelectObject(mem_dc, hbm as _);
    let bg_rc = RECT { left: 0, top: 0, right: w, bottom: h };
    let white_br = CreateSolidBrush(0x00FFFFFF);
    FillRect(mem_dc, &bg_rc, white_br);
    DeleteObject(white_br as _);

    let red_pen = CreatePen(PS_SOLID, 5, 0x002222DC);
    let old_pen = SelectObject(mem_dc, red_pen as _);
    let null_br = GetStockObject(5);
    let old_br = SelectObject(mem_dc, null_br);
    Rectangle(mem_dc, 12, 12, w - 12, h - 12);
    Rectangle(mem_dc, 20, 20, w - 20, h - 20);

    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(mem_dc, font);
    SetBkMode(mem_dc, TRANSPARENT as i32);
    SetTextColor(mem_dc, 0x002222DC);

    let seal_title = format!("【 {} 】", text.trim());
    let w_t1: Vec<u16> = seal_title.encode_utf16().collect();
    TextOutW(mem_dc, 34, 62, w_t1.as_ptr(), w_t1.len() as i32);
    let sub_lbl = "전자서명인장 (印)";
    let w_t2: Vec<u16> = sub_lbl.encode_utf16().collect();
    TextOutW(mem_dc, 38, 96, w_t2.as_ptr(), w_t2.len() as i32);

    let mut bmi: BITMAPINFO = zeroed();
    bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = w;
    bmi.bmiHeader.biHeight = -h;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    let mut buf = vec![0u8; (w * h * 4) as usize];
    GetDIBits(mem_dc, hbm, 0, h as u32, buf.as_mut_ptr() as *mut c_void, &mut bmi, DIB_RGB_COLORS);

    SelectObject(mem_dc, old_font);
    SelectObject(mem_dc, old_br);
    SelectObject(mem_dc, old_pen);
    DeleteObject(red_pen as _);
    SelectObject(mem_dc, old_bm);
    DeleteObject(hbm as _);
    DeleteDC(mem_dc);
    ReleaseDC(hwnd, screen_dc);

    let mut raw_bgra = vec![0u8; (w * h * 4) as usize];
    let mut pm_bgra = vec![0u8; (w * h * 4) as usize];
    for i in (0..buf.len()).step_by(4) {
        let b = buf[i];
        let g = buf[i + 1];
        let r = buf[i + 2];
        let a = if r > 240 && g > 240 && b > 240 { 0u8 } else { 235u8 };
        raw_bgra[i] = b;
        raw_bgra[i + 1] = g;
        raw_bgra[i + 2] = r;
        raw_bgra[i + 3] = a;
        let af = a as u32;
        pm_bgra[i] = ((b as u32 * af) / 255) as u8;
        pm_bgra[i + 1] = ((g as u32 * af) / 255) as u8;
        pm_bgra[i + 2] = ((r as u32 * af) / 255) as u8;
        pm_bgra[i + 3] = a;
    }
    Some(SignatureStampData { width: w, height: h, pm_bgra, raw_bgra })
}

fn open_image_file_dialog(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut file_buf = vec![0u16; 1024];
        let filter: Vec<u16> = "서명/인장 이미지 (*.png;*.jpg;*.jpeg;*.bmp)\0*.png;*.jpg;*.jpeg;*.bmp\0모든 파일 (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();
        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.Flags = OFN_EXPLORER | OFN_ENABLESIZING | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
        if GetOpenFileNameW(&mut ofn) != 0 {
            let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
            Some(String::from_utf16_lossy(&file_buf[..len]))
        } else {
            None
        }
    }
}

fn open_file_dialog(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut file_buf = vec![0u16; 1024];
        let filter: Vec<u16> = "PDF 문서 (*.pdf)\0*.pdf\0모든 파일 (*.*)\0*.*\0\0".encode_utf16().collect();
        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.Flags = OFN_EXPLORER | OFN_ENABLESIZING | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
        if GetOpenFileNameW(&mut ofn) != 0 {
            let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
            Some(String::from_utf16_lossy(&file_buf[..len]))
        } else {
            None
        }
    }
}

unsafe fn draw_left_sidebar_panel(hdc: *mut c_void, client_h: i32, state: &mut AppState) {
    let top_off = state.top_bar_offset();
    let bot_off = state.bottom_bar_offset();
    let panel_bottom = client_h - bot_off;
    let panel_rc = RECT { left: 0, top: top_off, right: state.sidebar_width, bottom: panel_bottom };
    let panel_brush = CreateSolidBrush(0x00252526);
    FillRect(hdc, &panel_rc, panel_brush);
    DeleteObject(panel_brush as _);
    let divider = RECT { left: state.sidebar_width - 3, top: top_off, right: state.sidebar_width, bottom: panel_bottom };
    let divider_brush = CreateSolidBrush(0x00555555);
    FillRect(hdc, &divider, divider_brush); DeleteObject(divider_brush as _);

    let tab_bar_rc = RECT { left: 0, top: top_off, right: state.sidebar_width, bottom: top_off + SIDEBAR_TAB_HEIGHT };
    let tab_bar_brush = CreateSolidBrush(0x001B1B1C);
    FillRect(hdc, &tab_bar_rc, tab_bar_brush);
    DeleteObject(tab_bar_brush as _);

    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT as i32);

    SetTextColor(hdc, if state.sidebar_tab == SidebarTab::Thumbnails { 0x0080D8FF } else { 0x00AAAAAA });
    let t1_lbl: Vec<u16> = format!("썸네일 ({})", state.page_count).encode_utf16().collect();
    TextOutW(hdc, 14, top_off + 10, t1_lbl.as_ptr(), t1_lbl.len() as i32);

    SetTextColor(hdc, if state.sidebar_tab == SidebarTab::Search { 0x0080D8FF } else { 0x00AAAAAA });
    let t2_lbl: Vec<u16> = "검색".encode_utf16().collect();
    TextOutW(hdc, 134, top_off + 10, t2_lbl.as_ptr(), t2_lbl.len() as i32);

    SetTextColor(hdc, 0x00999999);
    let close_lbl: Vec<u16> = "✕".encode_utf16().collect();
    TextOutW(hdc, state.sidebar_width - 24, top_off + 10, close_lbl.as_ptr(), close_lbl.len() as i32);

    let content_top = top_off + SIDEBAR_TAB_HEIGHT + 4;
    let avail_h = (panel_bottom - content_top).max(26);

    match state.sidebar_tab {
        SidebarTab::Search => {
            let mut field = RECT { left: 12, top: content_top + 12, right: state.sidebar_width - 12, bottom: content_top + 48 };
            let brush = CreateSolidBrush(if state.input_mode == InputMode::Search { 0x00505050 } else { 0x00383838 });
            FillRect(hdc, &field, brush);
            DeleteObject(brush as _);
            SetTextColor(hdc, 0x00EEEEEE);
            let focused = state.input_mode == InputMode::Search;
            let label = if state.search_query.is_empty() && !focused { "검색어 입력...".to_string() }
                else { state.search_query.clone() };
            // Win32 drawing may inspect the buffer even when its character count
            // is zero. Keep a real NUL-terminated buffer for an empty search box.
            let label = to_wstring(&label);
            let label_len = (label.len() - 1) as i32;
            field.left += 8; field.right -= 8;
            DrawTextW(hdc, label.as_ptr(), label_len, &mut field, 0x20 | 0x4 | 0x8000);
            if focused && state.window_focused && state.search_caret_visible {
                let mut size: SIZE = zeroed();
                if label_len > 0 {
                    GetTextExtentPoint32W(hdc, label.as_ptr(), label_len, &mut size);
                }
                let x = (field.left + size.cx).min(field.right - 2);
                let caret = RECT { left: x, top: field.top + 7, right: x + 2, bottom: field.bottom - 7 };
                let brush = CreateSolidBrush(0x00FFFFFF);
                FillRect(hdc, &caret, brush); DeleteObject(brush as _);
            }
            let labels = [
                (12, content_top + 64, "◀ 이전     다음 ▶".to_string()),
                (12, content_top + 108, if state.search_query.is_empty() { "문서 전체에서 검색합니다".into() }
                    else if state.search_matches.is_empty() { "검색 결과 없음".into() }
                    else { format!("{}페이지 · {} / {}건", state.current_page + 1, state.active_match + 1, state.search_matches.len()) }),
                (12, content_top + 144, "Enter: 검색 / F3: 다음".into()),
                (12, content_top + 176, "Shift+F3: 이전 / Esc: 입력 종료".into()),
            ];
            for (x, y, label) in labels {
                if y + 20 <= panel_bottom {
                    let text: Vec<u16> = label.encode_utf16().collect();
                    let mut bounds = RECT { left: x, top: y, right: state.sidebar_width - 12, bottom: y + 24 };
                    DrawTextW(hdc, text.as_ptr(), text.len() as i32, &mut bounds, 0x20 | 0x8000);
                }
            }
        }
        SidebarTab::Bookmarks => {
            let max_rows = (avail_h / BOOKMARK_ROW_HEIGHT).max(1) as usize;
            for (i, bm) in state.bookmarks.iter().enumerate().skip(state.bookmark_scroll).take(max_rows) {
                let row_idx = i - state.bookmark_scroll;
                let y_top = content_top + (row_idx as i32) * BOOKMARK_ROW_HEIGHT;
                let is_current = bm.page_index == state.current_page;
                SetTextColor(hdc, if is_current { 0x0000EAFF } else { 0x00E6E6E6 });
                let indent = (bm.depth.min(5) as i32) * 12;
                let label = if bm.page_index >= 0 { format!("[p.{}] {}", bm.page_index + 1, bm.title) } else { bm.title.clone() };
                let w_label: Vec<u16> = label.encode_utf16().collect();
                TextOutW(hdc, 10 + indent, y_top + 5, w_label.as_ptr(), w_label.len() as i32);
            }
        }
        SidebarTab::Thumbnails => {
            let max_rows = (avail_h / THUMB_ITEM_HEIGHT).max(1) as usize;
            let total_pages = state.page_count.max(0) as usize;
            let start_idx = state.thumb_scroll;
            let end_idx = (start_idx + max_rows).min(total_pages);
            for (slot, page_u) in (start_idx..end_idx).enumerate() {
                let p_idx = page_u as i32;
                let item_top = content_top + (slot as i32) * THUMB_ITEM_HEIGHT + 4;
                let is_active = p_idx == state.current_page;
                if let Some(thumb) = state.get_or_render_thumbnail(p_idx) {
                    let tx = 16 + ((state.sidebar_width - 34) - thumb.width) / 2;
                    let ty = item_top + 6 + (176 - thumb.height) / 2;
                    let mut bmi: BITMAPINFO = zeroed();
                    bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
                    bmi.bmiHeader.biWidth = thumb.width;
                    bmi.bmiHeader.biHeight = -thumb.height;
                    bmi.bmiHeader.biPlanes = 1;
                    bmi.bmiHeader.biBitCount = 32;
                    bmi.bmiHeader.biCompression = BI_RGB;
                    StretchDIBits(
                        hdc, tx, ty, thumb.width, thumb.height, 0, 0, thumb.width, thumb.height,
                        thumb.pixels.as_ptr() as *const c_void, &bmi, DIB_RGB_COLORS, SRCCOPY,
                    );
                }
                SetTextColor(hdc, if is_active { 0x0000EAFF } else { 0x00DDDDDD });
                let w_p: Vec<u16> = format!("- {} -", p_idx + 1).encode_utf16().collect();
                TextOutW(hdc, state.sidebar_width / 2 - 20, item_top + THUMB_ITEM_HEIGHT - 28, w_p.as_ptr(), w_p.len() as i32);
            }
        }
    }
    SelectObject(hdc, old_font);
}

// Measure with the same font used to paint; one layout drives both drawing and clicks.
unsafe fn toolbar_layout(hdc: *mut c_void, client_w: i32, dpi: u32) -> (Vec<RECT>, i32) {
    let scale = |value: i32| ((value as i64 * dpi.max(96) as i64 + 48) / 96) as i32;
    let margin = scale(6);
    let gap = scale(6);
    let padding = scale(10);
    let old_font = SelectObject(hdc, GetStockObject(DEFAULT_GUI_FONT));
    let sizes: Vec<SIZE> = TOOLBAR_BUTTONS.iter().map(|button| {
        let text: Vec<u16> = button.label.encode_utf16().collect();
        let mut size: SIZE = zeroed();
        GetTextExtentPoint32W(hdc, text.as_ptr(), text.len() as i32, &mut size);
        size
    }).collect();
    SelectObject(hdc, old_font);
    let vertical_margin = scale(1);
    let height = sizes.iter().map(|s| s.cy + scale(2)).max().unwrap_or(scale(18));
    let (mut x, mut y) = (margin, vertical_margin);
    let mut rects = Vec::new();
    for (button, size) in TOOLBAR_BUTTONS.iter().zip(sizes) {
        let width = scale(button.width).max(size.cx + padding * 2);
        if x > margin && x + width > client_w - margin {
            x = margin;
            y += height + scale(2);
        }
        rects.push(RECT { left: x, top: y, right: x + width, bottom: y + height });
        x += width + gap;
    }
    (rects, y + height + vertical_margin)
}

struct StatusControl { id: u16, label: String, rect: RECT }

unsafe fn status_layout(hdc: *mut c_void, client_w: i32, dpi: u32, state: &AppState) -> (Vec<StatusControl>, RECT, i32) {
    let scale = |value: i32| ((value as i64 * dpi.max(96) as i64 + 48) / 96) as i32;
    let margin = scale(6);
    let gap = scale(4);
    let old_font = SelectObject(hdc, GetStockObject(DEFAULT_GUI_FONT));
    let labels = [
        (IDM_TB_FIRST_PAGE,"⏮".to_string()), (IDM_TB_PREV_PAGE,"◀".to_string()),
        (IDM_TOOL_GOTO,format!("{} / {} p (이동)",state.current_page+1,state.page_count.max(1))),
        (IDM_TB_NEXT_PAGE,"▶".to_string()), (IDM_TB_LAST_PAGE,"⏭".to_string()),
        (IDM_VIEW_SPREAD,if state.spread_mode {"2p양면"} else {"1p단면"}.to_string()),
        (IDM_VIEW_ZOOM_OUT,"−".to_string()), (IDM_VIEW_ACTUAL_SIZE,format!("{:.0}%",state.zoom*100.0)),
        (IDM_VIEW_ZOOM_IN,"+".to_string()), (IDM_VIEW_FULLSCREEN,"⛶ 전체화면".to_string())
    ];
    let mut sizes = Vec::new();
    for (_,label) in &labels {
        let text:Vec<u16> = label.encode_utf16().collect(); let mut size:SIZE = zeroed();
        GetTextExtentPoint32W(hdc,text.as_ptr(),text.len() as i32,&mut size); sizes.push(size);
    }
    let sample:Vec<u16> = "📐 문서 대기 중 🔗 🌐 💡".encode_utf16().collect();
    let mut sample_size:SIZE = zeroed();
    GetTextExtentPoint32W(hdc,sample.as_ptr(),sample.len() as i32,&mut sample_size);
    SelectObject(hdc,old_font);
    let row_height = sizes.iter().map(|s|s.cy).max().unwrap_or(0).max(sample_size.cy) + scale(6);
    let widths:Vec<_> = sizes.iter().map(|s|s.cx+scale(10)).collect();
    let nav_width = widths[..5].iter().sum::<i32>()+4*gap;
    let right_width = widths[5..].iter().sum::<i32>()+4*gap;
    let mut controls = Vec::new();
    let mut info = RECT {left:margin,top:0,right:(client_w-margin).max(margin),bottom:row_height};
    let mut height = row_height;
    if nav_width + right_width + scale(100) + 2*gap + 2*margin <= client_w {
        let right_x = client_w-margin-right_width;
        let nav_x = ((client_w-nav_width)/2).min(right_x-gap-nav_width);
        info.right = nav_x-gap;
        let mut x=nav_x;
        for (index,((id,label),width)) in labels.iter().zip(&widths).enumerate() {
            if index==5 {x=right_x;}
            controls.push(StatusControl {id:*id,label:label.clone(),rect:RECT{left:x,top:0,right:x+width,bottom:row_height}});
            x+=width+gap;
        }
    } else {
        // Narrow windows keep the message on its own line and wrap controls intact.
        let mut x=margin;let mut y=row_height;
        for ((id,label),width) in labels.iter().zip(&widths) {
            if x>margin && x+width>client_w-margin {x=margin;y+=row_height;}
            controls.push(StatusControl{id:*id,label:label.clone(),rect:RECT{left:x,top:y,right:x+width,bottom:y+row_height}});
            x+=width+gap;
        }
        height=controls.last().map_or(row_height,|c|c.rect.bottom);
    }
    (controls,info,height)
}

impl AppState {
    unsafe fn update_toolbar_layout(&mut self, hwnd: HWND) {
        let mut rc: RECT = zeroed();
        GetClientRect(hwnd, &mut rc);
        let hdc = GetDC(hwnd);
        if !hdc.is_null() {
            let (rects, height) = toolbar_layout(hdc, rc.right, GetDpiForWindow(hwnd));
            let (controls,info,status_height) = status_layout(hdc,rc.right,GetDpiForWindow(hwnd),self);
            ReleaseDC(hwnd, hdc);
            if self.toolbar_height != height || self.statusbar_height != status_height { self.pixels.clear(); }
            self.toolbar_rects = rects;
            self.toolbar_height = height;
            self.status_controls=controls;self.status_info_rect=info;self.statusbar_height=status_height;
        }
    }
}

fn hit_test_toolbar(state: &AppState, click_x: i32, click_y: i32) -> Option<u16> {
    state.toolbar_rects.iter().zip(TOOLBAR_BUTTONS).find_map(|(rect, button)| {
        (click_x >= rect.left && click_x < rect.right && click_y >= rect.top && click_y < rect.bottom)
            .then_some(button.id)
    })
}

unsafe fn draw_top_toolbar(hdc: *mut c_void, client_w: i32, state: &AppState) {
    let bar_rc = RECT { left: 0, top: 0, right: client_w, bottom: state.toolbar_height };
    let bar_brush = CreateSolidBrush(0x002D2D30);
    FillRect(hdc, &bar_rc, bar_brush);
    DeleteObject(bar_brush as _);

    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT as i32);

    for (btn, btn_rc) in TOOLBAR_BUTTONS.iter().zip(&state.toolbar_rects) {
        let is_active = state.is_toolbar_btn_active(btn.id);
        let is_hover = state.hover_tb_id == Some(btn.id);
        let bg_col = if is_active { 0x007A4B16 } else if is_hover { 0x00505055 } else { 0x003C3C40 };
        let btn_brush = CreateSolidBrush(bg_col);
        FillRect(hdc, btn_rc, btn_brush);
        DeleteObject(btn_brush as _);

        SetTextColor(hdc, if is_active { 0x0066EAFF } else { 0x00F0F0F0 });
        let w_lbl: Vec<u16> = btn.label.encode_utf16().collect();
        let mut size: SIZE = zeroed();
        GetTextExtentPoint32W(hdc, w_lbl.as_ptr(), w_lbl.len() as i32, &mut size);
        TextOutW(hdc, btn_rc.left + (btn_rc.right - btn_rc.left - size.cx) / 2,
            btn_rc.top + (btn_rc.bottom - btn_rc.top - size.cy) / 2, w_lbl.as_ptr(), w_lbl.len() as i32);
    }
    SelectObject(hdc, old_font);
}

fn hit_test_status_bar(state: &AppState, click_x: i32, click_y: i32) -> Option<u16> {
    state.status_controls.iter().find_map(|control| {
        let r=&control.rect;
        (click_x>=r.left && click_x<r.right && click_y>=r.top && click_y<r.bottom).then_some(control.id)
    })
}

unsafe fn draw_bottom_status_bar(hdc: *mut c_void, client_w: i32, client_h: i32, state: &AppState) {
    let top_y = client_h - state.statusbar_height;
    let bar_rc = RECT { left: 0, top: top_y, right: client_w, bottom: client_h };
    let bar_brush = CreateSolidBrush(0x00222225);
    FillRect(hdc, &bar_rc, bar_brush);
    DeleteObject(bar_brush as _);

    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT as i32);

    let w_cm = state.cur_page_w_pt * 2.54 / 72.0;
    let h_cm = state.cur_page_h_pt * 2.54 / 72.0;
    let dim_text = if state.page_count > 0 {
        format!("📐 {:.1} x {:.1} cm ({:.0}x{:.0} pt)", w_cm, h_cm, state.cur_page_w_pt, state.cur_page_h_pt)
    } else {
        String::from("📐 문서 대기 중")
    };

    let status_info = if let Some(ref link) = state.hovered_link {
        match link {
            LinkTarget::InternalPage(p) => format!("🔗 내부 링크: {}페이지로 이동 (클릭)", p + 1),
            LinkTarget::ExternalUri(uri) => format!("🌐 웹 링크: {} (클릭)", uri),
        }
    } else if let Some(ref sel) = state.text_selection {
        format!("🔤 선택됨 ({}자) - 우클릭 메뉴 또는 Ctrl+C 복사", sel.text.chars().count())
    } else if let Some(ref msg) = state.status_msg {
        format!("{}  |  💡 {}", dim_text, msg)
    } else {
        dim_text
    };

    SetTextColor(hdc, 0x00CCCCCC);
    let w_left: Vec<u16> = status_info.encode_utf16().collect();
    let mut info=state.status_info_rect;
    info.top+=top_y;info.bottom+=top_y;
    // A bounded, vertically centered line prevents both clipping and overlap.
    DrawTextW(hdc,w_left.as_ptr(),w_left.len() as i32,&mut info,0x20|0x4|0x8000|0x800);
    for (index,control) in state.status_controls.iter().enumerate() {
        SetTextColor(hdc,if index<5 {0x0080D8FF} else {0x00E6E6E6});
        let text:Vec<u16>=control.label.encode_utf16().collect();let mut size:SIZE=zeroed();
        GetTextExtentPoint32W(hdc,text.as_ptr(),text.len() as i32,&mut size);
        let r=&control.rect;
        TextOutW(hdc,r.left+(r.right-r.left-size.cx)/2,top_y+r.top+(r.bottom-r.top-size.cy)/2,text.as_ptr(),text.len() as i32);
    }

    SelectObject(hdc, old_font);
}

unsafe fn draw_page_overlays_and_guides(hdc: *mut c_void, page_x: i32, page_y: i32, client_w: i32, client_h: i32, state: &AppState) {
    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT as i32);

    let pw = state.bmp_width;
    let ph = state.bmp_height;

    if let Some(ref wm) = state.watermark_text {
        SetTextColor(hdc, 0x009090D0);
        let w_wm: Vec<u16> = format!("=== [ {} ] ===", wm).encode_utf16().collect();
        for step in [-ph / 4, 0, ph / 4] {
            TextOutW(hdc, page_x + pw / 2 - 70, page_y + ph / 2 + step, w_wm.as_ptr(), w_wm.len() as i32);
        }
    }

    if state.show_annotations {
        for ov in state.overlays.iter().filter(|o| o.page_index == state.current_page) {
            let x1 = page_x + (ov.x1 * pw as f64) as i32;
            let y1 = page_y + (ov.y1 * ph as f64) as i32;
            let x2 = page_x + (ov.x2 * pw as f64) as i32;
            let y2 = page_y + (ov.y2 * ph as f64) as i32;

            match ov.kind {
                OverlayKind::Highlight | OverlayKind::AreaHighlight => {
                    let pen = CreatePen(PS_SOLID, 2, 0x0000D8FF);
                    let old_p = SelectObject(hdc, pen as _);
                    let old_b = SelectObject(hdc, GetStockObject(5));
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
                    let old_b = SelectObject(hdc, GetStockObject(5));
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
                        MoveToEx(hdc, page_x + (p0.0 * pw as f64) as i32, page_y + (p0.1 * ph as f64) as i32, null_mut());
                        for pt in &ov.points[1..] {
                            LineTo(hdc, page_x + (pt.0 * pw as f64) as i32, page_y + (pt.1 * ph as f64) as i32);
                        }
                        SelectObject(hdc, old_p);
                        DeleteObject(pen as _);
                    }
                }
                OverlayKind::StampApproved | OverlayKind::StampDraft | OverlayKind::Note => {
                    let rc = RECT { left: x1, top: y1, right: x1 + 150, bottom: y1 + 32 };
                    let color = if ov.kind == OverlayKind::StampApproved { 0x00228822 } else if ov.kind == OverlayKind::StampDraft { 0x00AA4400 } else { 0x0088FFFF };
                    let br = CreateSolidBrush(color);
                    FillRect(hdc, &rc, br);
                    DeleteObject(br as _);
                    SetTextColor(hdc, if ov.kind == OverlayKind::Note { 0x00000000 } else { 0x00FFFFFF });
                    let w_lbl: Vec<u16> = ov.label.encode_utf16().collect();
                    TextOutW(hdc, x1 + 8, y1 + 8, w_lbl.as_ptr(), w_lbl.len() as i32);
                }
                OverlayKind::Redaction => {
                    let rc = RECT { left: x1.min(x2), top: y1.min(y2), right: x1.max(x2).max(x1 + 10), bottom: y1.max(y2).max(y1 + 10) };
                    let br = CreateSolidBrush(0x00000000);
                    FillRect(hdc, &rc, br);
                    DeleteObject(br as _);
                }
                OverlayKind::FormText | OverlayKind::FormDropdown => {
                    let is_focused = matches!(state.input_mode, InputMode::EditFormField(idx) if state.overlays.get(idx).map(|f| std::ptr::eq(f, ov)).unwrap_or(false));
                    let rc = RECT { left: x1.min(x2), top: y1.min(y2), right: x1.max(x2).max(x1 + 140), bottom: y1.max(y2).max(y1 + 28) };
                    let br = CreateSolidBrush(if is_focused { 0x00D8F8FF } else { 0x00FFF4E8 });
                    FillRect(hdc, &rc, br);
                    DeleteObject(br as _);
                    let pen = CreatePen(PS_SOLID, if is_focused { 2 } else { 1 }, if is_focused { 0x000066FF } else { 0x00996633 });
                    let old_p = SelectObject(hdc, pen as _);
                    let old_b = SelectObject(hdc, GetStockObject(5));
                    Rectangle(hdc, rc.left, rc.top, rc.right, rc.bottom);
                    SelectObject(hdc, old_b);
                    SelectObject(hdc, old_p);
                    DeleteObject(pen as _);

                    SetTextColor(hdc, 0x00442200);
                    let disp = if is_focused {
                        format!("{}_", ov.label)
                    } else if ov.kind == OverlayKind::FormDropdown {
                        format!("▼ {}", ov.label)
                    } else if ov.label.is_empty() {
                        String::from("(클릭하여 입력)")
                    } else {
                        ov.label.clone()
                    };
                    let w_lbl: Vec<u16> = disp.encode_utf16().collect();
                    TextOutW(hdc, rc.left + 6, rc.top + 6, w_lbl.as_ptr(), w_lbl.len() as i32);
                }
                OverlayKind::SignatureImage(sig_idx) => {
                    if let Some(sig) = state.signatures.get(sig_idx) {
                        let rx1 = x1.min(x2);
                        let ry1 = y1.min(y2);
                        let rw = (x1.max(x2) - rx1).max(32);
                        let rh = (y1.max(y2) - ry1).max(24);
                        let src_dc = CreateCompatibleDC(hdc);
                        let src_bm = CreateCompatibleBitmap(hdc, sig.width, sig.height);
                        let old_sbm = SelectObject(src_dc, src_bm as _);
                        let mut bmi: BITMAPINFO = zeroed();
                        bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
                        bmi.bmiHeader.biWidth = sig.width;
                        bmi.bmiHeader.biHeight = -sig.height;
                        bmi.bmiHeader.biPlanes = 1;
                        bmi.bmiHeader.biBitCount = 32;
                        bmi.bmiHeader.biCompression = BI_RGB;
                        StretchDIBits(
                            src_dc, 0, 0, sig.width, sig.height, 0, 0, sig.width, sig.height,
                            sig.pm_bgra.as_ptr() as *const c_void, &bmi, DIB_RGB_COLORS, SRCCOPY,
                        );
                        let bf = BLENDFUNCTION { BlendOp: 0, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: 1 };
                        AlphaBlend(hdc as *mut c_void, rx1, ry1, rw, rh, src_dc as *mut c_void, 0, 0, sig.width, sig.height, bf);
                        SelectObject(src_dc, old_sbm);
                        DeleteObject(src_bm as _);
                        DeleteDC(src_dc);

                        let pen = CreatePen(PS_DOT, 1, 0x00FF8800);
                        let old_p = SelectObject(hdc, pen as _);
                        let old_b = SelectObject(hdc, GetStockObject(5));
                        Rectangle(hdc, rx1, ry1, rx1 + rw, ry1 + rh);
                        SelectObject(hdc, old_b);
                        SelectObject(hdc, old_p);
                        DeleteObject(pen as _);

                        let handle_rc = RECT { left: rx1 + rw - 10, top: ry1 + rh - 10, right: rx1 + rw + 2, bottom: ry1 + rh + 2 };
                        let h_br = CreateSolidBrush(0x00FF6600);
                        FillRect(hdc, &handle_rc, h_br);
                        DeleteObject(h_br as _);
                    }
                }
                OverlayKind::FormCheck | OverlayKind::FormRadio => {
                    let rc = RECT { left: x1, top: y1, right: x1 + 22, bottom: y1 + 22 };
                    let br = CreateSolidBrush(0x00FFFFFF);
                    FillRect(hdc, &rc, br);
                    DeleteObject(br as _);
                    SetTextColor(hdc, 0x00000000);
                    let mark = if ov.kind == OverlayKind::FormRadio {
                        if ov.checked { "(●)" } else { "( )" }
                    } else if ov.checked { "[V]" } else { "[ ]" };
                    let w_m: Vec<u16> = mark.encode_utf16().collect();
                    TextOutW(hdc, x1 + 1, y1 + 3, w_m.as_ptr(), w_m.len() as i32);
                }
            }
        }
    }

    if state.is_dragging && state.active_tool == ActiveTool::PlaceSignature {
        let pen = CreatePen(PS_SOLID, 2, 0x000066FF);
        let old_p = SelectObject(hdc, pen as _);
        let old_b = SelectObject(hdc, GetStockObject(5));
        Rectangle(
            hdc,
            state.drag_start_x.min(state.drag_cur_x),
            state.drag_start_y.min(state.drag_cur_y),
            state.drag_start_x.max(state.drag_cur_x),
            state.drag_start_y.max(state.drag_cur_y),
        );
        SelectObject(hdc, old_b);
        SelectObject(hdc, old_p);
        DeleteObject(pen as _);
    }

    let _ = (client_w, client_h);
    SelectObject(hdc, old_font);
}

unsafe fn build_main_menu_bar() -> *mut c_void {
    let hmenu = CreateMenu();
    let add_item = |menu: *mut c_void, id: u16, label: &str| {
        let w = to_wstring(label);
        AppendMenuW(menu, MF_STRING, id as usize, w.as_ptr());
    };
    let add_sep = |menu: *mut c_void| { AppendMenuW(menu, MF_SEPARATOR, 0, null()); };
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
    add_item(m_file, IDM_FILE_SAVE_V14, "PDF 1.4로 저장...");
    add_item(m_file, IDM_FILE_SAVE_V17, "PDF 1.7로 저장...");
    add_item(m_file, IDM_FILE_PRINT, "인쇄하기(&P)...\tCtrl+P");
    add_item(m_file, IDM_FILE_PROPS, "파일 속성(&I)...\tI");
    add_sep(m_file);
    add_item(m_file, IDM_FILE_EXIT, "종료(&X)\tESC");
    add_sub(hmenu, m_file, "파일(&F)");

    let m_edit = CreatePopupMenu();
    add_item(m_edit, IDM_EDIT_UNDO, "실행 취소\tCtrl+Z");
    add_item(m_edit, IDM_EDIT_REDO, "다시 실행\tCtrl+Y");
    add_sep(m_edit);
    add_item(m_edit, IDM_EDIT_ADD_TEXT, "텍스트 추가(&T)...");
    add_sep(m_edit);
    add_item(m_edit, IDM_EDIT_WM_CONFIDENTIAL, "워터마크 추가: 'CONFIDENTIAL'");
    add_item(m_edit, IDM_EDIT_WM_CUSTOM, "사용자 지정 워터마크 입력...");
    add_item(m_edit, IDM_EDIT_WM_CLEAR, "워터마크 제거");
    add_sep(m_edit);
    add_item(m_edit, IDM_EDIT_BG_CREAM, "배경색: 눈 보호 크림색");
    add_item(m_edit, IDM_EDIT_BG_CLEAR, "배경색 제거");
    add_sub(hmenu, m_edit, "편집(&E)");

    let m_annot = CreatePopupMenu();
    add_item(m_annot, IDM_ANNOT_NONE, "기본 손 도구 (화면 이동 및 링크 클릭)");
    add_item(m_annot, IDM_ANNOT_TEXT_SELECT, "텍스트 선택 도구 (드래그 선택 및 우클릭 메뉴)");
    add_sep(m_annot);
    add_item(m_annot, IDM_ANNOT_HIGHLIGHT, "강조 (형광펜 드래그)");
    add_item(m_annot, IDM_ANNOT_UNDERLINE, "밑줄 표시 (드래그)\tCtrl+U");
    add_item(m_annot, IDM_ANNOT_STRIKEOUT, "취소선 표시 (드래그)\tCtrl+Q");
    add_item(m_annot, IDM_ANNOT_PENCIL, "연필 (자유 곡선 그리기)");
    add_item(m_annot, IDM_ANNOT_ERASER, "지우개 (클릭하여 주석 삭제)");
    add_sep(m_annot);
    add_item(m_annot, IDM_SIGN_UPLOAD_IMAGE, "서명/인장 이미지 업로드 및 크기 조절 날인 (.png/.jpg/.bmp)...");
    add_item(m_annot, IDM_SIGN_CREATE_SEAL, "전자 인장(도장) 텍스트로 만들기...");
    add_item(m_annot, IDM_SIGN_DRAW_HAND, "자필 서명 마우스로 직접 그리기");
    add_item(m_annot, IDM_SIGN_PLACE_SAVED, "최근 등록한 서명/인장 다시 날인");
    add_sep(m_annot);
    add_item(m_annot, IDM_ANNOT_CLEAR_ALL, "모든 사용자 주석 초기화");
    add_sub(hmenu, m_annot, "주석(&A)");

    let m_conv = CreatePopupMenu();
    add_item(m_conv, IDM_CONV_TO_BMP, "이미지로 변환 (.bmp)...\tS");
    add_item(m_conv, IDM_CONV_TO_TXT, "텍스트로 변환 (.txt)...");
    add_item(m_conv, IDM_CONV_TO_HTML, "HTML 웹 문서로 변환 (.html)...");
    add_item(m_conv, IDM_CONV_TO_RTF, "Word 호환 문서로 변환 (.rtf)...");
    add_item(m_conv, IDM_CONV_TO_DOCX, "Word 문서 (.docx, 텍스트 중심·스캔 페이지 OCR)...");
    add_item(m_conv, IDM_CONV_TO_CSV, "Excel 호환 데이터로 변환 (.csv)...");
    add_sub(hmenu, m_conv, "변환(&C)");

    let m_view = CreatePopupMenu();
    add_item(m_view, IDM_VIEW_ZOOM_IN, "확대(&I)\t+");
    add_item(m_view, IDM_VIEW_ZOOM_OUT, "축소(&O)\t-");
    add_item(m_view, IDM_VIEW_FIT_WIDTH, "너비 맞추기 (Fit-to-Width)\tW");
    add_item(m_view, IDM_VIEW_SPREAD, "두 페이지 양면 보기 (2p)\tD");
    add_item(m_view, IDM_VIEW_ROTATE_CW, "시계 방향 회전 (90°)\tR");
    add_sep(m_view);
    add_item(m_view, IDM_VIEW_THUMBNAILS, "좌측 패널: 페이지 썸네일 미리보기 탭");
    add_item(m_view, IDM_VIEW_FULLSCREEN, "전체 화면\tF11");
    add_sub(hmenu, m_view, "보기(&V)");

    let m_org = CreatePopupMenu();
    add_item(m_org, IDM_ORG_INSERT_BLANK, "빈 페이지 삽입(&B)");
    add_item(m_org, IDM_ORG_INSERT_PDF, "다른 PDF에서 페이지 삽입(&I)...");
    add_item(m_org, IDM_ORG_DELETE_PAGE, "현재 페이지 삭제(&D)");
    add_item(m_org, IDM_ORG_EXTRACT_PAGE, "현재 페이지 추출 저장(&E)...");
    add_item(m_org, IDM_ORG_EXTRACT_RANGE, "페이지 범위 추출 (예: 1-3,5)...");
    add_item(m_org, IDM_ORG_SPLIT_PDF, "선택 범위를 한 페이지씩 분할 저장...");
    add_item(m_org, IDM_ORG_REORDER, "전체 페이지 순서 변경 (예: 3,1,2)...");
    add_item(m_org, IDM_ORG_CROP_MARGIN, "페이지 여백 자르기 (CropBox)");
    add_sub(hmenu, m_org, "구성(&P)");

    let m_tool = CreatePopupMenu();
    add_item(m_tool, IDM_TOOL_SEARCH, "문서 전체 키워드 검색(&F)...\tF");
    add_item(m_tool, IDM_TOOL_OCR_EXTRACT, "현재 페이지 OCR → 텍스트 저장...");
    add_item(m_tool, IDM_TOOL_GOTO, "페이지 번호 직접 이동(&G)...\tG");
    add_item(m_tool, IDM_TOOL_MERGE_PDF, "PDF 병합...");
    add_item(m_tool, IDM_TOOL_COMPARE_PDF, "PDF 비교 (VS)...");
    add_item(m_tool, IDM_TOOL_FLATTEN, "평면화 (주석/양식/서명 영구 병합)");
    add_item(m_tool, IDM_TOOL_DETECT_TABLES, "표 인식 (CSV 클립보드 복사)");
    add_item(m_tool, IDM_TOOL_SCREENSHOT, "현재 화면 스크린샷 복사");
    add_sub(hmenu, m_tool, "도구(&T)");

    let m_form = CreatePopupMenu();
    add_item(m_form, IDM_FORM_ADD_TEXTFIELD, "텍스트 필드 추가 (드래그 후 클릭하여 타이핑)");
    add_item(m_form, IDM_FORM_ADD_CHECKBOX, "체크 박스 추가 (클릭 토글)");
    add_item(m_form, IDM_FORM_ADD_RADIO, "라디오 버튼 추가 (단일 선택)");
    add_item(m_form, IDM_FORM_ADD_DROPDOWN, "드롭다운(콤보) 필드 추가 (클릭 옵션 순환)");
    add_item(m_form, IDM_FORM_ADD_DATE, "날짜 필드 추가 (YYYY-MM-DD)");
    add_sep(m_form);
    add_item(m_form, IDM_FORM_EXPORT_CSV, "양식 데이터 추출 (.csv 저장)...");
    add_item(m_form, IDM_FORM_EXPORT_FDF, "양식 데이터 내보내기 (.fdf 저장)...");
    add_item(m_form, IDM_FORM_IMPORT_FDF, "양식 데이터 가져오기 (.fdf 불러오기)...");
    add_item(m_form, IDM_FORM_RESET_DATA, "기본 데이터 복구 (양식 입력값 초기화)");
    add_sub(hmenu, m_form, "양식(&M)");

    let m_sec = CreatePopupMenu();
    add_item(m_sec, IDM_SEC_REDACT_TOOL, "교정 표시 도구 (검은색 마스킹)");
    add_item(m_sec, IDM_SEC_SEARCH_REDACT, "검색 & 교정 (일괄 마스킹)...");
    add_item(m_sec, IDM_SIGN_UPLOAD_IMAGE, "서명/인장 이미지 파일 불러와 날인...");
    add_item(m_sec, IDM_SIGN_CREATE_SEAL, "붉은색 직인/도장 생성 및 날인...");
    add_sub(hmenu, m_sec, "보안(&S)");

    let m_etc = CreatePopupMenu();
    add_item(m_etc, IDM_THEME_DARK_DOC, "야간 다크 모드 토글\tV");
    add_item(m_etc, IDM_HELP_SHORTCUTS, "전체 기능 및 단축키 도움말(&H)\tH / F1");
    add_sub(hmenu, m_etc, "기타(&O)");

    hmenu
}

fn update_window_title(hwnd: HWND, state: &AppState) {
    let title = if state.page_count > 0 {
        let base = format!(
            "DaPDF(v{}) - {} ({}/{}p) [Zoom: {:.0}%]",
            APP_VERSION,
            state.file_name,
            state.current_page + 1,
            state.page_count,
            state.zoom * 100.0
        );
        match state.input_mode {
            InputMode::GotoPage => format!("{} | 📄 페이지 이동 (1~{}): {}_ [Enter:이동]", base, state.page_count, state.goto_input),
            InputMode::PageOperation(action) => format!("{} | {}: {}_ [Enter:적용, ESC:취소]",base,
                if action==IDM_ORG_REORDER {"전체 페이지 순서 (예: 3,1,2)"} else {"페이지 범위 (예: 1-3,5)"},state.goto_input),
            InputMode::AddTextPrompt => format!("{} | ✏️ 추가할 텍스트 입력: \"{}_\" [Enter:삽입]", base, state.text_prompt_buf),
            InputMode::CustomWatermarkPrompt => format!("{} | 💧 워터마크 입력: \"{}_\" [Enter:적용]", base, state.text_prompt_buf),
            InputMode::CreateSealPrompt => format!("{} | 💮 도장/인장에 새길 이름 입력: \"{}_\" [Enter:생성, ESC:취소]", base, state.text_prompt_buf),
            InputMode::EditFormField(idx) => {
                let cur_val = state.overlays.get(idx).map(|o| o.label.as_str()).unwrap_or("");
                format!("{} | 📝 양식 필드 타이핑: \"{}_\" [Enter:확정, Tab:다음필드, ESC:종료]", base, cur_val)
            }
            InputMode::SearchRedactPrompt => format!("{} | ⬛ 일괄 교정할 단어 입력: \"{}_\" [Enter:적용]", base, state.search_query),
            InputMode::Search => format!("{} | 🔍 전체검색: \"{}_\" [Enter:확정, F3:다음]", base, state.search_query),
            InputMode::Normal => {
                if let Some(ref msg) = state.status_msg {
                    format!("{} - [{}]", base, msg)
                } else {
                    base
                }
            }
        }
    } else {
        format!("DaPDF(v{}) - {}", APP_VERSION, state.file_name)
    };
    let title = if state.has_unsaved_changes() { format!("* {}",title) } else { title };
    let w_title = to_wstring(&title);
    unsafe { SetWindowTextW(hwnd, w_title.as_ptr()) };
}

fn to_wstring(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn write_bgra_to_bmp_file(path: &str, width: i32, height: i32, pixels: &[u8]) -> std::io::Result<()> {
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
    header[34..38].copy_from_slice(&(data_size as u32).to_le_bytes());
    file.write_all(&header)?;
    file.write_all(&pixels[..data_size])?;
    Ok(())
}

fn save_custom_dialog(hwnd: HWND, default_name: &str, filter_str: &str, def_ext_str: &str) -> Option<String> {
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
        ofn.Flags = OFN_EXPLORER | OFN_ENABLESIZING | OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
        if GetSaveFileNameW(&mut ofn) != 0 {
            let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
            Some(String::from_utf16_lossy(&file_buf[..len]))
        } else {
            None
        }
    }
}

fn get_clipboard_unicode_text(hwnd: HWND) -> Option<String> {
    unsafe {
        if OpenClipboard(hwnd) == 0 { return None; }
        let hmem = GetClipboardData(CF_UNICODETEXT);
        if hmem.is_null() { CloseClipboard(); return None; }
        let ptr = GlobalLock(hmem) as *const u16;
        if ptr.is_null() { CloseClipboard(); return None; }
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
        if OpenClipboard(hwnd) == 0 { return false; }
        EmptyClipboard();
        let hmem = GlobalAlloc(GMEM_MOVEABLE, byte_len);
        if hmem.is_null() { CloseClipboard(); return false; }
        let locked_ptr = GlobalLock(hmem) as *mut u16;
        if locked_ptr.is_null() { CloseClipboard(); return false; }
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
        hdc_mem, 0, 0, state.bmp_width, state.bmp_height, 0, 0, state.bmp_width, state.bmp_height,
        state.pixels.as_ptr() as *const c_void, &bmi, DIB_RGB_COLORS, SRCCOPY,
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

unsafe fn handle_menu_command(hwnd: HWND, cmd_id: u16, state: &mut AppState) {
    if matches!(cmd_id, IDM_FILE_NEW_BLANK | IDM_FILE_FROM_CLIPBOARD | IDM_FILE_OPEN) && !state.confirm_unsaved(hwnd) { return; }
    let native_edit = matches!(cmd_id, IDM_ORG_INSERT_BLANK | IDM_ORG_INSERT_PDF | IDM_ORG_DELETE_PAGE |
        IDM_ORG_CROP_MARGIN | IDM_TOOL_MERGE_PDF | IDM_TOOL_FLATTEN);
    let overlay_edit = matches!(cmd_id, IDM_ANNOT_CLEAR_ALL | IDM_SEC_SEARCH_REDACT | IDM_FORM_RESET_DATA |
        IDM_FORM_IMPORT_FDF | IDM_EDIT_WM_CONFIDENTIAL | IDM_EDIT_WM_CLEAR | IDM_EDIT_BG_CREAM | IDM_EDIT_BG_CLEAR);
    if native_edit || overlay_edit {
        if !state.record_edit() { return; }
        if native_edit { state.history.native_cache = None; }
    }
    match cmd_id {
        IDM_TOOL_OCR_EXTRACT => { if !state.doc.is_null() {state.export_ocr(hwnd);} }
        IDM_CONV_TO_DOCX => { if !state.doc.is_null() {state.export_docx(hwnd);} }
        IDM_ORG_EXTRACT_RANGE | IDM_ORG_SPLIT_PDF | IDM_ORG_REORDER => {
            if !state.doc.is_null() { state.goto_input.clear(); state.input_mode=InputMode::PageOperation(cmd_id); state.refresh_view(hwnd,false); }
        }
        IDM_EDIT_UNDO => { state.undo_redo(false); state.refresh_view(hwnd,true); }
        IDM_EDIT_REDO => { state.undo_redo(true); state.refresh_view(hwnd,true); }
        IDM_TB_PREV_PAGE => state.change_page(-1, hwnd),
        IDM_TB_NEXT_PAGE => state.change_page(1, hwnd),
        IDM_TB_FIRST_PAGE => state.go_to_first_page(hwnd),
        IDM_TB_LAST_PAGE => state.go_to_last_page(hwnd),
        IDM_FILE_NEW_BLANK => state.create_blank_pdf(hwnd),
        IDM_FILE_FROM_CLIPBOARD => state.create_pdf_from_clipboard(hwnd),
        IDM_FILE_OPEN => {
            if let Some(path) = open_file_dialog(hwnd) {
                match state.load_file(&path) {
                    Ok(()) => state.refresh_view(hwnd, true),
                    Err(err) => {
                        state.status_msg = Some(format!("열기 실패: {}", err));
                        state.refresh_view(hwnd, false);
                    }
                }
            }
        }
        IDM_FILE_SAVE => {
            if let Some(existing) = state.current_path.clone() {
                state.status_msg = Some(match state.save_pdf_to_path(&existing, None) {
                    Ok(()) => format!("저장 완료: {}", existing),
                    Err(err) => format!("저장 실패: {}", err),
                });
                state.refresh_view(hwnd, false);
            } else {
                handle_menu_command(hwnd, IDM_FILE_SAVE_AS, state);
            }
        }
        IDM_FILE_SAVE_AS | IDM_FILE_SAVE_V14 | IDM_FILE_SAVE_V17 => {
            if let Some(save_path) = save_custom_dialog(hwnd, &state.file_name, "PDF 문서 (*.pdf)\0*.pdf\0\0", "pdf") {
                let version = match cmd_id {
                    IDM_FILE_SAVE_V14 => Some(14),
                    IDM_FILE_SAVE_V17 => Some(17),
                    _ => None,
                };
                state.status_msg = Some(match state.save_pdf_to_path(&save_path, version) {
                    Ok(()) => format!("PDF 저장 완료: {}", save_path),
                    Err(err) => format!("저장 실패: {}", err),
                });
                state.refresh_view(hwnd, false);
            }
        }
        IDM_FILE_PRINT => {
            if state.doc.is_null() { return; }
            let mut pd: PRINTDLGW = zeroed();
            pd.lStructSize = size_of::<PRINTDLGW>() as u32;
            pd.hwndOwner = hwnd;
            pd.Flags = PD_RETURNDC | 0x40000 | 0x4; // driver copies/collation; no text selection
            pd.nMinPage = 1;
            pd.nMaxPage = state.page_count.min(u16::MAX as i32) as u16;
            pd.nFromPage = 1;
            pd.nToPage = pd.nMaxPage;
            pd.nCopies = 1;
            if PrintDlgW(&mut pd) != 0 && !pd.hDC.is_null() {
                let first = if pd.Flags & 2 != 0 { pd.nFromPage as i32 - 1 } else { 0 };
                let last = if pd.Flags & 2 != 0 { pd.nToPage as i32 - 1 } else { state.page_count - 1 };
                let result = printing::print_document(state, pd.hDC, first, last, None);
                DeleteDC(pd.hDC);
                state.status_msg = Some(match result { Ok(()) => "인쇄 작업을 프린터에 전송했습니다".into(), Err(err) => format!("인쇄 실패: {}",err) });
                state.refresh_view(hwnd, false);
            }
            if !pd.hDevMode.is_null() { GlobalFree(pd.hDevMode); }
            if !pd.hDevNames.is_null() { GlobalFree(pd.hDevNames); }
        }
        IDM_FILE_PROPS => state.show_metadata_dialog(hwnd),
        IDM_FILE_EXIT => { if state.confirm_unsaved(hwnd) { PostQuitMessage(0); } }

        IDM_EDIT_ADD_TEXT => {
            if state.doc.is_null() { state.create_blank_pdf(hwnd); }
            state.input_mode = InputMode::AddTextPrompt;
            state.text_prompt_buf.clear();
            state.refresh_view(hwnd, false);
        }
        IDM_EDIT_WM_CONFIDENTIAL => {
            state.watermark_text = Some("CONFIDENTIAL / 기밀".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_EDIT_WM_CUSTOM => {
            state.input_mode = InputMode::CustomWatermarkPrompt;
            state.text_prompt_buf.clear();
            state.refresh_view(hwnd, false);
        }
        IDM_EDIT_WM_CLEAR => {
            state.watermark_text = None;
            state.refresh_view(hwnd, false);
        }
        IDM_EDIT_BG_CREAM => {
            state.page_bg_tint = Some((252, 248, 227));
            state.refresh_view(hwnd, true);
        }
        IDM_EDIT_BG_CLEAR => {
            state.page_bg_tint = None;
            state.refresh_view(hwnd, true);
        }

        IDM_ANNOT_NONE => {
            state.active_tool = ActiveTool::Pan;
            state.text_selection = None;
            state.status_msg = Some("기본 손 도구 모드".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_TEXT_SELECT => {
            state.active_tool = ActiveTool::TextSelect;
            state.status_msg = Some("텍스트 선택 도구: 드래그 후 우클릭 메뉴 또는 Ctrl+C를 누르세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_ANNOT_HIGHLIGHT => { state.active_tool = ActiveTool::Highlight; state.refresh_view(hwnd, false); }
        IDM_ANNOT_UNDERLINE => { state.active_tool = ActiveTool::Underline; state.refresh_view(hwnd, false); }
        IDM_ANNOT_STRIKEOUT => { state.active_tool = ActiveTool::Strikeout; state.refresh_view(hwnd, false); }
        IDM_ANNOT_PENCIL => { state.active_tool = ActiveTool::Pencil; state.refresh_view(hwnd, false); }
        IDM_ANNOT_ERASER => { state.active_tool = ActiveTool::Eraser; state.refresh_view(hwnd, false); }
        IDM_ANNOT_CLEAR_ALL => { state.overlays.clear(); state.refresh_view(hwnd, false); }

        IDM_SIGN_UPLOAD_IMAGE => {
            if state.doc.is_null() { state.create_blank_pdf(hwnd); }
            if let Some(img_path) = open_image_file_dialog(hwnd) {
                if let Some(sig_data) = load_signature_image_from_file(&img_path) {
                    let w = sig_data.width;
                    let h = sig_data.height;
                    state.signatures.push(sig_data);
                    state.active_sig_idx = Some(state.signatures.len() - 1);
                    state.active_tool = ActiveTool::PlaceSignature;
                    state.status_msg = Some(format!("서명/인장 로드 완료 ({}x{}px, 흰 배경 투명화됨): 페이지 위를 드래그해 원하는 크기로 날인하세요!", w, h));
                    state.refresh_view(hwnd, false);
                } else {
                    state.status_msg = Some("이미지 파일을 읽을 수 없습니다 (.png/.jpg/.bmp 지원)".to_string());
                    state.refresh_view(hwnd, false);
                }
            }
        }
        IDM_SIGN_CREATE_SEAL => {
            if state.doc.is_null() { state.create_blank_pdf(hwnd); }
            state.input_mode = InputMode::CreateSealPrompt;
            state.text_prompt_buf.clear();
            state.status_msg = Some("도장/인장에 새길 이름을 입력 후 Enter를 누르세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_SIGN_DRAW_HAND => {
            if state.doc.is_null() { state.create_blank_pdf(hwnd); }
            state.active_tool = ActiveTool::Pencil;
            state.status_msg = Some("자필 서명 그리기 모드".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_SIGN_PLACE_SAVED => {
            if state.active_sig_idx.is_some() {
                state.active_tool = ActiveTool::PlaceSignature;
                state.status_msg = Some("등록된 서명/인장 날인 모드: 원하는 위치에 드래그하여 날인하세요".to_string());
            } else {
                state.status_msg = Some("먼저 서명 이미지를 업로드하거나 전자 인장을 생성해 주세요".to_string());
            }
            state.refresh_view(hwnd, false);
        }

        IDM_CONV_TO_BMP => state.save_current_page_image(hwnd),
        IDM_CONV_TO_TXT => state.export_document_as(hwnd, "txt"),
        IDM_CONV_TO_HTML => state.export_document_as(hwnd, "html"),
        IDM_CONV_TO_RTF => state.export_document_as(hwnd, "rtf"),
        IDM_CONV_TO_CSV => state.export_document_as(hwnd, "csv"),

        IDM_VIEW_ZOOM_IN => state.change_zoom(1.15, hwnd),
        IDM_VIEW_ZOOM_OUT => state.change_zoom(1.0 / 1.15, hwnd),
        IDM_VIEW_ACTUAL_SIZE => { state.zoom = 1.0; state.fit_width = false; state.refresh_view(hwnd, true); }
        IDM_VIEW_FIT_WIDTH => state.toggle_fit_width(hwnd),
        IDM_VIEW_SPREAD => state.toggle_spread_mode(hwnd),
        IDM_VIEW_ROTATE_CW => state.rotate_page(true, hwnd),
        IDM_VIEW_BOOKMARKS => {
            // The outline panel is disabled; thumbnails are the default navigation.
        }
        IDM_VIEW_THUMBNAILS => {
            if state.show_sidebar && state.sidebar_tab == SidebarTab::Thumbnails { state.show_sidebar = false; }
            else { state.show_sidebar = true; state.sidebar_tab = SidebarTab::Thumbnails; }
            state.refresh_view(hwnd, true);
        }
        IDM_VIEW_AUTO_BOOKMARK => {},
        IDM_VIEW_FULLSCREEN => state.toggle_fullscreen(hwnd),

        IDM_ORG_INSERT_BLANK => state.insert_blank_page_after_current(hwnd),
        IDM_ORG_INSERT_PDF => state.merge_external_pdf(hwnd, false),
        IDM_ORG_DELETE_PAGE => state.delete_current_page(hwnd),
        IDM_ORG_EXTRACT_PAGE => state.extract_current_page_to_pdf(hwnd),
        IDM_ORG_CROP_MARGIN => state.crop_current_page_margins(hwnd),

        IDM_TOOL_SEARCH => {
            if !state.doc.is_null() {
                state.input_mode = InputMode::Search;
                state.show_sidebar = true;
                state.sidebar_tab = SidebarTab::Search;
                state.refresh_view(hwnd, true);
            }
        }
        IDM_TOOL_GOTO => {
            if !state.doc.is_null() && state.page_count > 0 {
                state.input_mode = InputMode::GotoPage;
                state.goto_input.clear();
                state.refresh_view(hwnd, false);
            }
        }
        IDM_TOOL_MERGE_PDF => state.merge_external_pdf(hwnd, true),
        IDM_TOOL_COMPARE_PDF => state.compare_with_other_pdf(hwnd),
        IDM_TOOL_FLATTEN => state.flatten_current_page(hwnd),
        IDM_TOOL_DETECT_TABLES => state.detect_and_copy_tables(hwnd),
        IDM_TOOL_SCREENSHOT => {
            if copy_current_view_bitmap_to_clipboard(hwnd, state) {
                state.status_msg = Some("스크린샷 클립보드 복사 완료!".to_string());
                state.refresh_view(hwnd, false);
            }
        }

        IDM_FORM_ADD_TEXTFIELD => {
            state.active_tool = ActiveTool::FormTextField;
            state.status_msg = Some("텍스트 필드 추가: 영역을 드래그한 뒤 바로 키보드로 타이핑하세요".to_string());
            state.refresh_view(hwnd, false);
        }
        IDM_FORM_ADD_CHECKBOX => { state.active_tool = ActiveTool::FormCheckBox; state.refresh_view(hwnd, false); }
        IDM_FORM_ADD_RADIO => { state.active_tool = ActiveTool::FormRadio; state.refresh_view(hwnd, false); }
        IDM_FORM_ADD_DROPDOWN => { state.active_tool = ActiveTool::FormDropdown; state.refresh_view(hwnd, false); }
        IDM_FORM_ADD_DATE => { state.active_tool = ActiveTool::FormDate; state.refresh_view(hwnd, false); }
        IDM_FORM_EXPORT_CSV => {
            if let Some(save_path) = save_custom_dialog(hwnd, "form_data.csv", "CSV 파일 (*.csv)\0*.csv\0\0", "csv") {
                let mut csv = String::from("Page,FieldType,Value,Checked\r\n");
                for ov in &state.overlays {
                    if matches!(ov.kind, OverlayKind::FormText | OverlayKind::FormCheck | OverlayKind::FormRadio | OverlayKind::FormDropdown) {
                        csv.push_str(&format!("{},Form,\"{}\",{}\r\n", ov.page_index + 1, ov.label, ov.checked));
                    }
                }
                let _ = fs::write(&save_path, csv.as_bytes());
                state.status_msg = Some(format!("양식 CSV 저장 완료: {}", save_path));
                state.refresh_view(hwnd, false);
            }
        }
        IDM_FORM_EXPORT_FDF => {
            if let Some(save_path) = save_custom_dialog(hwnd, "form_data.fdf", "FDF 양식 데이터 (*.fdf)\0*.fdf\0\0", "fdf") {
                let mut fdf = String::from("%FDF-1.2\n1 0 obj\n<< /FDF << /Fields [\n");
                for (i, ov) in state.overlays.iter().enumerate() {
                    if matches!(ov.kind, OverlayKind::FormText | OverlayKind::FormCheck | OverlayKind::FormRadio | OverlayKind::FormDropdown) {
                        fdf.push_str(&format!("<< /T (Field_{}_p{}) /V ({}) /Checked {} >>\n", i + 1, ov.page_index + 1, ov.label, ov.checked));
                    }
                }
                fdf.push_str("] >> >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n");
                let _ = fs::write(&save_path, fdf.as_bytes());
                state.status_msg = Some(format!("FDF 내보내기 완료: {}", save_path));
                state.refresh_view(hwnd, false);
            }
        }
        IDM_FORM_IMPORT_FDF => {
            if let Some(open_path) = open_file_dialog(hwnd) {
                if let Ok(content) = fs::read_to_string(&open_path) {
                    for (line, ov) in content.lines().filter(|l| l.starts_with("<< /T")).zip(
                        state.overlays.iter_mut().filter(|o| matches!(o.kind, OverlayKind::FormText | OverlayKind::FormCheck | OverlayKind::FormRadio | OverlayKind::FormDropdown)),
                    ) {
                        if let Some(v_start) = line.find("/V (") {
                            if let Some(v_end) = line[v_start + 4..].find(')') {
                                ov.label = line[v_start + 4..v_start + 4 + v_end].to_string();
                            }
                        }
                        ov.checked = line.contains("/Checked true");
                    }
                    state.status_msg = Some("FDF 양식 데이터 가져오기 완료".to_string());
                    state.refresh_view(hwnd, true);
                }
            }
        }
        IDM_FORM_RESET_DATA => {
            for ov in state.overlays.iter_mut() {
                match ov.kind {
                    OverlayKind::FormText => ov.label.clear(),
                    OverlayKind::FormDropdown => ov.label = String::from("선택 1: 승인"),
                    OverlayKind::FormCheck | OverlayKind::FormRadio => ov.checked = false,
                    _ => {}
                }
            }
            state.input_mode = InputMode::Normal;
            state.status_msg = Some("모든 양식 필드 데이터가 초기화되었습니다".to_string());
            state.refresh_view(hwnd, true);
        }

        IDM_SEC_REDACT_TOOL => { state.active_tool = ActiveTool::RedactBox; state.refresh_view(hwnd, false); }
        IDM_SEC_SEARCH_REDACT => {
            if !state.doc.is_null() {
                state.input_mode = InputMode::SearchRedactPrompt;
                state.search_query.clear();
                state.refresh_view(hwnd, false);
            }
        }
        IDM_THEME_DARK_DOC => state.toggle_dark_mode(hwnd),
        IDM_HELP_SHORTCUTS => state.show_help_dialog(hwnd),
        _ => {}
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;

    match msg {
        windows_sys::Win32::UI::WindowsAndMessaging::WM_CAPTURECHANGED => {
            if !state_ptr.is_null() { (&mut *state_ptr).resizing_sidebar = false; }
            0
        }
        windows_sys::Win32::UI::WindowsAndMessaging::WM_SETFOCUS |
        windows_sys::Win32::UI::WindowsAndMessaging::WM_KILLFOCUS => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                state.window_focused = msg == windows_sys::Win32::UI::WindowsAndMessaging::WM_SETFOCUS;
                state.search_caret_visible = true;
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        windows_sys::Win32::UI::WindowsAndMessaging::WM_TIMER => {
            if wparam == 1 && !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if state.show_sidebar && state.sidebar_tab == SidebarTab::Search && state.input_mode == InputMode::Search {
                    state.search_caret_visible = !state.search_caret_visible;
                    let rect = RECT { left: 0, top: state.top_bar_offset(), right: state.sidebar_width, bottom: state.top_bar_offset() + SIDEBAR_TAB_HEIGHT + 56 };
                    InvalidateRect(hwnd, &rect, 0);
                }
            }
            0
        }
        WM_ERASEBKGND => 1,

        WM_SETCURSOR => {
            if !state_ptr.is_null() {
                let state = &*state_ptr;
                if (lparam & 0xFFFF) as u16 == 1 {
                    let mut point: POINT = zeroed();
                    GetCursorPos(&mut point); ScreenToClient(hwnd, &mut point);
                    if state.show_sidebar && !state.is_fullscreen && (point.x - state.sidebar_width).abs() <= 5 && point.y >= state.top_bar_offset() {
                        SetCursor(LoadCursorW(null_mut(), windows_sys::Win32::UI::WindowsAndMessaging::IDC_SIZEWE));
                        return 1;
                    }
                    if state.hovered_link.is_some() {
                        SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                        return 1;
                    } else if state.active_tool == ActiveTool::TextSelect {
                        SetCursor(LoadCursorW(null_mut(), IDC_IBEAM));
                        return 1;
                    }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        WM_COMMAND => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                handle_menu_command(hwnd, (wparam & 0xFFFF) as u16, state);
            }
            0
        }

        WM_SIZE => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                state.update_toolbar_layout(hwnd);
                let width = (lparam & 0xFFFF) as i16 as i32;
                let height = ((lparam >> 16) & 0xFFFF) as i16 as i32;
                state.sidebar_width = state.sidebar_width.min((width - 160).max(200));
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
                    if state.confirm_unsaved(hwnd) && state.load_file(&path).is_ok() {
                        state.refresh_view(hwnd, true);
                    }
                }
                DragFinish(hdrop);
            }
            0
        }

        WM_RBUTTONDOWN => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if state.text_selection.is_some() {
                    let hpopup = CreatePopupMenu();
                    let w_copy = to_wstring("복사 (Ctrl+C)");
                    let w_hl = to_wstring("형광펜 강조 추가");
                    let w_sch = to_wstring("문서에서 이 단어 검색");
                    AppendMenuW(hpopup, MF_STRING, IDM_CTX_COPY as usize, w_copy.as_ptr());
                    AppendMenuW(hpopup, MF_STRING, IDM_CTX_HIGHLIGHT as usize, w_hl.as_ptr());
                    AppendMenuW(hpopup, MF_STRING, IDM_CTX_SEARCH as usize, w_sch.as_ptr());

                    let mut pt = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut pt);
                    let cmd = TrackPopupMenu(hpopup, TPM_RETURNCMD | TPM_NONOTIFY, pt.x, pt.y, 0, hwnd, null());
                    DeleteObject(hpopup as _);

                    if cmd == IDM_CTX_COPY as i32 {
                        state.copy_text_to_clipboard(hwnd);
                    } else if cmd == IDM_CTX_HIGHLIGHT as i32 {
                        if let Some(ref sel) = state.text_selection.clone() {
                            for r in &sel.rects {
                                state.overlays.push(UserOverlay {
                                    page_index: sel.page_index,
                                    kind: OverlayKind::Highlight,
                                    x1: (r.left / state.cur_page_w_pt).clamp(0.0, 1.0),
                                    y1: (1.0 - r.top / state.cur_page_h_pt).clamp(0.0, 1.0),
                                    x2: (r.right / state.cur_page_w_pt).clamp(0.0, 1.0),
                                    y2: (1.0 - r.bottom / state.cur_page_h_pt).clamp(0.0, 1.0),
                                    points: Vec::new(),
                                    label: String::new(),
                                    checked: false,
                                });
                            }
                            state.status_msg = Some("선택 영역에 형광펜 강조 주석이 추가되었습니다".to_string());
                            state.refresh_view(hwnd, false);
                        }
                    } else if cmd == IDM_CTX_SEARCH as i32 {
                        if let Some(ref sel) = state.text_selection.clone() {
                            let query: String = sel.text.chars().take(24).collect();
                            if !query.trim().is_empty() {
                                state.search_query = query.trim().to_string();
                                state.input_mode = InputMode::Normal;
                                state.perform_search(true);
                                state.refresh_view(hwnd, true);
                            }
                        }
                    }
                }
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
                let client_h = rc.bottom - rc.top;
                if state.show_sidebar && !state.is_fullscreen && (click_x - state.sidebar_width).abs() <= 5
                    && click_y >= state.top_bar_offset() && click_y < client_h - state.bottom_bar_offset() {
                    state.resizing_sidebar = true;
                    SetCapture(hwnd);
                    return 0;
                }

                if !state.is_fullscreen && click_y < state.toolbar_height {
                    if let Some(cmd_id) = hit_test_toolbar(state, click_x, click_y) {
                        handle_menu_command(hwnd, cmd_id, state);
                    }
                    return 0;
                }
                if !state.is_fullscreen && click_y >= client_h - state.statusbar_height {
                    if let Some(cmd_id) = hit_test_status_bar(state,click_x,click_y-(client_h-state.statusbar_height)) {
                        handle_menu_command(hwnd, cmd_id, state);
                    }
                    return 0;
                }

                let top_off = state.top_bar_offset();
                if state.show_sidebar && click_x < state.sidebar_width {
                    if click_y >= top_off && click_y < top_off + SIDEBAR_TAB_HEIGHT {
                        if click_x >= state.sidebar_width - 34 {
                            state.show_sidebar = false;
                            if state.input_mode == InputMode::Search { state.input_mode = InputMode::Normal; }
                            state.refresh_view(hwnd, true);
                        } else if click_x < 123 {
                            state.sidebar_tab = SidebarTab::Thumbnails;
                            state.input_mode = InputMode::Normal;
                            InvalidateRect(hwnd, null(), 0);
                        } else {
                            state.sidebar_tab = SidebarTab::Search;
                            state.input_mode = InputMode::Search;
                            InvalidateRect(hwnd, null(), 0);
                        }
                        return 0;
                    }
                    let content_top = top_off + SIDEBAR_TAB_HEIGHT + 4;
                    if click_y >= content_top {
                        match state.sidebar_tab {
                            SidebarTab::Search => {
                                let y = click_y - content_top;
                                if (12..48).contains(&y) {
                                    state.input_mode = InputMode::Search;
                                    state.search_caret_visible = true;
                                    state.refresh_view(hwnd, false);
                                } else if (56..96).contains(&y) {
                                    state.step_search_match(click_x >= 100, hwnd);
                                }
                            }
                            SidebarTab::Bookmarks => {
                                let row = ((click_y - content_top) / BOOKMARK_ROW_HEIGHT) as usize;
                                if let Some(bm) = state.bookmarks.get(state.bookmark_scroll + row) {
                                    if bm.page_index >= 0 {
                                        state.go_to_page(bm.page_index, hwnd);
                                    }
                                }
                            }
                            SidebarTab::Thumbnails => {
                                let slot = ((click_y - content_top) / THUMB_ITEM_HEIGHT) as usize;
                                let target_page = (state.thumb_scroll + slot) as i32;
                                if target_page >= 0 && target_page < state.page_count {
                                    state.go_to_page(target_page, hwnd);
                                }
                            }
                        }
                    }
                    return 0;
                }

                if state.active_tool != ActiveTool::TextSelect && !state.record_edit() { return 0; }
                if state.active_tool == ActiveTool::TextSelect {
                    if let Some((page_idx, char_idx)) = state.get_char_index_at_client_pos(click_x, click_y) {
                        state.text_selection = state.build_text_selection(page_idx, char_idx, char_idx);
                    } else {
                        state.text_selection = None;
                    }
                    state.is_dragging = true;
                    state.mouse_down_x = click_x;
                    state.mouse_down_y = click_y;
                    SetCapture(hwnd);
                    InvalidateRect(hwnd, null(), 0);
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

                    for (idx, ov) in state.overlays.iter().enumerate().rev() {
                        if ov.page_index == state.current_page && matches!(ov.kind, OverlayKind::SignatureImage(_)) {
                            let x_min = ov.x1.min(ov.x2);
                            let x_max = ov.x1.max(ov.x2);
                            let y_min = ov.y1.min(ov.y2);
                            let y_max = ov.y1.max(ov.y2);
                            if (nx - x_max).abs() < 0.035 && (ny - y_max).abs() < 0.035 {
                                state.selected_overlay_idx = Some(idx);
                                state.is_resizing_overlay = true;
                                state.is_dragging = true;
                                SetCapture(hwnd);
                                state.status_msg = Some("서명/인장 크기 조절 중: 마우스를 드래그하세요".to_string());
                                update_window_title(hwnd, state);
                                return 0;
                            }
                            if nx >= x_min && nx <= x_max && ny >= y_min && ny <= y_max && state.active_tool == ActiveTool::Pan {
                                state.selected_overlay_idx = Some(idx);
                                state.is_moving_overlay = true;
                                state.is_dragging = true;
                                state.drag_start_x = click_x;
                                state.drag_start_y = click_y;
                                SetCapture(hwnd);
                                state.status_msg = Some("서명/인장 위치 이동 중".to_string());
                                update_window_title(hwnd, state);
                                return 0;
                            }
                        }
                    }

                    let mut clicked_radio_page = None;
                    let mut clicked_radio_idx = None;
                    for (idx, ov) in state.overlays.iter_mut().enumerate() {
                        if ov.page_index != state.current_page { continue; }
                        if (ov.kind == OverlayKind::FormCheck || ov.kind == OverlayKind::FormRadio)
                            && (nx - ov.x1).abs() < 0.04
                            && (ny - ov.y1).abs() < 0.04
                        {
                            if ov.kind == OverlayKind::FormRadio {
                                clicked_radio_page = Some(ov.page_index);
                                clicked_radio_idx = Some(idx);
                            } else {
                                ov.checked = !ov.checked;
                            }
                            state.input_mode = InputMode::Normal;
                            InvalidateRect(hwnd, null(), 0);
                            break;
                        }
                        let x_min = ov.x1.min(ov.x2);
                        let x_max = ov.x1.max(ov.x2).max(x_min + 0.18);
                        let y_min = ov.y1.min(ov.y2);
                        let y_max = ov.y1.max(ov.y2).max(y_min + 0.035);
                        if nx >= x_min && nx <= x_max && ny >= y_min && ny <= y_max && state.active_tool == ActiveTool::Pan {
                            if ov.kind == OverlayKind::FormDropdown {
                                ov.label = match ov.label.as_str() {
                                    "선택 1: 승인" => String::from("선택 2: 검토중"),
                                    "선택 2: 검토중" => String::from("선택 3: 반려"),
                                    _ => String::from("선택 1: 승인"),
                                };
                                InvalidateRect(hwnd, null(), 0);
                                return 0;
                            } else if ov.kind == OverlayKind::FormText {
                                state.input_mode = InputMode::EditFormField(idx);
                                state.ignore_next_char = false;
                                update_window_title(hwnd, state);
                                InvalidateRect(hwnd, null(), 0);
                                return 0;
                            }
                        }
                    }
                    if let (Some(p_idx), Some(sel_i)) = (clicked_radio_page, clicked_radio_idx) {
                        for (i, ov) in state.overlays.iter_mut().enumerate() {
                            if ov.page_index == p_idx && ov.kind == OverlayKind::FormRadio {
                                ov.checked = i == sel_i;
                            }
                        }
                        return 0;
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
                    let nx = ((click_x - state.last_draw_x) as f64 / state.bmp_width as f64).clamp(0.0, 1.0);
                    let ny = ((click_y - state.last_draw_y) as f64 / state.bmp_height as f64).clamp(0.0, 1.0);
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
                if state.resizing_sidebar {
                    let mut rect: RECT = zeroed(); GetClientRect(hwnd, &mut rect);
                    state.sidebar_width = cur_x.clamp(200, (rect.right - 160).max(200));
                    state.refresh_view(hwnd, true);
                    return 0;
                }

                if !state.is_dragging {
                    if !state.is_fullscreen {
                        let new_hover = if cur_y < state.toolbar_height { hit_test_toolbar(state, cur_x, cur_y) } else { None };
                        if new_hover != state.hover_tb_id {
                            state.hover_tb_id = new_hover;
                            InvalidateRect(hwnd, null(), 0);
                        }
                    }
                    let new_link = if state.active_tool == ActiveTool::Pan && cur_x >= state.sidebar_offset() {
                        state.detect_link_at_client_pos(cur_x, cur_y)
                    } else {
                        None
                    };
                    if new_link != state.hovered_link {
                        state.hovered_link = new_link;
                        InvalidateRect(hwnd, null(), 0);
                    }
                } else {
                    if state.is_resizing_overlay {
                        if let Some(idx) = state.selected_overlay_idx {
                            if state.bmp_width > 0 && state.bmp_height > 0 {
                                let nx = ((cur_x - state.last_draw_x) as f64 / state.bmp_width as f64).clamp(0.02, 0.99);
                                let ny = ((cur_y - state.last_draw_y) as f64 / state.bmp_height as f64).clamp(0.02, 0.99);
                                if let Some(ov) = state.overlays.get_mut(idx) {
                                    ov.x2 = nx.max(ov.x1 + 0.03);
                                    ov.y2 = ny.max(ov.y1 + 0.025);
                                }
                                InvalidateRect(hwnd, null(), 0);
                            }
                        }
                        return 0;
                    }
                    if state.is_moving_overlay {
                        if let Some(idx) = state.selected_overlay_idx {
                            if state.bmp_width > 0 && state.bmp_height > 0 {
                                let dx = (cur_x - state.drag_start_x) as f64 / state.bmp_width as f64;
                                let dy = (cur_y - state.drag_start_y) as f64 / state.bmp_height as f64;
                                state.drag_start_x = cur_x;
                                state.drag_start_y = cur_y;
                                if let Some(ov) = state.overlays.get_mut(idx) {
                                    let w = (ov.x2 - ov.x1).abs();
                                    let h = (ov.y2 - ov.y1).abs();
                                    ov.x1 = (ov.x1 + dx).clamp(0.0, (1.0 - w).max(0.0));
                                    ov.y1 = (ov.y1 + dy).clamp(0.0, (1.0 - h).max(0.0));
                                    ov.x2 = ov.x1 + w;
                                    ov.y2 = ov.y1 + h;
                                }
                                InvalidateRect(hwnd, null(), 0);
                            }
                        }
                        return 0;
                    }
                    if state.active_tool == ActiveTool::TextSelect {
                        if let Some(ref existing) = state.text_selection.clone() {
                            if let Some((page_idx, cur_char)) = state.get_char_index_at_client_pos(cur_x, cur_y) {
                                if page_idx == existing.page_index {
                                    state.text_selection = state.build_text_selection(page_idx, existing.anchor_char, cur_char);
                                    InvalidateRect(hwnd, null(), 0);
                                }
                            }
                        }
                        return 0;
                    }
                    if state.active_tool == ActiveTool::Pan {
                        state.pan_x += cur_x - state.drag_start_x;
                        state.pan_y += cur_y - state.drag_start_y;
                        state.drag_start_x = cur_x;
                        state.drag_start_y = cur_y;
                    } else if state.active_tool == ActiveTool::Pencil && state.bmp_width > 0 {
                        let nx = ((cur_x - state.last_draw_x) as f64 / state.bmp_width as f64).clamp(0.0, 1.0);
                        let ny = ((cur_y - state.last_draw_y) as f64 / state.bmp_height as f64).clamp(0.0, 1.0);
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
                if state.resizing_sidebar {
                    state.resizing_sidebar = false;
                    ReleaseCapture();
                    return 0;
                }
                if state.is_dragging {
                    state.is_dragging = false;
                    ReleaseCapture();

                    let end_x = (lparam & 0xFFFF) as i16 as i32;
                    let end_y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                    if state.is_resizing_overlay || state.is_moving_overlay {
                        state.is_resizing_overlay = false;
                        state.is_moving_overlay = false;
                        InvalidateRect(hwnd, null(), 0);
                        return 0;
                    }
                    if state.active_tool == ActiveTool::TextSelect {
                        update_window_title(hwnd, state);
                        InvalidateRect(hwnd, null(), 0);
                        return 0;
                    }
                    if state.active_tool == ActiveTool::Pan {
                        let move_dist = (end_x - state.mouse_down_x).abs().max((end_y - state.mouse_down_y).abs());
                        if move_dist < 4 {
                            if let Some(target) = state.detect_link_at_client_pos(end_x, end_y) {
                                state.execute_link_target(&target, hwnd);
                            }
                        }
                        return 0;
                    }

                    if state.bmp_width > 0 && state.bmp_height > 0 {
                        let x1 = ((state.drag_start_x - state.last_draw_x) as f64 / state.bmp_width as f64).clamp(0.0, 1.0);
                        let y1 = ((state.drag_start_y - state.last_draw_y) as f64 / state.bmp_height as f64).clamp(0.0, 1.0);
                        let x2 = ((end_x - state.last_draw_x) as f64 / state.bmp_width as f64).clamp(0.0, 1.0);
                        let y2 = ((end_y - state.last_draw_y) as f64 / state.bmp_height as f64).clamp(0.0, 1.0);

                        match state.active_tool {
                            ActiveTool::PlaceSignature => {
                                if let Some(sig_idx) = state.active_sig_idx {
                                    let mut rx1 = x1.min(x2);
                                    let mut ry1 = y1.min(y2);
                                    let mut rx2 = x1.max(x2);
                                    let mut ry2 = y1.max(y2);
                                    if (rx2 - rx1) < 0.02 || (ry2 - ry1) < 0.02 {
                                        rx1 = x1.clamp(0.0, 0.82);
                                        ry1 = y1.clamp(0.0, 0.88);
                                        rx2 = (rx1 + 0.16).min(0.98);
                                        ry2 = (ry1 + 0.10).min(0.98);
                                    }
                                    state.overlays.push(UserOverlay {
                                        page_index: state.current_page,
                                        kind: OverlayKind::SignatureImage(sig_idx),
                                        x1: rx1,
                                        y1: ry1,
                                        x2: rx2,
                                        y2: ry2,
                                        points: Vec::new(),
                                        label: String::from("SIGNATURE"),
                                        checked: true,
                                    });
                                    state.active_tool = ActiveTool::Pan;
                                    state.status_msg = Some("서명/인장 날인 완료! (드래그로 이동, 우측 하단 ■ 핸들로 크기 조절)".to_string());
                                }
                            }
                            ActiveTool::Pencil => {
                                if state.pencil_trace.len() >= 2 {
                                    state.overlays.push(UserOverlay {
                                        page_index: state.current_page,
                                        kind: OverlayKind::Pencil,
                                        x1, y1, x2, y2,
                                        points: state.pencil_trace.clone(),
                                        label: String::new(),
                                        checked: false,
                                    });
                                }
                            }
                            _ => {
                                let (kind, label) = match state.active_tool {
                                    ActiveTool::Highlight => (OverlayKind::Highlight, ""),
                                    ActiveTool::Underline => (OverlayKind::Underline, ""),
                                    ActiveTool::Strikeout => (OverlayKind::Strikeout, ""),
                                    ActiveTool::RedactBox => (OverlayKind::Redaction, "REDACTED"),
                                    ActiveTool::FormTextField => (OverlayKind::FormText, ""),
                                    ActiveTool::FormCheckBox => (OverlayKind::FormCheck, "Check"),
                                    ActiveTool::FormRadio => (OverlayKind::FormRadio, "Radio"),
                                    ActiveTool::FormDropdown => (OverlayKind::FormDropdown, "선택 1: 승인"),
                                    ActiveTool::FormDate => (OverlayKind::FormText, "2026-11-16"),
                                    _ => (OverlayKind::Highlight, ""),
                                };
                                state.overlays.push(UserOverlay {
                                    page_index: state.current_page,
                                    kind,
                                    x1, y1, x2, y2,
                                    points: Vec::new(),
                                    label: label.to_string(),
                                    checked: false,
                                });
                                if matches!(state.active_tool, ActiveTool::FormTextField | ActiveTool::FormDate) {
                                    let new_idx = state.overlays.len() - 1;
                                    state.input_mode = InputMode::EditFormField(new_idx);
                                    state.active_tool = ActiveTool::Pan;
                                }
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
                let mut pt = POINT { x: (lparam & 0xFFFF) as i16 as i32, y: ((lparam >> 16) & 0xFFFF) as i16 as i32 };
                ScreenToClient(hwnd, &mut pt);

                if !ctrl_pressed && state.show_sidebar && pt.x >= 0 && pt.x < state.sidebar_width {
                    state.scroll_sidebar(delta, hwnd);
                    return 0;
                }
                if ctrl_pressed {
                    state.change_zoom(if delta > 0 { 1.15 } else { 1.0 / 1.15 }, hwnd);
                } else {
                    state.scroll_or_turn_page(delta < 0, hwnd);
                }
            }
            0
        }

        WM_CHAR => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if matches!(state.input_mode,InputMode::EditFormField(_)) && !state.record_edit() { return 0; }
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
                                if state.input_mode == InputMode::Search { state.perform_search(true); }
                                state.refresh_view(hwnd, true);
                            }
                        }
                    }
                    InputMode::EditFormField(idx) => {
                        if ch >= 0x20 {
                            if let Some(c) = char::from_u32(ch) {
                                if let Some(ov) = state.overlays.get_mut(idx) {
                                    ov.label.push(c);
                                }
                                update_window_title(hwnd, state);
                                InvalidateRect(hwnd, null(), 0);
                            }
                        }
                    }
                    InputMode::AddTextPrompt | InputMode::CustomWatermarkPrompt | InputMode::CreateSealPrompt => {
                        if ch >= 0x20 {
                            if let Some(c) = char::from_u32(ch) {
                                state.text_prompt_buf.push(c);
                                update_window_title(hwnd, state);
                            }
                        }
                    }
                    InputMode::GotoPage | InputMode::PageOperation(_) => {
                        if let Some(c) = char::from_u32(ch) {
                            if c.is_ascii_digit() || (matches!(state.input_mode,InputMode::PageOperation(_)) && matches!(c,','|'-'|' ')) {
                                state.goto_input.push(c);
                                update_window_title(hwnd, state);
                            }
                        }
                    }
                    InputMode::Normal => {}
                }
            }
            0
        }

        WM_KEYDOWN => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if GetKeyState(VK_CONTROL as i32) < 0 {
                    if wparam == 0x5a || wparam == 0x59 {
                        state.undo_redo(wparam == 0x59);
                        state.refresh_view(hwnd,true);
                        return 0;
                    }
                    if wparam == 0x50 { handle_menu_command(hwnd,IDM_FILE_PRINT,state); return 0; }
                }
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
                        VK_TAB => {
                            if let InputMode::EditFormField(cur_idx) = state.input_mode {
                                let form_indices: Vec<usize> = state
                                    .overlays
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, o)| o.page_index == state.current_page && o.kind == OverlayKind::FormText)
                                    .map(|(i, _)| i)
                                    .collect();
                                if !form_indices.is_empty() {
                                    let pos = form_indices.iter().position(|&i| i == cur_idx).unwrap_or(0);
                                    state.input_mode = InputMode::EditFormField(form_indices[(pos + 1) % form_indices.len()]);
                                    update_window_title(hwnd, state);
                                    InvalidateRect(hwnd, null(), 0);
                                }
                            }
                        }
                        VK_RETURN => {
                            let mode = state.input_mode;
                            state.input_mode = InputMode::Normal;
                            match mode {
                                InputMode::PageOperation(action) => {
                                    let result=state.apply_page_operation(hwnd,action);
                                    state.status_msg=Some(match result {Ok(())=>"페이지 작업 완료".into(),Err(err)=>err});
                                }
                                InputMode::GotoPage => {
                                    if let Ok(p) = state.goto_input.trim().parse::<i32>() {
                                        if p >= 1 && p <= state.page_count {
                                            state.go_to_page(p - 1, hwnd);
                                        }
                                    }
                                    state.goto_input.clear();
                                }
                                InputMode::AddTextPrompt => {
                                    if !state.record_edit() { return 0; }
                                    state.history.native_cache = None;
                                    let txt = state.text_prompt_buf.trim().to_string();
                                    state.text_prompt_buf.clear();
                                    if !txt.is_empty() && !state.doc.is_null() {
                                        let page = FPDF_LoadPage(state.doc, state.current_page);
                                        if !page.is_null() {
                                            let h = FPDF_GetPageHeight(page);
                                            AppState::insert_pdf_text_object(state.doc, page, &txt, 60.0, h - 80.0, 14.0);
                                            FPDFPage_GenerateContent(page);
                                            FPDF_ClosePage(page);
                                        }
                                        state.thumb_cache.remove(&state.current_page);
                                    }
                                }
                                InputMode::CustomWatermarkPrompt => {
                                    if !state.record_edit() { return 0; }
                                    let wm = state.text_prompt_buf.trim().to_string();
                                    state.text_prompt_buf.clear();
                                    if !wm.is_empty() { state.watermark_text = Some(wm); }
                                }
                                InputMode::CreateSealPrompt => {
                                    let seal_name = state.text_prompt_buf.trim().to_string();
                                    state.text_prompt_buf.clear();
                                    if !seal_name.is_empty() {
                                        if let Some(sig_data) = create_red_seal_stamp(hwnd, &seal_name) {
                                            state.signatures.push(sig_data);
                                            state.active_sig_idx = Some(state.signatures.len() - 1);
                                            state.active_tool = ActiveTool::PlaceSignature;
                                            state.status_msg = Some(format!("'{}' 전자 인장 생성 완료! 원하는 위치에 드래그하여 날인하세요.", seal_name));
                                        }
                                    }
                                }
                                InputMode::EditFormField(_) => {
                                    state.status_msg = Some("양식 필드 입력 완료".to_string());
                                }
                                InputMode::SearchRedactPrompt => state.apply_search_redaction(hwnd),
                                InputMode::Search => {
                                    if state.search_matches.is_empty() && !state.search_query.is_empty() {
                                        state.perform_search(true);
                                    }
                                }
                                InputMode::Normal => {}
                            }
                            state.refresh_view(hwnd, true);
                        }
                        VK_BACK => {
                            match state.input_mode {
                                InputMode::GotoPage | InputMode::PageOperation(_) => { state.goto_input.pop(); }
                                InputMode::EditFormField(idx) => {
                                    if let Some(ov) = state.overlays.get_mut(idx) { ov.label.pop(); }
                                }
                                InputMode::AddTextPrompt | InputMode::CustomWatermarkPrompt | InputMode::CreateSealPrompt => {
                                    state.text_prompt_buf.pop();
                                }
                                InputMode::Search | InputMode::SearchRedactPrompt => {
                                    state.search_query.pop();
                                    if state.input_mode == InputMode::Search { state.perform_search(true); }
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
                        if ctrl_pressed { handle_menu_command(hwnd, IDM_FILE_SAVE, state); }
                        else { state.save_current_page_image(hwnd); }
                    }
                    0x57 => state.toggle_fit_width(hwnd),
                    0x44 => state.toggle_spread_mode(hwnd),
                    0x49 => state.show_metadata_dialog(hwnd),
                    0x48 | VK_F1 => state.show_help_dialog(hwnd),
                    0x56 => state.toggle_dark_mode(hwnd),
                    VK_F11 => state.toggle_fullscreen(hwnd),
                    0x47 => {
                        if !state.doc.is_null() && state.page_count > 0 {
                            state.input_mode = InputMode::GotoPage;
                            state.goto_input.clear();
                            if !ctrl_pressed { state.ignore_next_char = true; }
                            update_window_title(hwnd, state);
                        }
                    }
                    0x42 => handle_menu_command(hwnd, IDM_VIEW_THUMBNAILS, state),
                    VK_OEM_4 | VK_OEM_6 => {},
                    0x46 => {
                        if !state.doc.is_null() {
                            handle_menu_command(hwnd, IDM_TOOL_SEARCH, state);
                            if !ctrl_pressed { state.ignore_next_char = true; }
                            update_window_title(hwnd, state);
                        }
                    }
                    VK_F3 => state.step_search_match(!shift_pressed, hwnd),
                    0x4F => handle_menu_command(hwnd, IDM_FILE_OPEN, state),
                    VK_ESCAPE => {
                        if state.text_selection.is_some() {
                            state.text_selection = None;
                            InvalidateRect(hwnd, null(), 0);
                        } else if state.active_tool != ActiveTool::Pan {
                            state.active_tool = ActiveTool::Pan;
                            InvalidateRect(hwnd, null(), 0);
                        } else if !state.search_query.is_empty() {
                            state.search_query.clear();
                            state.search_matches.clear();
                            state.refresh_view(hwnd, true);
                        } else if state.is_fullscreen {
                            state.toggle_fullscreen(hwnd);
                        } else {
                            if state.confirm_unsaved(hwnd) { PostQuitMessage(0); }
                        }
                    }
                    _ => {}
                }
            }
            0
        }

        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let screen_hdc = BeginPaint(hwnd, &mut ps);

            let mut rc: RECT = zeroed();
            GetClientRect(hwnd, &mut rc);
            let client_w = (rc.right - rc.left).max(1);
            let client_h = (rc.bottom - rc.top).max(1);

            let hdc = CreateCompatibleDC(screen_hdc);
            let hbm = CreateCompatibleBitmap(screen_hdc, client_w, client_h);
            let old_bm = SelectObject(hdc, hbm as _);

            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let bg_color = if state.dark_mode { 0x00141414 } else { state.theme_bg_color };
                state.update_toolbar_layout(hwnd);
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
                        hdc, x, y, state.bmp_width, state.bmp_height, 0, 0, state.bmp_width, state.bmp_height,
                        state.pixels.as_ptr() as *const c_void, &bmi, DIB_RGB_COLORS, SRCCOPY,
                    );

                    if let Some(ref sel) = state.text_selection {
                        let target_info = if sel.page_index == state.current_page {
                            Some((sel.page_index, state.p1_rect_in_bmp))
                        } else if let Some(p2_rc) = state.p2_rect_in_bmp {
                            if sel.page_index == state.current_page + 1 { Some((sel.page_index, p2_rc)) } else { None }
                        } else {
                            None
                        };
                        if let Some((p_idx, p_rc)) = target_info {
                            let page = FPDF_LoadPage(state.doc, p_idx);
                            if !page.is_null() {
                                let pw = p_rc.right - p_rc.left;
                                let ph = p_rc.bottom - p_rc.top;
                                let src_dc = CreateCompatibleDC(hdc);
                                let src_bm = CreateCompatibleBitmap(screen_hdc, 1, 1);
                                let old_src_bm = SelectObject(src_dc, src_bm as _);
                                let fill_rc = RECT { left: 0, top: 0, right: 1, bottom: 1 };
                                let fill_br = CreateSolidBrush(0x00FF9933);
                                FillRect(src_dc, &fill_rc, fill_br);
                                DeleteObject(fill_br as _);

                                let bf = BLENDFUNCTION { BlendOp: 0, BlendFlags: 0, SourceConstantAlpha: 96, AlphaFormat: 0 };
                                for r in &sel.rects {
                                    let (mut dx1, mut dy1, mut dx2, mut dy2) = (0, 0, 0, 0);
                                    FPDF_PageToDevice(page, 0, 0, pw, ph, state.rotation, r.left, r.top, &mut dx1, &mut dy1);
                                    FPDF_PageToDevice(page, 0, 0, pw, ph, state.rotation, r.right, r.bottom, &mut dx2, &mut dy2);
                                    let rx1 = x + p_rc.left + dx1.min(dx2);
                                    let ry1 = y + p_rc.top + dy1.min(dy2);
                                    let rw = (dx1.max(dx2) - dx1.min(dx2)).max(1);
                                    let rh = (dy1.max(dy2) - dy1.min(dy2)).max(1);
                                    AlphaBlend(hdc as *mut c_void, rx1, ry1, rw, rh, src_dc as *mut c_void, 0, 0, 1, 1, bf);
                                }
                                SelectObject(src_dc, old_src_bm);
                                DeleteObject(src_bm as _);
                                DeleteDC(src_dc);
                                FPDF_ClosePage(page);
                            }
                        }
                    }

                    draw_page_overlays_and_guides(hdc as *mut c_void, x, y, client_w, client_h, state);
                }

                if state.show_sidebar {
                    draw_left_sidebar_panel(hdc as *mut c_void, client_h, state);
                }
                if !state.is_fullscreen {
                    draw_top_toolbar(hdc as *mut c_void, client_w, state);
                    draw_bottom_status_bar(hdc as *mut c_void, client_w, client_h, state);
                }
            }

            BitBlt(screen_hdc, 0, 0, client_w, client_h, hdc, 0, 0, SRCCOPY);
            SelectObject(hdc, old_bm);
            DeleteObject(hbm as _);
            DeleteDC(hdc);

            EndPaint(hwnd, &ps);
            0
        }

        WM_CLOSE => {
            if state_ptr.is_null() || (&mut *state_ptr).confirm_unsaved(hwnd) {
                DefWindowProcW(hwnd,msg,wparam,lparam)
            } else { 0 }
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
    let initial_pdf = if args.len() > 1 { args[1].clone() } else { "test- .pdf".to_string() };

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
        wc.hIcon = LoadIconW(hinstance, 1usize as *const u16);
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
            1480,
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
        windows_sys::Win32::UI::WindowsAndMessaging::SetTimer(hwnd, 1, 500, None);
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

