use super::*;
use std::sync::Arc;

#[derive(Clone,PartialEq)]
pub(super) struct EditSnapshot {
    native: Arc<Vec<u8>>, overlays: Vec<UserOverlay>, watermark: Option<String>,
    tint: Option<(u8,u8,u8)>, header: Option<String>, numbers: bool, bates: bool,
}
#[derive(Default)]
pub(super) struct EditHistory {
    pub native_cache: Option<Arc<Vec<u8>>>, undo: Vec<EditSnapshot>, redo: Vec<EditSnapshot>, saved: Option<EditSnapshot>, redo_base: Option<EditSnapshot>,
}
impl AppState {
    fn edit_snapshot(&mut self) -> Result<EditSnapshot,String> {
        let native = match &self.history.native_cache {
            Some(bytes)=>bytes.clone(),
            None=> {
                if self.doc.is_null() { return Err("문서가 없습니다".into()); }
                let bytes=Arc::new(unsafe { annotations::serialize_document(self.doc,None)? });
                self.history.native_cache=Some(bytes.clone()); bytes
            }
        };
        Ok(EditSnapshot {native,overlays:self.overlays.clone(),watermark:self.watermark_text.clone(),tint:self.page_bg_tint,
            header:self.header_footer_text.clone(),numbers:self.show_page_numbers,bates:self.show_bates_numbers})
    }
    pub(super) fn mark_saved(&mut self)->Result<(),String> {
        self.history.saved=Some(self.edit_snapshot()?); Ok(())
    }
    pub(super) fn is_dirty(&mut self)->bool {
        if self.doc.is_null() { return false; }
        self.edit_snapshot().ok().as_ref()!=self.history.saved.as_ref()
    }
    pub(super) fn has_unsaved_changes(&self)->bool {
        if self.doc.is_null() { return false; }
        match (&self.history.native_cache,&self.history.saved) {
            (Some(native),Some(saved))=> native!=&saved.native || self.overlays!=saved.overlays || self.watermark_text!=saved.watermark ||
                self.page_bg_tint!=saved.tint || self.header_footer_text!=saved.header || self.show_page_numbers!=saved.numbers || self.show_bates_numbers!=saved.bates,
            _=>true,
        }
    }
    pub(super) fn record_edit(&mut self)->bool {
        if self.doc.is_null() { return true; }
        match self.edit_snapshot() {
            Ok(snapshot)=> {
                if self.history.undo.last()!=Some(&snapshot) { self.history.undo.push(snapshot); }
                // Conservative cap, even when several states share native bytes.
                while self.history.undo.len()>20 || (self.history.undo.len()>1 && self.history.undo.iter().map(|s|s.native.len()).sum::<usize>()>128*1024*1024) {
                    self.history.undo.remove(0);
                }
                true
            }
            Err(err)=>{self.status_msg=Some(format!("편집 이력 저장 실패: {}",err));false}
        }
    }
    pub(super) fn undo_redo(&mut self,redo:bool) {
        let Ok(current)=self.edit_snapshot() else { return };
        if self.history.redo_base.as_ref()!=Some(&current) { self.history.redo.clear(); }
        let stack=if redo {&mut self.history.redo} else {&mut self.history.undo};
        let target=loop { match stack.pop() { Some(snapshot) if snapshot!=current=>break Some(snapshot),Some(_)=>continue,None=>break None } };
        let Some(target)=target else { self.status_msg=Some("되돌릴 편집이 없습니다".into()); return };
        let bytes=(*target.native).clone();
        let loaded=unsafe { annotations::PdfDocument::from_bytes(&bytes) };
        let Ok(loaded)=loaded else { self.status_msg=Some("편집 이력 복원 실패".into());return };
        if redo { self.history.undo.push(current); } else { self.history.redo.push(current); }
        self.history.redo_base=Some(target.clone());
        unsafe { FPDF_CloseDocument(self.doc); }
        self.doc=loaded.0; std::mem::forget(loaded);
        self.pdf_bytes=bytes;
        self.history.native_cache=Some(target.native);
        self.overlays=target.overlays;self.watermark_text=target.watermark;self.page_bg_tint=target.tint;
        self.header_footer_text=target.header;self.show_page_numbers=target.numbers;self.show_bates_numbers=target.bates;
        self.page_count=unsafe {FPDF_GetPageCount(self.doc)};
        self.current_page=self.current_page.min(self.page_count-1).max(0);
        self.input_mode=InputMode::Normal;self.text_selection=None;self.selected_overlay_idx=None;
        self.search_matches.clear();self.clear_thumbnail_cache();self.pixels.clear();self.load_bookmarks();
        self.status_msg=Some(if redo {"다시 실행 완료"} else {"실행 취소 완료"}.into());
    }
    pub(super) unsafe fn confirm_unsaved(&mut self,hwnd:HWND)->bool {
        if !self.is_dirty() { return true; }
        let answer=MessageBoxW(hwnd,to_wstring("변경한 내용을 저장하시겠습니까?").as_ptr(),to_wstring("DaPDF - 저장 확인").as_ptr(),3|0x30);
        match answer {
            7=>true,
            6=> { handle_menu_command(hwnd,IDM_FILE_SAVE,self); !self.is_dirty() },
            _=>false,
        }
    }
}
