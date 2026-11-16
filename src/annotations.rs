use super::*;

#[link(name="kernel32")]
extern "system" {
    fn MoveFileExW(existing: *const u16, destination: *const u16, flags: u32) -> i32;
}
pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(),String> {
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
    let temporary=path.with_file_name(format!(".dapdf-save-{}-{}.tmp",std::process::id(),stamp));
    let result=(|| {
        let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|e|e.to_string())?;
        file.write_all(bytes).map_err(|e|e.to_string())?; file.sync_all().map_err(|e|e.to_string())?; drop(file);
        let source:Vec<u16>=temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let dest:Vec<u16>=path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(source.as_ptr(),dest.as_ptr(),1|8) }==0 { return Err(std::io::Error::last_os_error().to_string()); }
        Ok(())
    })();
    if result.is_err() { let _=fs::remove_file(temporary); }
    result
}
use std::os::windows::ffi::OsStrExt;

#[repr(C)]
struct PdfRectF { left: f32, top: f32, right: f32, bottom: f32 }
#[repr(C)]
struct PdfPointF { x: f32, y: f32 }
#[repr(C)]
struct PdfQuad { values: [f32; 8] }

#[link(name = "pdfium.dll")]
extern "C" {
    fn FPDFPage_CreateAnnot(page: *mut c_void, subtype: c_int) -> *mut c_void;
    fn FPDFPage_GetAnnotCount(page: *mut c_void) -> c_int;
    fn FPDFPage_GetAnnot(page: *mut c_void, index: c_int) -> *mut c_void;
    fn FPDFPage_CloseAnnot(annot: *mut c_void);
    fn FPDFPage_RemoveAnnot(page: *mut c_void, index: c_int) -> c_int;
    fn FPDFAnnot_SetRect(annot: *mut c_void, rect: *const PdfRectF) -> c_int;
    fn FPDFAnnot_SetColor(annot: *mut c_void, kind: c_int, r: u32, g: u32, b: u32, a: u32) -> c_int;
    fn FPDFAnnot_SetBorder(annot: *mut c_void, horizontal: f32, vertical: f32, width: f32) -> c_int;
    fn FPDFAnnot_AppendAttachmentPoints(annot: *mut c_void, quad: *const PdfQuad) -> c_int;
    fn FPDFAnnot_AddInkStroke(annot: *mut c_void, points: *const PdfPointF, count: usize) -> c_int;
    fn FPDFAnnot_SetStringValue(annot: *mut c_void, key: *const c_char, value: *const u16) -> c_int;
    fn FPDFAnnot_GetStringValue(annot: *mut c_void, key: *const c_char, buffer: *mut u16, bytes: c_ulong) -> c_ulong;
    fn FPDFAnnot_SetFlags(annot: *mut c_void, flags: c_int) -> c_int;
}

pub(super) struct PdfDocument(pub *mut c_void);
impl PdfDocument {
    pub unsafe fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let size = c_int::try_from(bytes.len()).map_err(|_| "PDF가 너무 큽니다")?;
        let doc = FPDF_LoadMemDocument(bytes.as_ptr() as *const c_void, size, null());
        if doc.is_null() { Err("PDF 사본 생성 실패".into()) } else { Ok(Self(doc)) }
    }
}
impl Drop for PdfDocument {
    fn drop(&mut self) { unsafe { FPDF_CloseDocument(self.0); } }
}
pub(super) unsafe fn serialize_document(doc: *mut c_void, version: Option<i32>) -> Result<Vec<u8>, String> {
    let mut context = FileWriteContext { fw: FPDF_FILEWRITE { version: 1, WriteBlock: Some(write_block_cb) }, buf: Vec::new() };
    let result = match version {
        Some(version) => FPDF_SaveWithVersion(doc, &mut context.fw, 0, version),
        None => FPDF_SaveAsCopy(doc, &mut context.fw, 0),
    };
    if result == 0 || context.buf.is_empty() { Err("PDF 직렬화 실패".into()) } else { Ok(context.buf) }
}

fn subtype(kind: OverlayKind) -> Option<i32> {
    match kind {
        OverlayKind::Highlight | OverlayKind::AreaHighlight => Some(9),
        OverlayKind::Underline => Some(10),
        OverlayKind::Strikeout => Some(12),
        OverlayKind::Pencil => Some(15),
        _ => None,
    }
}

// A versioned private string preserves exact editing coordinates without an
// external sidecar. Other PDF readers still see ordinary PDF annotations.
fn encode(overlay: &UserOverlay) -> String {
    let kind = match overlay.kind {
        OverlayKind::Highlight => 0, OverlayKind::AreaHighlight => 1,
        OverlayKind::Underline => 2, OverlayKind::Strikeout => 3, _ => 4,
    };
    let mut values = vec!["1".into(), kind.to_string(), overlay.x1.to_string(), overlay.y1.to_string(), overlay.x2.to_string(), overlay.y2.to_string()];
    for (x, y) in &overlay.points { values.push(x.to_string()); values.push(y.to_string()); }
    values.join(";")
}
fn decode(value: &str, page: i32) -> Option<UserOverlay> {
    let fields: Vec<_> = value.split(';').collect();
    if fields.len() < 6 || fields.len() > 200006 || fields[0] != "1" || fields.len() % 2 != 0 { return None; }
    let kind = match fields[1] { "0" => OverlayKind::Highlight, "1" => OverlayKind::AreaHighlight,
        "2" => OverlayKind::Underline, "3" => OverlayKind::Strikeout, "4" => OverlayKind::Pencil, _ => return None };
    let numbers: Vec<f64> = fields[2..].iter().map(|v| v.parse().ok()).collect::<Option<_>>()?;
    if numbers.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)) { return None; }
    let points = numbers[4..].chunks_exact(2).map(|xy| (xy[0], xy[1])).collect();
    Some(UserOverlay { page_index: page, kind, x1: numbers[0], y1: numbers[1], x2: numbers[2], y2: numbers[3], points, label: String::new(), checked: false })
}

