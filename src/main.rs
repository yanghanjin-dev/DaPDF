#![windows_subsystem = "windows"]

use std::env;
use std::ffi::{c_void, CString};
use std::fs::{self, File};
use std::io::Write;
use std::mem::{size_of, zeroed};
use std::os::raw::{c_char, c_double, c_int, c_ulong, c_ushort};
use std::path::Path;
use std::ptr::{copy_nonoverlapping, null, null_mut};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, GetMonitorInfoW,
        GetStockObject, InvalidateRect, MonitorFromWindow, SelectObject, SetBkMode, SetTextColor,
        StretchDIBits, TextOutW, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DEFAULT_GUI_FONT,
        DIB_RGB_COLORS, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
    },
    System::{
        DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
        LibraryLoader::GetModuleHandleW,
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    },
    UI::{
        Controls::Dialogs::{
            GetOpenFileNameW, GetSaveFileNameW, OFN_ENABLESIZING, OFN_EXPLORER, OFN_FILEMUSTEXIST,
            OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
        },
        HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
        Input::KeyboardAndMouse::{
            GetKeyState, ReleaseCapture, SetCapture, VK_BACK, VK_CONTROL, VK_DOWN, VK_ESCAPE,
            VK_F1, VK_F11, VK_F3, VK_LEFT, VK_NEXT, VK_OEM_4, VK_OEM_6, VK_OEM_MINUS, VK_OEM_PLUS,
            VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_UP,
        },
        Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, HDROP},
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW,
            GetWindowLongPtrW, GetWindowPlacement, LoadCursorW, MessageBoxW, PostQuitMessage,
            RegisterClassW, SetWindowLongPtrW, SetWindowPlacement, SetWindowPos, SetWindowTextW,
            ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, GWL_STYLE,
            HWND_TOP, IDC_ARROW, MB_ICONINFORMATION, MB_OK, MSG, SWP_FRAMECHANGED,
            SWP_NOOWNERZORDER, SWP_NOZORDER, SW_SHOW, WINDOWPLACEMENT, WM_CHAR, WM_DESTROY,
            WM_DROPFILES, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
            WM_MOUSEWHEEL, WM_PAINT, WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const CF_UNICODETEXT: u32 = 13;
const SIDEBAR_WIDTH: i32 = 280;
const BOOKMARK_ROW_HEIGHT: i32 = 26;
const SPREAD_GAP: i32 = 16;

const FPDF_ANNOT: c_int = 0x01;
const FPDF_LCD_TEXT: c_int = 0x02;

