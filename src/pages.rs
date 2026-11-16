use super::*;
pub(super) fn parse_range(input:&str,count:i32)->Result<Vec<i32>,String> {
    if input.len()>100_000 { return Err("페이지 입력이 너무 깁니다".into()); }
    let mut pages=Vec::new(); let mut seen=std::collections::HashSet::new();
    for item in input.split(',') {
        let part:Vec<_>=item.trim().split('-').collect();
        let first=part[0].trim().parse::<i32>().map_err(|_|"페이지 번호를 확인하세요")?;
        let last=if part.len()==2 {part[1].trim().parse::<i32>().map_err(|_|"페이지 범위를 확인하세요")?} else if part.len()==1 {first} else {return Err("잘못된 페이지 범위".into())};
        if first<1 || last<first || last>count { return Err(format!("1~{} 사이의 올바른 페이지 범위를 입력하세요",count)); }
        for page in first..=last { if !seen.insert(page) {return Err("중복 페이지가 있습니다".into());} pages.push(page-1); }
    }
    if pages.is_empty() {return Err("페이지를 선택하세요".into());} Ok(pages)
}
pub(super) unsafe fn import_selection(source:*mut c_void,pages:&[i32])->Result<annotations::PdfDocument,String> {
    let doc=FPDF_CreateNewDocument();
    if doc.is_null() { return Err("새 PDF 생성 실패".into()); }
    let doc=annotations::PdfDocument(doc);
    let range=CString::new(pages.iter().map(|p|(p+1).to_string()).collect::<Vec<_>>().join(",")).unwrap();
    if FPDF_ImportPages(doc.0,source,range.as_ptr(),0)==0 {return Err("페이지 가져오기 실패".into());} Ok(doc)
}
impl AppState {
    pub(super) fn reorder_pages(&mut self,pages:&[i32])->Result<(),String> {
        let mut sorted=pages.to_vec();sorted.sort_unstable();
        if sorted!=(0..self.page_count).collect::<Vec<_>>() {return Err("전체 페이지를 중복·누락 없이 입력하세요".into());}
        let doc=unsafe {import_selection(self.doc,pages)?};
        if !self.record_edit() {return Err("편집 이력 저장 실패".into());}
        let mut inverse=vec![0;pages.len()];
        for (new,old) in pages.iter().enumerate(){inverse[*old as usize]=new as i32;}
        for overlay in &mut self.overlays {overlay.page_index=inverse[overlay.page_index as usize];}
        self.current_page=inverse[self.current_page as usize];
        unsafe {FPDF_CloseDocument(self.doc);}
        self.doc=doc.0;std::mem::forget(doc);self.pdf_bytes.clear();self.history.native_cache=None;
        self.clear_thumbnail_cache();self.text_selection=None;self.search_matches.clear();self.load_bookmarks();self.pixels.clear();Ok(())
    }
    pub(super) unsafe fn apply_page_operation(&mut self,hwnd:HWND,action:u16)->Result<(),String> {
        let pages=parse_range(&self.goto_input,self.page_count)?;
        if action==IDM_ORG_REORDER {self.reorder_pages(&pages)?;self.refresh_view(hwnd,true);return Ok(());}
        let Some(path)=save_custom_dialog(hwnd,"pages.pdf","PDF 문서 (*.pdf)\0*.pdf\0\0","pdf") else {return Err("페이지 작업을 취소했습니다".into())};
        let bytes=annotations::serialize_document(self.doc,None)?;
        let source=annotations::PdfDocument::from_bytes(&bytes)?;
        annotations::write_overlays(source.0,&self.overlays)?;
        if action==IDM_ORG_SPLIT_PDF {
            let base=Path::new(&path);let stem=base.file_stem().and_then(|s|s.to_str()).unwrap_or("page");
            let targets:Vec<_>=pages.iter().map(|page|base.with_file_name(format!("{}-{:04}.pdf",stem,page+1))).collect();
            if targets.iter().any(|p|p.exists()) {return Err("동일한 분할 파일이 이미 있습니다. 다른 이름을 선택하세요".into());}
            for (page,target) in pages.iter().zip(targets) {
                let doc=import_selection(source.0,&[*page])?;
                annotations::atomic_write(&target,&annotations::serialize_document(doc.0,None)?)?;
            }
        } else {
            if self.current_path.as_deref().map(|p|Path::new(p)==Path::new(&path)).unwrap_or(false) {return Err("추출 파일은 원본과 다른 이름을 선택하세요".into());}
            let doc=import_selection(source.0,&pages)?;
            annotations::atomic_write(Path::new(&path),&annotations::serialize_document(doc.0,None)?)?;
        }
        Ok(())
    }
}
