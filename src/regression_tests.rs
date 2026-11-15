use super::*;

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
    assert!(state.save_pdf_to_path(output.join("empty.pdf").to_str().unwrap(), None).is_err());
    state.load_file(concat!(env!("CARGO_MANIFEST_DIR"), "/test-한글.pdf")).unwrap();
    assert_eq!(state.page_count, 2);
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
}
