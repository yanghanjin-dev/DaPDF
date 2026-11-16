use super::*;
#[repr(C)]
struct DocInfo { size: i32, name: *const u16, output: *const u16, datatype: *const u16, flags: u32 }
#[link(name="gdi32")]
extern "system" {
    fn StartDocW(hdc: *mut c_void, info: *const DocInfo) -> i32;
    fn EndDoc(hdc: *mut c_void) -> i32;
    fn StartPage(hdc: *mut c_void) -> i32;
    fn EndPage(hdc: *mut c_void) -> i32;
    fn AbortDoc(hdc: *mut c_void) -> i32;
    fn GetDeviceCaps(hdc: *mut c_void, index: i32) -> i32;
}
pub(super) unsafe fn print_document(state: &AppState, hdc: *mut c_void, first: i32, last: i32, output: Option<&str>) -> Result<(),String> {
    if first < 0 || last < first || last >= state.page_count { return Err("인쇄 페이지 범위를 확인하세요".into()); }
    let bytes = annotations::serialize_document(state.doc,None)?;
    let copy = annotations::PdfDocument::from_bytes(&bytes)?;
    annotations::write_overlays(copy.0,&state.overlays)?;
    let name = to_wstring(&state.file_name);
    let target = output.map(to_wstring);
    let info = DocInfo { size: size_of::<DocInfo>() as i32, name: name.as_ptr(), output: target.as_ref().map_or(null(),|s|s.as_ptr()), datatype:null(),flags:0 };
    if StartDocW(hdc,&info) <= 0 { return Err("프린터 작업 시작 실패 또는 취소".into()); }
    let result = (|| {
        let (paper_w,paper_h) = (GetDeviceCaps(hdc,8),GetDeviceCaps(hdc,10));
        if paper_w <= 0 || paper_h <= 0 { return Err("인쇄 가능 영역을 읽을 수 없습니다".into()); }
        for index in first..=last {
            let page = FPDF_LoadPage(copy.0,index);
            if page.is_null() { return Err("인쇄 페이지 로드 실패".into()); }
            let (pw,ph) = (FPDF_GetPageWidth(page),FPDF_GetPageHeight(page));
            let scale = (paper_w as f64 / pw).min(paper_h as f64 / ph);
            let (dest_w,dest_h) = ((pw*scale).round() as i32,(ph*scale).round() as i32);
            // Bound memory independently of printer DPI (36 MB maximum bitmap).
            let raster_scale = (3000.0 / pw.max(ph)).min(scale);
            let (width,height) = ((pw*raster_scale).round().max(1.0) as i32,(ph*raster_scale).round().max(1.0) as i32);
            let bitmap = FPDFBitmap_Create(width,height,0);
            if bitmap.is_null() { FPDF_ClosePage(page); return Err("인쇄 이미지 메모리 할당 실패".into()); }
            FPDFBitmap_FillRect(bitmap,0,0,width,height,0xffffffff);
            FPDF_RenderPageBitmap(bitmap,page,0,0,width,height,0,FPDF_ANNOT | 0x800);
            let mut info: BITMAPINFO = zeroed();
            info.bmiHeader.biSize=size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth=width; info.bmiHeader.biHeight=-height;
            info.bmiHeader.biPlanes=1; info.bmiHeader.biBitCount=32; info.bmiHeader.biCompression=BI_RGB;
            let started=StartPage(hdc)>0;
            let copied=if started { StretchDIBits(hdc,(paper_w-dest_w)/2,(paper_h-dest_h)/2,dest_w,dest_h,0,0,width,height,FPDFBitmap_GetBuffer(bitmap) as _,&info,DIB_RGB_COLORS,SRCCOPY) } else { 0 };
            FPDFBitmap_Destroy(bitmap); FPDF_ClosePage(page);
            if !started || copied <= 0 || EndPage(hdc) <= 0 { return Err(format!("{}페이지 인쇄 실패",index+1)); }
        }
        Ok(())
    })();
    if result.is_err() { AbortDoc(hdc); return result; }
    if EndDoc(hdc)<=0 { AbortDoc(hdc); return Err("인쇄 작업 완료 실패".into()); }
    Ok(())
}
