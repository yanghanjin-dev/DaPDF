use super::*;
use std::os::windows::process::CommandExt;

pub(super) fn ocr_image(path:&Path)->Result<String,String> {
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
    let work=env::temp_dir().join(format!("dapdf-ocr-{}-{}",std::process::id(),stamp));
    fs::create_dir(&work).map_err(|e|e.to_string())?;
    let script=work.join("recognize.ps1");let text=work.join("text.txt");
    let result=(|| {
        fs::write(&script,include_str!("ocr.ps1")).map_err(|e|e.to_string())?;
        let mut process=std::process::Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-ExecutionPolicy","Bypass","-File"])
            .arg(&script).arg("-ImagePath").arg(path).arg("-TextPath").arg(&text)
            .creation_flags(0x08000000).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped())
            .spawn().map_err(|e|format!("Windows OCR 시작 실패: {}",e))?;
        let started=std::time::Instant::now();
        loop {
            match process.try_wait().map_err(|e|e.to_string())? {
                Some(status)=> {
                    if !status.success() {return Err("Windows OCR 실패: Windows의 OCR 언어 팩 설치 여부를 확인하세요".into());}
                    break;
                }
                None if started.elapsed()>std::time::Duration::from_secs(45)=> {
                    let _=process.kill();let _=process.wait();return Err("OCR 처리 시간이 초과됐습니다".into());
                }
                None=>std::thread::sleep(std::time::Duration::from_millis(50)),
            }
        }
        fs::read_to_string(&text).map_err(|e|e.to_string())
    })();
    // Only remove the three known files in the unique directory created above.
    let _=fs::remove_file(script);let _=fs::remove_file(text);let _=fs::remove_dir(work);result
}

impl AppState {
    pub(super) fn ocr_page(&self,index:i32)->Result<String,String> {
        unsafe {
            let page=FPDF_LoadPage(self.doc,index);
            if page.is_null(){return Err("OCR 페이지 로드 실패".into());}
            let (pw,ph)=(FPDF_GetPageWidth(page),FPDF_GetPageHeight(page));
            let scale=(2400.0/pw.max(ph)).min(3.0);
            let (width,height)=((pw*scale).round().max(1.0) as i32,(ph*scale).round().max(1.0) as i32);
            let bitmap=FPDFBitmap_Create(width,height,0);
            if bitmap.is_null(){FPDF_ClosePage(page);return Err("OCR 이미지 메모리 할당 실패".into());}
            FPDFBitmap_FillRect(bitmap,0,0,width,height,0xffffffff);
            FPDF_RenderPageBitmap(bitmap,page,0,0,width,height,0,0);
            let stride=FPDFBitmap_GetStride(bitmap) as usize;
            let raw=std::slice::from_raw_parts(FPDFBitmap_GetBuffer(bitmap),stride*height as usize);
            let mut pixels=Vec::with_capacity(width as usize*height as usize*4);
            for row in raw.chunks_exact(stride){pixels.extend_from_slice(&row[..width as usize*4]);}
            FPDFBitmap_Destroy(bitmap);FPDF_ClosePage(page);
            let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
            let image=env::temp_dir().join(format!("dapdf-page-{}-{}.bmp",std::process::id(),stamp));
            write_bgra_to_bmp_file(&image.to_string_lossy(),width,height,&pixels).map_err(|e|e.to_string())?;
            let result=ocr_image(&image);let _=fs::remove_file(image);result
        }
    }
    pub(super) fn export_ocr(&mut self,hwnd:HWND) {
        let Some(path)=save_custom_dialog(hwnd,"ocr.txt","텍스트 (*.txt)\0*.txt\0\0","txt") else{return};
        self.status_msg=Some("현재 페이지 Windows OCR 처리 중...".into());update_window_title(hwnd,self);
        let result=self.ocr_page(self.current_page).and_then(|text|annotations::atomic_write(Path::new(&path),text.as_bytes()));
        self.status_msg=Some(match result{Ok(())=>format!("현재 페이지 OCR 텍스트 저장 완료: {}",path),Err(err)=>err});self.refresh_view(hwnd,false);
    }
    pub(super) fn export_docx(&mut self,hwnd:HWND) {
        let Some(path)=save_custom_dialog(hwnd,"document.docx","Word 문서 (*.docx)\0*.docx\0\0","docx") else{return};
        let result=(|| {
            let mut pages=Vec::new();
            for index in 0..self.page_count {
                self.status_msg=Some(format!("Word 변환 중 {}/{}페이지",index+1,self.page_count));update_window_title(hwnd,self);
                let text=self.extract_page_text_at(index).unwrap_or_default();
                pages.push(if text.trim().is_empty(){self.ocr_page(index)?}else{text});
            }
            annotations::atomic_write(Path::new(&path),&docx_bytes(&pages))
        })();
        self.status_msg=Some(match result{Ok(())=>format!("텍스트 중심 DOCX 저장 완료: {}",path),Err(err)=>err});self.refresh_view(hwnd,false);
    }
}

