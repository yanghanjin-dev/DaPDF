use super::*;

#[test]
fn page_ranges_reject_invalid_and_duplicate_pages() {
    assert_eq!(pages::parse_range("1-3,5",5).unwrap(),vec![0,1,2,4]);
    assert_eq!(pages::parse_range("3,1,2",3).unwrap(),vec![2,0,1]);
    for input in ["", "0", "1-9", "3-1", "1,1", "1--2", "x"] {assert!(pages::parse_range(input,5).is_err());}
}

#[test]
fn toolbar_labels_fit_and_wrapped_buttons_remain_clickable() {
    unsafe {
        let hdc = CreateCompatibleDC(null_mut());
        assert!(!hdc.is_null());
        let old_font = SelectObject(hdc, GetStockObject(DEFAULT_GUI_FONT));
        for dpi in [96, 144, 192, 288] {
            for width in [480, 1000, 2100] {
                let (rects, height) = toolbar_layout(hdc, width, dpi);
                assert_eq!(rects.len(), TOOLBAR_BUTTONS.len());
                let mut state = AppState::new();
                state.toolbar_rects = rects;
                state.toolbar_height = height;
                for (i, (rect, button)) in state.toolbar_rects.iter().zip(TOOLBAR_BUTTONS).enumerate() {
                    let text: Vec<u16> = button.label.encode_utf16().collect();
                    let mut size: SIZE = zeroed();
                    assert_ne!(GetTextExtentPoint32W(hdc, text.as_ptr(), text.len() as i32, &mut size), 0);
                    assert!(rect.right - rect.left >= size.cx + 20);
                    assert!(rect.bottom - rect.top >= size.cy + 2);
                    assert!(rect.bottom - rect.top <= size.cy + 6);
                    assert!(rect.right <= width && rect.bottom < height);
                    assert_eq!(hit_test_toolbar(&state, (rect.left + rect.right) / 2,
                        (rect.top + rect.bottom) / 2), Some(button.id));
                    for other in &state.toolbar_rects[..i] {
                        assert!(rect.left >= other.right || rect.right <= other.left ||
                            rect.top >= other.bottom || rect.bottom <= other.top);
                    }
                }
                let (controls,info,status_height)=status_layout(hdc,width,dpi,&state);
                assert!(info.right>info.left && info.bottom>info.top);
                state.status_controls=controls;state.statusbar_height=status_height;
                for (index,control) in state.status_controls.iter().enumerate() {
                    let text:Vec<u16>=control.label.encode_utf16().collect();let mut size:SIZE=zeroed();
                    GetTextExtentPoint32W(hdc,text.as_ptr(),text.len() as i32,&mut size);
                    let rect=&control.rect;
                    assert!(rect.right-rect.left>=size.cx+10 && rect.bottom-rect.top>=size.cy+6);
                    assert!(rect.left>=0 && rect.right<=width && rect.bottom<=status_height);
                    assert_eq!(hit_test_status_bar(&state,(rect.left+rect.right)/2,(rect.top+rect.bottom)/2),Some(control.id));
                    assert!(rect.left>=info.right || rect.right<=info.left || rect.top>=info.bottom || rect.bottom<=info.top);
                    for other in &state.status_controls[..index] {
                        let other=&other.rect;
                        assert!(rect.left>=other.right || rect.right<=other.left || rect.top>=other.bottom || rect.bottom<=other.top);
                    }
                }
            }
        }
        SelectObject(hdc, old_font);
        DeleteDC(hdc);
    }
}

