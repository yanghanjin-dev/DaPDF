use super::*;

use windows_sys::Win32::UI::Input::Ime::{ImmGetContext, ImmReleaseContext,
    ImmGetCompositionStringW, ImmSetCandidateWindow, CANDIDATEFORM};

unsafe fn composition_string(context: *mut c_void, index: u32) -> String {
    let bytes = ImmGetCompositionStringW(context, index, null_mut(), 0);
    if bytes <= 0 { return String::new(); }
    let mut buffer = vec![0u16; (bytes as usize + 1) / 2];
    let copied = ImmGetCompositionStringW(context, index, buffer.as_mut_ptr() as _, bytes as u32);
    if copied <= 0 { return String::new(); }
    String::from_utf16_lossy(&buffer[..(copied as usize / 2).min(buffer.len())])
}

impl AppState {
    pub(super) fn search_display_text(&self) -> String {
        format!("{}{}", self.search_query, self.search_composition)
    }

    pub(super) fn update_search_composition(&mut self, committed: Option<&str>, pending: Option<&str>) {
        if let Some(text) = committed {
            self.search_query.push_str(text);
            self.search_composition.clear();
            self.search_query_changed();
        }
        if let Some(text) = pending { self.search_composition = text.into(); }
        self.search_caret_visible = true;
    }

    pub(super) unsafe fn position_search_ime(&self, hwnd: HWND, context: *mut c_void) {
        let hdc = GetDC(hwnd);
        let old = SelectObject(hdc, GetStockObject(DEFAULT_GUI_FONT));
        let text = to_wstring(&self.search_query);
        let mut size: SIZE = zeroed();
        if text.len() > 1 { GetTextExtentPoint32W(hdc, text.as_ptr(), (text.len() - 1) as i32, &mut size); }
        SelectObject(hdc, old); ReleaseDC(hwnd, hdc);
        let form = CANDIDATEFORM { dwIndex: 0, dwStyle: 0x40,
            ptCurrentPos: POINT { x: (20 + size.cx).min(self.sidebar_width - 24), y: self.top_bar_offset() + SIDEBAR_TAB_HEIGHT + 52 },
            rcArea: zeroed() };
        ImmSetCandidateWindow(context, &form);
    }
}

// Return None for other input modes so their existing Windows IME behavior remains.
pub(super) unsafe fn handle_message(state: &mut AppState, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if msg == 0x010E {
        state.search_composition.clear();
        state.search_ime_active = false;
    }
    if state.input_mode != InputMode::Search { return None; }
    match msg {
        0x0281 => Some(DefWindowProcW(hwnd, msg, wparam, lparam & !(0x80000000u32 as isize))), // WM_IME_SETCONTEXT
        0x010D => { // WM_IME_STARTCOMPOSITION: render composition inline ourselves.
            state.search_composition.clear();
            state.search_ime_active = true;
            let context = ImmGetContext(hwnd);
            if !context.is_null() { state.position_search_ime(hwnd, context); ImmReleaseContext(hwnd, context); }
            InvalidateRect(hwnd, null(), 0);
            Some(0)
        }
        0x010F => { // WM_IME_COMPOSITION
            let context = ImmGetContext(hwnd);
            if !context.is_null() {
                let result = if lparam & 0x0800 != 0 { Some(composition_string(context, 0x0800)) } else { None };
                let pending = if lparam & 0x0008 != 0 { Some(composition_string(context, 0x0008)) } else { None };
                state.update_search_composition(result.as_deref(), pending.as_deref());
                if lparam == 0 { state.search_composition.clear(); }
                state.position_search_ime(hwnd, context);
                ImmReleaseContext(hwnd, context);
                state.refresh_view(hwnd, true);
            }
            // Do not call DefWindowProc: it would emit the committed text again as WM_CHAR.
            Some(0)
        }
        0x010E => { // WM_IME_ENDCOMPOSITION
            state.search_composition.clear();
            state.search_ime_active = false;
            InvalidateRect(hwnd, null(), 0);
            Some(0)
        }
        _ => None,
    }
}