#[link(name = "pdfium.dll")]
extern "C" {
    fn FPDF_InitLibrary();
    fn FPDF_DestroyLibrary();
    fn FPDF_GetLastError() -> c_ulong;
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

    fn FPDFBookmark_GetFirstChild(document: *mut c_void, bookmark: *mut c_void) -> *mut c_void;
    fn FPDFBookmark_GetNextSibling(document: *mut c_void, bookmark: *mut c_void) -> *mut c_void;
    fn FPDFBookmark_GetTitle(
        bookmark: *mut c_void,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;
    fn FPDFBookmark_GetDest(document: *mut c_void, bookmark: *mut c_void) -> *mut c_void;
    fn FPDFBookmark_GetAction(bookmark: *mut c_void) -> *mut c_void;
    fn FPDFAction_GetDest(document: *mut c_void, action: *mut c_void) -> *mut c_void;
    fn FPDFDest_GetDestPageIndex(document: *mut c_void, dest: *mut c_void) -> c_int;

    fn FPDF_GetMetaText(
        document: *mut c_void,
        tag: *const c_char,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;
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

#[derive(PartialEq, Eq, Clone, Copy)]
enum InputMode {
    Normal,
    Search,
    GotoPage,
}

struct AppState {
    doc: *mut c_void,
    pdf_bytes: Vec<u8>,
    file_name: String,
    page_count: i32,
    current_page: i32,
    spread_mode: bool,
    fit_width: bool,
    pending_align_top: bool,
    pending_align_bottom: bool,
    dark_mode: bool,
    is_fullscreen: bool,
    saved_placement: WINDOWPLACEMENT,
    rotation: i32,
    zoom: f64,
    pan_x: i32,
    pan_y: i32,
    is_dragging: bool,
    drag_start_x: i32,
    drag_start_y: i32,
    pixels: Vec<u8>,
    bmp_width: i32,
    bmp_height: i32,
    status_msg: Option<String>,

    input_mode: InputMode,
    ignore_next_char: bool,
    search_query: String,
    search_matches: Vec<SearchMatch>,
    active_match: usize,
    goto_input: String,

    bookmarks: Vec<BookmarkItem>,
    show_bookmarks: bool,
    bookmark_scroll: usize,
}

impl AppState {
    fn new() -> Self {
        let mut wp: WINDOWPLACEMENT = unsafe { zeroed() };
        wp.length = size_of::<WINDOWPLACEMENT>() as u32;

        Self {
            doc: null_mut(),
            pdf_bytes: Vec::new(),
            file_name: String::from("파일 없음 (O키: 열기, H/F1: 도움말)"),
            page_count: 0,
            current_page: 0,
            spread_mode: false,
            fit_width: false,
            pending_align_top: false,
            pending_align_bottom: false,
            dark_mode: false,
            is_fullscreen: false,
            saved_placement: wp,
            rotation: 0,
            zoom: 1.0,
            pan_x: 0,
            pan_y: 0,
            is_dragging: false,
            drag_start_x: 0,
            drag_start_y: 0,
            pixels: Vec::new(),
            bmp_width: 0,
            bmp_height: 0,
            status_msg: None,
            input_mode: InputMode::Normal,
            ignore_next_char: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            active_match: 0,
            goto_input: String::new(),
            bookmarks: Vec::new(),
            show_bookmarks: false,
            bookmark_scroll: 0,
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
        self.page_count = unsafe { FPDF_GetPageCount(self.doc) };
        self.current_page = 0;
        self.rotation = 0;
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.pending_align_top = self.fit_width;
        self.pending_align_bottom = false;
        self.pixels.clear();
        self.status_msg = None;
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
        self.search_matches.clear();
        self.active_match = 0;
        self.goto_input.clear();
        self.bookmark_scroll = 0;

        self.file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string();

        self.load_bookmarks();

        Ok(())
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

            let mut size_str = String::from("알 수 없음");
            let page = FPDF_LoadPage(self.doc, self.current_page);
            if !page.is_null() {
                let w_pt = FPDF_GetPageWidth(page);
                let h_pt = FPDF_GetPageHeight(page);
                let w_mm = w_pt * 25.4 / 72.0;
                let h_mm = h_pt * 25.4 / 72.0;
                size_str = format!(
                    "{:.1} x {:.1} pt ({:.0} x {:.0} mm)",
                    w_pt, h_pt, w_mm, h_mm
                );
                FPDF_ClosePage(page);
            }

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
            let w_caption = to_wstring(&format!("DaPDF(v{}) - 문서 메타데이터 정보", APP_VERSION));
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
[파일 및 저장 / 정보]
• O (또는 Ctrl+O) : PDF 파일 열기 (창으로 파일 드래그 앤 드롭 지원)
• S (또는 Ctrl+S) : 현재 렌더링된 페이지를 고해상도 BMP 이미지로 저장
• C (또는 Ctrl+C) : 현재 페이지 텍스트 추출 및 클립보드 복사
• I : 문서 메타데이터(제목, 저자, 생성일, 페이지 치수 등) 확인
• H / F1 : 단축키 및 조작 도움말 보기

[화면 보기 및 레이아웃]
• W : 창 너비 맞춤(Fit-to-Width) ↔ 페이지 전체 맞춤(Fit-to-Page) 전환
• D : 2페이지 양면 보기(Spread View) ↔ 1페이지 단면 보기 전환
• V (또는 검색어 없을 때 N) : 야간 독서용 다크 모드(색상 반전) 전환
• R / Shift+R : 시계 방향 / 반시계 방향 90° 페이지 회전
• F11 : 전체 화면(Fullscreen) 전환 및 복원
• Ctrl + 마우스 휠 (또는 + / -) : 확대 / 축소 (25% ~ 500%)
• 0 : 배율(100%), 회전(0°), 화면 위치 초기화
• 마우스 좌클릭 드래그 : 화면 자유 이동 (Pan)

[페이지 이동 및 목차]
• 마우스 휠 / 방향키(←,→,↑,↓) / PageUp, PageDown : 페이지 이동 및 스크롤
• G (또는 Ctrl+G) : 페이지 번호 직접 입력하여 이동 (Go-to-Page)
• B : 좌측 북마크(목차) 사이드바 열기 / 닫기 (목차 클릭 시 이동)
• [ / ] : 이전 / 다음 북마크(목차) 챕터로 바로 이동

[문서 전체 키워드 검색]
• F (또는 Ctrl+F) : 실시간 키워드 검색 모드 진입 (전체 페이지 자동 순회)
• N / F3 : 다음 검색 결과 하이라이트로 이동 (페이지 간 자동 점프)
• Shift+N / Shift+F3 : 이전 검색 결과 하이라이트로 이동
• ESC : 검색/입력 취소 → 전체화면 해제 → 프로그램 종료";

        let w_help = to_wstring(help_text);
        let w_caption = to_wstring(&format!(
            "DaPDF(v{}) Viewer - 단축키 및 조작 도움말 (H / F1)",
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
            self.show_bookmarks = true;
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
        if self.show_bookmarks {
            SIDEBAR_WIDTH
        } else {
            0
        }
    }

    fn align_vertical_pan_if_needed(&mut self, client_h: i32) {
        if self.pending_align_top {
            self.pending_align_top = false;
            if self.bmp_height > client_h - 32 {
                self.pan_y = (self.bmp_height - client_h) / 2 + 16;
            } else {
                self.pan_y = 0;
            }
        } else if self.pending_align_bottom {
            self.pending_align_bottom = false;
            if self.bmp_height > client_h - 32 {
                self.pan_y = -((self.bmp_height - client_h) / 2 + 16);
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

        unsafe {
            let page1 = FPDF_LoadPage(self.doc, self.current_page);
            if page1.is_null() {
                return;
            }

            let raw_w1 = FPDF_GetPageWidth(page1);
            let raw_h1 = FPDF_GetPageHeight(page1);
            let (w1, h1) = if self.rotation % 2 == 1 {
                (raw_h1, raw_w1)
            } else {
                (raw_w1, raw_h1)
            };

            let has_second = self.spread_mode && (self.current_page + 1 < self.page_count);
            let page2 = if has_second {
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
            let avail_h = (client_h - 32).max(50) as f64;

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

            if self.bmp_width == total_w && self.bmp_height == total_h && !self.pixels.is_empty() {
                self.align_vertical_pan_if_needed(client_h);
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
            self.align_vertical_pan_if_needed(client_h);

            let y_off1 = (total_h - target_h1) / 2;
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
            FPDFBitmap_FillRect(bitmap, 0, 0, w, h, 0xFFFFFFFF);
            FPDF_RenderPageBitmap(
                bitmap,
                page,
                0,
                0,
                w,
                h,
                self.rotation,
                FPDF_ANNOT | FPDF_LCD_TEXT,
            );

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
        let default_name = if self.spread_mode && (self.current_page + 1 < self.page_count) {
            format!(
                "{}_pages_{}-{}.bmp",
                stem,
                self.current_page + 1,
                self.current_page + 2
            )
        } else {
            format!("{}_page_{}.bmp", stem, self.current_page + 1)
        };

        if let Some(save_path) = save_image_dialog(hwnd, &default_name) {
            match write_bgra_to_bmp_file(&save_path, self.bmp_width, self.bmp_height, &self.pixels)
            {
                Ok(_) => {
                    let saved_name = Path::new(&save_path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&save_path);
                    self.status_msg = Some(format!(
                        "이미지 저장 완료: {} ({}x{}px)",
                        saved_name, self.bmp_width, self.bmp_height
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
            self.status_msg = None;
            if !self.search_query.is_empty() {
                self.perform_search(false);
            }
            self.refresh_view(hwnd, true);
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
        let client_h = rc.bottom - rc.top;

        if self.bmp_height > client_h - 16 {
            let max_pan = (self.bmp_height - client_h) / 2 + 16;
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
}

impl Drop for AppState {
    fn drop(&mut self) {
        if !self.doc.is_null() {
            unsafe { FPDF_CloseDocument(self.doc) };
        }
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

fn save_image_dialog(hwnd: HWND, default_name: &str) -> Option<String> {
    unsafe {
        let mut file_buf = vec![0u16; 1024];
        for (i, ch) in default_name.encode_utf16().enumerate().take(1000) {
            file_buf[i] = ch;
        }

        let filter: Vec<u16> = "BMP 비트맵 이미지 (*.bmp)\0*.bmp\0모든 파일 (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();
        let def_ext = to_wstring("bmp");

        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.lpstrDefExt = def_ext.as_ptr();
        ofn.Flags = OFN_EXPLORER | OFN_ENABLESIZING | OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;

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

unsafe fn draw_bookmark_sidebar(hdc: *mut c_void, client_h: i32, state: &AppState) {
    let panel_rc = RECT {
        left: 0,
        top: 0,
        right: SIDEBAR_WIDTH,
        bottom: client_h,
    };
    let panel_brush = CreateSolidBrush(0x00252526);
    FillRect(hdc, &panel_rc, panel_brush);
    DeleteObject(panel_brush as _);

    let header_rc = RECT {
        left: 0,
        top: 0,
        right: SIDEBAR_WIDTH,
        bottom: 32,
    };
    let header_brush = CreateSolidBrush(0x001E1E1E);
    FillRect(hdc, &header_rc, header_brush);
    DeleteObject(header_brush as _);

    let border_rc = RECT {
        left: SIDEBAR_WIDTH - 1,
        top: 0,
        right: SIDEBAR_WIDTH,
        bottom: client_h,
    };
    let border_brush = CreateSolidBrush(0x00454545);
    FillRect(hdc, &border_rc, border_brush);
    DeleteObject(border_brush as _);

    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT as i32);

    SetTextColor(hdc, 0x0080D8FF);
    let header_title = format!("목차 (Bookmarks: {}) [B:닫기]", state.bookmarks.len());
    let w_header: Vec<u16> = header_title.encode_utf16().collect();
    TextOutW(hdc, 10, 8, w_header.as_ptr(), w_header.len() as i32);

    if state.bookmarks.is_empty() {
        SetTextColor(hdc, 0x00AAAAAA);
        let empty_msg: Vec<u16> = "이 문서에는 목차가 없습니다.".encode_utf16().collect();
        TextOutW(hdc, 12, 46, empty_msg.as_ptr(), empty_msg.len() as i32);
    } else {
        let max_rows = ((client_h - 36) / BOOKMARK_ROW_HEIGHT).max(1) as usize;
        for (i, bm) in state
            .bookmarks
            .iter()
            .enumerate()
            .skip(state.bookmark_scroll)
            .take(max_rows)
        {
            let row_idx = i - state.bookmark_scroll;
            let y_top = 34 + (row_idx as i32) * BOOKMARK_ROW_HEIGHT;
            let is_current = bm.page_index == state.current_page
                || (state.spread_mode && bm.page_index == state.current_page + 1);

            if is_current {
                let active_rc = RECT {
                    left: 2,
                    top: y_top,
                    right: SIDEBAR_WIDTH - 2,
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
    }

    SelectObject(hdc, old_font);
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
        } else if state.spread_mode {
            format!("{} / {}p [2p양면]", state.current_page + 1, state.page_count)
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
            InputMode::GotoPage => {
                format!(
                    "{} | 📄 페이지 이동 (1~{}): {}_ [Enter:이동, ESC:취소]",
                    base, state.page_count, state.goto_input
                )
            }
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
                    format!(
                        "{} | O:열기, W:폭맞춤, D:양면, V:다크, F:검색, G:이동, H/F1:도움말",
                        base
                    )
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
        ofn.Flags = OFN_EXPLORER | OFN_ENABLESIZING | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;

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

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;

    match msg {
        WM_ERASEBKGND => 1,

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
                    if let Err(e) = state.load_file(&path) {
                        eprintln!("[DaPDF] {}", e);
                    } else {
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

                if state.show_bookmarks && click_x < SIDEBAR_WIDTH {
                    if click_y >= 34 {
                        let row = ((click_y - 34) / BOOKMARK_ROW_HEIGHT) as usize;
                        let bm_idx = state.bookmark_scroll + row;
                        if let Some(bm) = state.bookmarks.get(bm_idx) {
                            let target_page = bm.page_index;
                            if target_page >= 0 {
                                state.go_to_page(target_page, hwnd);
                            }
                        }
                    }
                    return 0;
                }

                state.is_dragging = true;
                state.drag_start_x = click_x;
                state.drag_start_y = click_y;
                SetCapture(hwnd);
            }
            0
        }

        WM_MOUSEMOVE => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if state.is_dragging {
                    let cur_x = (lparam & 0xFFFF) as i16 as i32;
                    let cur_y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
                    state.pan_x += cur_x - state.drag_start_x;
                    state.pan_y += cur_y - state.drag_start_y;
                    state.drag_start_x = cur_x;
                    state.drag_start_y = cur_y;
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
                }
            }
            0
        }

        WM_MOUSEWHEEL => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let delta = ((wparam >> 16) & 0xFFFF) as i16;
                let ctrl_pressed = (GetKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;

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

                match state.input_mode {
                    InputMode::Search => {
                        let ch = wparam as u32;
                        if ch >= 0x20 {
                            if let Some(c) = char::from_u32(ch) {
                                state.search_query.push(c);
                                state.perform_search(true);
                                state.refresh_view(hwnd, true);
                            }
                        }
                        return 0;
                    }
                    InputMode::GotoPage => {
                        let ch = wparam as u32;
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

                if state.input_mode == InputMode::GotoPage {
                    match key {
                        VK_ESCAPE => {
                            state.input_mode = InputMode::Normal;
                            state.goto_input.clear();
                            update_window_title(hwnd, state);
                        }
                        VK_RETURN => {
                            let input_str = state.goto_input.trim().to_string();
                            state.input_mode = InputMode::Normal;
                            state.goto_input.clear();

                            if let Ok(page_num) = input_str.parse::<i32>() {
                                if page_num >= 1 && page_num <= state.page_count {
                                    state.go_to_page(page_num - 1, hwnd);
                                    state.status_msg =
                                        Some(format!("{}페이지로 이동 완료", page_num));
                                } else {
                                    state.status_msg = Some(format!(
                                        "유효하지 않은 페이지 번호입니다 (1~{} 사이 입력)",
                                        state.page_count
                                    ));
                                }
                            }
                            update_window_title(hwnd, state);
                        }
                        VK_BACK => {
                            state.goto_input.pop();
                            update_window_title(hwnd, state);
                        }
                        _ => {}
                    }
                    return 0;
                }

                if state.input_mode == InputMode::Search {
                    match key {
                        VK_ESCAPE => {
                            state.input_mode = InputMode::Normal;
                            state.search_query.clear();
                            state.search_matches.clear();
                            state.refresh_view(hwnd, true);
                        }
                        VK_RETURN => {
                            state.input_mode = InputMode::Normal;
                            if state.search_matches.is_empty() && !state.search_query.is_empty() {
                                state.perform_search(true);
                            }
                            state.refresh_view(hwnd, true);
                        }
                        VK_BACK => {
                            state.search_query.pop();
                            state.perform_search(true);
                            state.refresh_view(hwnd, true);
                        }
                        VK_F3 => {
                            state.step_search_match(!shift_pressed, hwnd);
                        }
                        _ => {}
                    }
                    return 0;
                }

                match key {
                    VK_RIGHT | VK_NEXT => state.change_page(1, hwnd),
                    VK_LEFT | VK_PRIOR => state.change_page(-1, hwnd),
                    VK_DOWN => state.scroll_or_turn_page(true, hwnd),
                    VK_UP => state.scroll_or_turn_page(false, hwnd),
                    VK_OEM_PLUS => state.change_zoom(1.15, hwnd),
                    VK_OEM_MINUS => state.change_zoom(1.0 / 1.15, hwnd),
                    0x30 => state.reset_view(hwnd),
                    0x52 => state.rotate_page(!shift_pressed, hwnd),
                    0x43 => state.copy_text_to_clipboard(hwnd),
                    0x53 => state.save_current_page_image(hwnd),
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
                            state.status_msg = None;
                            if !ctrl_pressed {
                                state.ignore_next_char = true;
                            }
                            update_window_title(hwnd, state);
                        }
                    }
                    0x42 => {
                        state.show_bookmarks = !state.show_bookmarks;
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
                    VK_F3 => {
                        state.step_search_match(!shift_pressed, hwnd);
                    }
                    0x4E => {
                        if !state.search_query.is_empty() {
                            state.step_search_match(!shift_pressed, hwnd);
                        } else {
                            state.toggle_dark_mode(hwnd);
                        }
                    }
                    0x4F => {
                        if let Some(path) = open_file_dialog(hwnd) {
                            if let Err(e) = state.load_file(&path) {
                                eprintln!("[DaPDF] {}", e);
                            } else {
                                state.refresh_view(hwnd, true);
                            }
                        }
                    }
                    VK_ESCAPE => {
                        if !state.search_query.is_empty() {
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
                    0x00323232
                };
                let bg_brush = CreateSolidBrush(bg_color);
                FillRect(hdc, &rc, bg_brush);
                DeleteObject(bg_brush as _);

                let sb_w = state.sidebar_offset();

                if state.pixels.is_empty() && !state.doc.is_null() {
                    state.render_current_page(client_w, client_h);
                }

                if !state.pixels.is_empty() && state.bmp_width > 0 && state.bmp_height > 0 {
                    let canvas_w = (client_w - sb_w).max(1);
                    let x = sb_w + (canvas_w - state.bmp_width) / 2 + state.pan_x;
                    let y = (client_h - state.bmp_height) / 2 + state.pan_y;

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
                }

                if state.show_bookmarks {
                    draw_bookmark_sidebar(hdc as *mut c_void, client_h, state);
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
    // 고해상도(HiDPI) 모니터 배율 인식 활성화 -> 파일 열기 창 및 폰트 흐림 현상 완벽 해결
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

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            to_wstring(&format!("DaPDF(v{}) Viewer", APP_VERSION)).as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            80,
            80,
            1350,
            1050,
            null_mut(),
            null_mut(),
            hinstance,
            null(),
        );

        if hwnd.is_null() {
            FPDF_DestroyLibrary();
            std::process::exit(1);
        }

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