// One test owns PDFium's process-global lifetime; no parallel initialization.
#[test]
fn pdf_open_render_search_edit_save_roundtrip() {
    struct Library;
    impl Drop for Library {
        fn drop(&mut self) { unsafe { FPDF_DestroyLibrary(); } }
    }
    unsafe { FPDF_InitLibrary(); }
    let _library = Library;
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/regression-output");
    fs::create_dir_all(&output).unwrap();
    let mut state = AppState::new();
    assert!(state.show_sidebar);
    assert!(state.sidebar_tab == SidebarTab::Thumbnails);
    state.show_sidebar = false;
    state.sidebar_tab = SidebarTab::Search;
    assert!(state.save_pdf_to_path(output.join("empty.pdf").to_str().unwrap(), None).is_err());
    state.load_file(concat!(env!("CARGO_MANIFEST_DIR"), "/test-한글.pdf")).unwrap();
    assert_eq!(state.page_count, 2);
    assert!(state.show_sidebar);
    assert!(state.sidebar_tab == SidebarTab::Thumbnails);
    assert!(state.extract_page_text_at(0).unwrap().contains("DaPDF Rendering Test"));
    state.search_query = "Rendering".into();
    state.perform_search(true);
    assert_eq!(state.search_matches.len(), 1);
    assert_eq!(state.build_text_selection(0, 0, 4).unwrap().text, "DaPDF");
    for (spread, rotation) in [(false, 0), (true, 0), (false, 1)] {
        state.spread_mode = spread;
        state.rotation = rotation;
        state.pixels.clear();
        state.render_current_page(1000, 800);
        assert!(state.bmp_width > 0 && state.bmp_height > 0);
        assert!(!state.pixels.is_empty());
        assert!(state.pixels.chunks_exact(4).any(|pixel| pixel[0] < 128));
    }
    assert!(state.get_or_render_thumbnail(0).is_some());
    assert!(state.thumb_cache.contains_key(&0));
    unsafe {
        let page = FPDF_LoadPage(state.doc, 0);
        assert!(!page.is_null());
        AppState::insert_pdf_text_object(state.doc, page, "Roundtrip marker", 72.0, 620.0, 14.0);
        assert_ne!(FPDFPage_GenerateContent(page), 0);
        FPDF_ClosePage(page);
        let blank = FPDFPage_New(state.doc, 2, 595.0, 842.0);
        assert!(!blank.is_null());
        assert_ne!(FPDFPage_GenerateContent(blank), 0);
        FPDF_ClosePage(blank);
    }
    state.history.native_cache=None;
    for (version, header) in [(14, b"%PDF-1.4"), (17, b"%PDF-1.7")] {
        let path = output.join(format!("저장-{}.pdf", version));
        state.save_pdf_to_path(path.to_str().unwrap(), Some(version)).unwrap();
        assert!(fs::read(&path).unwrap().starts_with(header));
        let mut reopened = AppState::new();
        reopened.load_file(path.to_str().unwrap()).unwrap();
        assert_eq!(reopened.page_count, 3);
        assert!(reopened.extract_page_text_at(0).unwrap().contains("Roundtrip marker"));
        let previous_path = reopened.current_path.clone();
        assert!(reopened.save_pdf_to_path(output.to_str().unwrap(), None).is_err());
        assert_eq!(reopened.current_path, previous_path);
        assert!(reopened.load_file(output.to_str().unwrap()).is_err());
        assert_eq!(reopened.page_count, 3);
    }
    state.overlays = [OverlayKind::Highlight, OverlayKind::AreaHighlight, OverlayKind::Underline,
        OverlayKind::Strikeout, OverlayKind::Pencil].iter().enumerate().map(|(index, kind)| {
        let y = 0.25 + index as f64 * 0.08;
        UserOverlay { page_index: 0, kind: *kind, x1: 0.2, y1: y, x2: 0.5, y2: y + 0.04,
            points: if *kind == OverlayKind::Pencil { vec![(0.2,y),(0.35,y+0.02),(0.5,y+0.04)] } else { vec![] },
            label: String::new(), checked: false }
    }).collect();
    let annotation_path = output.join("annotations.pdf");
    for _ in 0..2 {
        state.save_pdf_to_path(annotation_path.to_str().unwrap(), None).unwrap();
        let bytes = fs::read(&annotation_path).unwrap();
        unsafe {
            let native = annotations::PdfDocument::from_bytes(&bytes).unwrap();
            assert_eq!(annotations::annotation_count(native.0, 0), 5);
            let page = FPDF_LoadPage(native.0, 0);
            let bitmap = FPDFBitmap_Create(612,792,1);
            FPDFBitmap_FillRect(bitmap,0,0,612,792,0xffffffff);
            FPDF_RenderPageBitmap(bitmap,page,0,0,612,792,0,FPDF_ANNOT);
            let pixels = std::slice::from_raw_parts(FPDFBitmap_GetBuffer(bitmap), FPDFBitmap_GetStride(bitmap) as usize * 792);
            assert!(pixels.chunks_exact(4).filter(|p| p[0] < 180 && p[1] > 100 && p[2] > 150).count() > 100, "native PDF highlight is visible");
            FPDFBitmap_Destroy(bitmap);
            FPDF_ClosePage(page);
        }
    }
    let mut reopened = AppState::new();
    reopened.load_file(annotation_path.to_str().unwrap()).unwrap();
    assert!(reopened.overlays == state.overlays);
    assert_eq!(unsafe { annotations::annotation_count(reopened.doc,0) }, 0);
    reopened.overlays.remove(0);
    reopened.save_pdf_to_path(annotation_path.to_str().unwrap(),None).unwrap();
    let bytes = fs::read(&annotation_path).unwrap();
    let native = unsafe { annotations::PdfDocument::from_bytes(&bytes).unwrap() };
    assert_eq!(unsafe { annotations::annotation_count(native.0,0) },4);
    assert!(!reopened.is_dirty());
    assert!(reopened.record_edit());
    reopened.overlays.pop();
    assert!(reopened.is_dirty());
    reopened.undo_redo(false);
    assert_eq!(reopened.overlays.len(),4);
    assert!(!reopened.is_dirty());
    reopened.undo_redo(true);
    assert_eq!(reopened.overlays.len(),3);
    assert!(reopened.is_dirty());
    assert!(reopened.record_edit());
    unsafe {
        let page=FPDFPage_New(reopened.doc,reopened.page_count,595.0,842.0);
        FPDFPage_GenerateContent(page);FPDF_ClosePage(page);
    }
    reopened.page_count+=1;
    reopened.history.native_cache=None;
    reopened.undo_redo(false);
    assert_eq!(reopened.page_count,3);
    reopened.undo_redo(true);
    assert_eq!(reopened.page_count,4);
    let first_page=unsafe {FPDF_LoadPage(reopened.doc,0)};
    let first_width=unsafe {FPDF_GetPageWidth(first_page)};
    unsafe {FPDF_ClosePage(first_page);}
    reopened.reorder_pages(&[3,2,1,0]).unwrap();
    assert!(reopened.overlays.iter().all(|o|o.page_index==3));
    let last_page=unsafe {FPDF_LoadPage(reopened.doc,3)};
    assert_eq!(unsafe {FPDF_GetPageWidth(last_page)},first_width);
    unsafe {FPDF_ClosePage(last_page);}
    reopened.undo_redo(false);
    assert!(reopened.overlays.iter().all(|o|o.page_index==0));
    let bytes=unsafe {annotations::serialize_document(reopened.doc,None).unwrap()};
    let source=unsafe {annotations::PdfDocument::from_bytes(&bytes).unwrap()};
    unsafe {annotations::write_overlays(source.0,&reopened.overlays).unwrap();}
    let selection=unsafe {pages::import_selection(source.0,&[0,2]).unwrap()};
    assert_eq!(unsafe {FPDF_GetPageCount(selection.0)},2);
    assert_eq!(unsafe {annotations::annotation_count(selection.0,0)},3);
    for index in 0..4 {
        let split=unsafe {pages::import_selection(source.0,&[index]).unwrap()};
        assert_eq!(unsafe {FPDF_GetPageCount(split.0)},1);
        annotations::atomic_write(&output.join(format!("split-{}.pdf",index)),&unsafe {annotations::serialize_document(split.0,None).unwrap()}).unwrap();
    }
    fs::write(output.join("conversion.docx"),conversion::docx_bytes(&["한글 & <텍스트>\nSecond line".into(),"Next page".into()])).unwrap();
    if env::var_os("DAPDF_TEST_OCR").is_some() {
        let recognized=reopened.ocr_page(0).unwrap();
        println!("OCR result: {}",recognized);
        assert!(recognized.to_lowercase().contains("rendering"));
    }
    if env::var_os("DAPDF_TEST_PRINT").is_some() {
        #[link(name="gdi32")]
        extern "system" { fn CreateDCW(driver:*const u16,device:*const u16,output:*const u16,mode:*const c_void)->*mut c_void; }
        let printed=output.join("printed.pdf");
        unsafe {
            let hdc=CreateDCW(to_wstring("WINSPOOL").as_ptr(),to_wstring("Microsoft Print to PDF").as_ptr(),null(),null());
            assert!(!hdc.is_null());
            let result=printing::print_document(&reopened,hdc,0,0,Some(printed.to_str().unwrap()));
            DeleteDC(hdc);result.unwrap();
        }
        for _ in 0..100 { if printed.exists() { break; } std::thread::sleep(std::time::Duration::from_millis(100)); }
        let bytes=fs::read(printed).unwrap();
        let printed=unsafe {annotations::PdfDocument::from_bytes(&bytes).unwrap()};
        assert_eq!(unsafe {FPDF_GetPageCount(printed.0)},1);
    }
}
