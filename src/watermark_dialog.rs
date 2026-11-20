use super::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Controls::{EM_SETLIMITTEXT, EM_SETSEL};

const TEXT_ID: i32 = 100;
struct DialogState { initial: String, result: Option<String> }

fn word(data: &mut Vec<u8>, value: u16) { data.extend_from_slice(&value.to_le_bytes()); }
fn dword(data: &mut Vec<u8>, value: u32) { data.extend_from_slice(&value.to_le_bytes()); }
fn string(data: &mut Vec<u8>, text: &str) {
    for unit in text.encode_utf16().chain(Some(0)) { word(data, unit); }
}
fn control(data: &mut Vec<u8>, id: u16, class: u16, title: &str, rect: [u16; 4], style: u32) {
    while data.len() % 4 != 0 { data.push(0); }
    dword(data, WS_CHILD | WS_VISIBLE | style); dword(data, 0);
    for value in rect { word(data, value); }
    word(data, id); word(data, 0xffff); word(data, class);
    string(data, title); word(data, 0);
}

fn template() -> Vec<u32> {
    let mut data = Vec::new();
    dword(&mut data, WS_POPUP | WS_CAPTION | WS_SYSMENU | DS_MODALFRAME as u32 | DS_SETFONT as u32 | DS_CENTER as u32);
    dword(&mut data, 0); word(&mut data, 5);
    for value in [0, 0, 330, 130] { word(&mut data, value); }
    word(&mut data, 0); word(&mut data, 0);
    string(&mut data, "사용자 지정 워터마크");
    word(&mut data, 10); string(&mut data, "맑은 고딕");
    control(&mut data, 101, 0x82, "워터마크 문구", [14, 12, 302, 14], 0);
    control(&mut data, TEXT_ID as u16, 0x81, "", [14, 31, 302, 23], WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32);
    control(&mut data, 102, 0x82, "화면 표시용 워터마크입니다. 일반 PDF 저장에는 포함되지 않습니다.", [14, 62, 302, 26], 0);
    control(&mut data, IDOK as u16, 0x80, "적용", [170, 100, 68, 20], WS_TABSTOP | BS_DEFPUSHBUTTON as u32);
    control(&mut data, IDCANCEL as u16, 0x80, "취소", [248, 100, 68, 20], WS_TABSTOP);
    // DLGTEMPLATE and each DLGITEMTEMPLATE require DWORD alignment.
    while data.len() % 4 != 0 { data.push(0); }
    data.chunks_exact(4).map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap())).collect()
}

unsafe extern "system" fn dialog_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    match msg {
        WM_INITDIALOG => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam);
            let state = &*(lparam as *const DialogState);
            SetDlgItemTextW(hwnd, TEXT_ID, to_wstring(&state.initial).as_ptr());
            let edit = GetDlgItem(hwnd, TEXT_ID);
            SendMessageW(edit, EM_SETLIMITTEXT, 256, 0);
            windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(edit);
            SendMessageW(edit, EM_SETSEL, 0, -1);
            0 // Focus was explicitly assigned to the native edit control.
        }
        WM_COMMAND => {
            match (wparam & 0xffff) as i32 {
                IDOK => {
                    let mut text = vec![0u16; 257];
                    let count = GetDlgItemTextW(hwnd, TEXT_ID, text.as_mut_ptr(), text.len() as i32);
                    let value = String::from_utf16_lossy(&text[..count as usize]).trim().to_string();
                    if value.is_empty() {
                        MessageBoxW(hwnd, to_wstring("워터마크 문구를 입력하세요.").as_ptr(), to_wstring("워터마크").as_ptr(), MB_OK | MB_ICONINFORMATION);
                        return 1;
                    }
                    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DialogState;
                    if !ptr.is_null() { (*ptr).result = Some(value); }
                    EndDialog(hwnd, IDOK as isize);
                    1
                }
                IDCANCEL => { EndDialog(hwnd, IDCANCEL as isize); 1 }
                _ => 0,
            }
        }
        WM_CLOSE => { EndDialog(hwnd, IDCANCEL as isize); 1 }
        _ => 0,
    }
}

pub(super) fn show(owner: HWND, initial: &str) -> Result<Option<String>, String> {
    let template = template();
    let mut state = DialogState { initial: initial.into(), result: None };
    let result = unsafe { DialogBoxIndirectParamW(GetModuleHandleW(null()), template.as_ptr() as _, owner,
        Some(dialog_proc), &mut state as *mut _ as LPARAM) };
    if result == -1 { Err("워터마크 입력창을 열 수 없습니다.".into()) } else { Ok(state.result) }
}