fn xml(text:&str)->String {
    text.chars().filter(|c|*c=='\t'||*c=='\n'||*c=='\r'||(*c as u32>=0x20 && !matches!(*c,'\u{fffe}'|'\u{ffff}'))).collect::<String>()
        .replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;")
}
pub(super) fn docx_bytes(pages:&[String])->Vec<u8> {
    let mut doc=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>");
    for (index,page) in pages.iter().enumerate() {
        if index>0{doc.push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");}
        for line in page.split('\n') {
            doc.push_str("<w:p><w:r><w:t xml:space=\"preserve\">");doc.push_str(&xml(line.trim_end_matches('\r')));doc.push_str("</w:t></w:r></w:p>");
        }
    }
    doc.push_str("<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1134\" w:right=\"1134\" w:bottom=\"1134\" w:left=\"1134\"/></w:sectPr></w:body></w:document>");
    let types="<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>";
    let rels="<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";
    zip_store(&[("[Content_Types].xml",types.as_bytes()),("_rels/.rels",rels.as_bytes()),("word/document.xml",doc.as_bytes())])
}
fn crc32(bytes:&[u8])->u32 {
    let mut crc=!0u32;
    for byte in bytes{crc^=*byte as u32;for _ in 0..8{crc=(crc>>1)^(0xedb88320u32.wrapping_mul(crc&1));}}
    !crc
}
fn zip_store(files:&[(&str,&[u8])])->Vec<u8> {
    let mut output=Vec::new();let mut central=Vec::new();
    fn u16le(out:&mut Vec<u8>,n:u16){out.extend_from_slice(&n.to_le_bytes());}
    fn u32le(out:&mut Vec<u8>,n:u32){out.extend_from_slice(&n.to_le_bytes());}
    for (name,data) in files {
        let offset=output.len() as u32;let crc=crc32(data);let size=data.len() as u32;
        u32le(&mut output,0x04034b50);for n in [20,0,0,0,33]{u16le(&mut output,n);}
        for n in [crc,size,size]{u32le(&mut output,n);}u16le(&mut output,name.len() as u16);u16le(&mut output,0);
        output.extend_from_slice(name.as_bytes());output.extend_from_slice(data);
        u32le(&mut central,0x02014b50);for n in [20,20,0,0,0,33]{u16le(&mut central,n);}
        for n in [crc,size,size]{u32le(&mut central,n);}u16le(&mut central,name.len() as u16);
        for _ in 0..4{u16le(&mut central,0);}u32le(&mut central,0);u32le(&mut central,offset);central.extend_from_slice(name.as_bytes());
    }
    let offset=output.len() as u32;let size=central.len() as u32;output.extend(central);
    u32le(&mut output,0x06054b50);u16le(&mut output,0);u16le(&mut output,0);
    u16le(&mut output,files.len() as u16);u16le(&mut output,files.len() as u16);u32le(&mut output,size);u32le(&mut output,offset);u16le(&mut output,0);output
}