pub(super) unsafe fn write_overlays(doc: *mut c_void, overlays: &[UserOverlay]) -> Result<(), String> {
    if overlays.iter().any(|overlay| subtype(overlay.kind).is_none()) {
        return Err("아직 PDF 저장을 지원하지 않는 양식/서명/마스킹 등이 있습니다. 현재 강조·밑줄·취소선·연필 주석만 저장할 수 있습니다.".into());
    }
    for overlay in overlays {
        let Some(kind) = subtype(overlay.kind) else { continue };
        let page = FPDF_LoadPage(doc, overlay.page_index);
        if page.is_null() { return Err("주석 페이지를 열 수 없습니다".into()); }
        let result = write_annotation(page, overlay, kind);
        FPDF_ClosePage(page);
        result?;
    }
    Ok(())
}
unsafe fn write_annotation(page: *mut c_void, overlay: &UserOverlay, kind: i32) -> Result<(), String> {
    let annot = FPDFPage_CreateAnnot(page, kind);
    if annot.is_null() { return Err("PDF 주석 생성 실패".into()); }
    let result = (|| {
        let (width, height) = (FPDF_GetPageWidth(page) as f32, FPDF_GetPageHeight(page) as f32);
        let mut xs = vec![overlay.x1, overlay.x2];
        let mut ys = vec![overlay.y1, overlay.y2];
        for (x, y) in &overlay.points { xs.push(*x); ys.push(*y); }
        let left = xs.iter().copied().fold(f64::INFINITY, f64::min) as f32 * width;
        let right = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max) as f32 * width;
        let top = (1.0 - ys.iter().copied().fold(f64::INFINITY, f64::min)) as f32 * height;
        let bottom = (1.0 - ys.iter().copied().fold(f64::NEG_INFINITY, f64::max)) as f32 * height;
        let rect = PdfRectF { left: left - 1.0, top: top + 1.0, right: right + 1.0, bottom: bottom - 1.0 };
        if FPDFAnnot_SetRect(annot, &rect) == 0 { return Err("주석 영역 설정 실패".into()); }
        let (r, g, b, a) = if kind == 9 { (255, 220, 0, 120) } else { (35, 80, 210, 255) };
        if FPDFAnnot_SetColor(annot, 0, r, g, b, a) == 0 { return Err("주석 색상 설정 실패".into()); }
        if kind == 15 {
            let points: Vec<_> = overlay.points.iter().map(|(x,y)| PdfPointF { x: *x as f32 * width, y: (1.0 - *y) as f32 * height }).collect();
            if points.len() < 2 || FPDFAnnot_AddInkStroke(annot, points.as_ptr(), points.len()) < 0 { return Err("연필 주석 저장 실패".into()); }
            if FPDFAnnot_SetBorder(annot, 0.0, 0.0, 1.5) == 0 { return Err("연필 두께 저장 실패".into()); }
        } else {
            let quad = PdfQuad { values: [left, top, right, top, left, bottom, right, bottom] };
            if FPDFAnnot_AppendAttachmentPoints(annot, &quad) == 0 { return Err("텍스트 주석 위치 저장 실패".into()); }
        }
        if FPDFAnnot_SetStringValue(annot, b"DaPDFOverlayV1\0".as_ptr() as _, to_wstring(&encode(overlay)).as_ptr()) == 0 {
            return Err("주석 편집 정보 저장 실패".into());
        }
        FPDFAnnot_SetStringValue(annot, b"T\0".as_ptr() as _, to_wstring("DaPDF").as_ptr());
        FPDFAnnot_SetFlags(annot, 4); // printable
        Ok(())
    })();
    FPDFPage_CloseAnnot(annot);
    result
}

pub(super) unsafe fn take_managed_overlays(doc: *mut c_void) -> Vec<UserOverlay> {
    let mut overlays = Vec::new();
    for index in 0..FPDF_GetPageCount(doc) {
        let page = FPDF_LoadPage(doc, index);
        if page.is_null() { continue; }
        for i in (0..FPDFPage_GetAnnotCount(page)).rev() {
            let annot = FPDFPage_GetAnnot(page, i);
            if annot.is_null() { continue; }
            let key = b"DaPDFOverlayV1\0".as_ptr() as *const c_char;
            let size = FPDFAnnot_GetStringValue(annot, key, null_mut(), 0);
            let value = if size > 2 && size <= 8_000_000 {
                let mut text = vec![0u16; (size as usize + 1) / 2];
                let written = FPDFAnnot_GetStringValue(annot, key, text.as_mut_ptr(), size);
                if written == size { decode(&String::from_utf16_lossy(&text[..text.len().saturating_sub(1)]), index) } else { None }
            } else { None };
            FPDFPage_CloseAnnot(annot);
            if let Some(value) = value {
                if FPDFPage_RemoveAnnot(page, i) != 0 { overlays.push(value); }
            }
        }
        FPDF_ClosePage(page);
    }
    overlays.reverse();
    overlays
}

#[cfg(test)]
pub(super) unsafe fn annotation_count(doc: *mut c_void, index: i32) -> i32 {
    let page = FPDF_LoadPage(doc, index);
    let count = FPDFPage_GetAnnotCount(page);
    FPDF_ClosePage(page);
    count
}
