#ifndef UNICODE
#define UNICODE
#endif
#define _UNICODE
#include <windows.h>
#include <windowsx.h>
#include <commdlg.h>
#include <shellapi.h>
#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include "vendor/include/fpdfview.h"

enum { OPEN=100, PREV, NEXT, MINUS, PLUS, FIT, ROTATE };
enum { TOP=60, MARGIN=20, GAP=12 };      // toolbar height, outer margin, gap between pages
#define MAX_PIXELS 25000000.0             // per-page render buffer limit (about 100 MB)

// One entry per page. Sizes are in PDF points; pw/ph/y are the laid-out pixel box.
typedef struct { double w, h; int y, pw, ph; FPDF_BITMAP bmp; int bw, bh, rot; BOOL failed; } Slot;

static HWND window, label, buttons[7];
static FPDF_DOCUMENT doc;
static Slot *slots;
static void *data;
static int index_, count, rotation, sx, sy, doc_w, doc_h;
static double zoom=1;
static BOOL fit=TRUE;
static wchar_t filename[32768];
static HFONT font;

static void error(const wchar_t *s) { MessageBoxW(window,s,L"DaPDF",MB_ICONERROR); }
static void view_size(int *vw,int *vh) { RECT r; GetClientRect(window,&r); *vw=max(1,r.right); *vh=max(1,r.bottom-TOP); }
static void free_slot(Slot *s) { if(s->bmp) FPDFBitmap_Destroy(s->bmp); s->bmp=NULL; s->bw=s->bh=0; }
static void close_doc(void) {
    for(int i=0;i<count;i++) free_slot(&slots[i]);
    free(slots); slots=NULL;
    if(doc) FPDF_CloseDocument(doc);
    doc=NULL; free(data); data=NULL; count=0; doc_w=doc_h=0;
}
// Last page whose top is at or above document coordinate y.
static int page_at(int y) {
    int lo=0, hi=count-1;
    while(lo<hi) { int mid=(lo+hi+1)/2; if(slots[mid].y<=y) lo=mid; else hi=mid-1; }
    return lo;
}
static void layout(void) {
    if(!doc) { doc_w=doc_h=0; return; }
    int vw, vh; view_size(&vw,&vh);
    double maxw=1;
    for(int i=0;i<count;i++) { double w=rotation%2?slots[i].h:slots[i].w; if(w>maxw) maxw=w; }
    if(fit) zoom=max(.05,min(6.0,(vw-2*MARGIN-8)/maxw));
    int y=MARGIN; doc_w=0;
    for(int i=0;i<count;i++) {
        Slot *s=&slots[i];
        double w=rotation%2?s->h:s->w, h=rotation%2?s->w:s->h;
        s->pw=max(1,(int)(w*zoom+.5)); s->ph=max(1,(int)(h*zoom+.5)); s->y=y;
        y+=s->ph+GAP; doc_w=max(doc_w,s->pw);
    }
    doc_h=y-GAP+MARGIN; doc_w+=2*MARGIN;
}
static void clamp_scroll(void) {
    int vw, vh; view_size(&vw,&vh);
    sx=max(0,min(sx,doc_w-vw)); sy=max(0,min(sy,doc_h-vh));
    // The vertical bar stays visible so width-fit does not oscillate as it appears/disappears.
    SCROLLINFO si={sizeof(si),SIF_RANGE|SIF_PAGE|SIF_POS|(doc?SIF_DISABLENOSCROLL:0),0,max(0,doc_h-1),(UINT)vh,sy,0};
    SetScrollInfo(window,SB_VERT,&si,TRUE);
    si.fMask=SIF_RANGE|SIF_PAGE|SIF_POS; si.nMax=max(0,doc_w-1); si.nPage=vw; si.nPos=sx;
    SetScrollInfo(window,SB_HORZ,&si,TRUE);
}
static void refresh(void) {
    clamp_scroll();
    wchar_t text[128];
    if(doc) swprintf(text,128,L"%d / %d 페이지   ·   %.0f%%",index_+1,count,zoom*100);
    else wcscpy(text,L"PDF 파일을 열어 시작하세요");
    SetWindowTextW(label,text);
    for(int i=1;i<7;i++) EnableWindow(buttons[i],doc!=NULL);
    EnableWindow(buttons[1],doc && index_>0); EnableWindow(buttons[2],doc && index_+1<count);
    RECT r; GetClientRect(window,&r); r.top=TOP; InvalidateRect(window,&r,FALSE);
}
// Re-layout after a zoom or size change, keeping the same spot of the same page at the top.
static void relayout(void) {
    if(!doc) { refresh(); return; }
    int vw, vh; view_size(&vw,&vh);
    int a=page_at(sy+MARGIN);
    double frac=(sy-slots[a].y)/(double)slots[a].ph, cx=doc_w>vw?(sx+vw/2.0)/doc_w:.5;
    layout();
    sy=(int)(slots[a].y+frac*slots[a].ph); sx=(int)(cx*doc_w-vw/2.0);
    refresh();
}
// The "current" page follows the view while the user scrolls.
static void update_index(void) {
    if(!doc) return;
    int vw, vh; view_size(&vw,&vh);
    index_=(doc_h>vh && sy>=doc_h-vh)?count-1:page_at(sy+vh/3);
}
static void scroll_to(int x,int y) { sx=x; sy=y; clamp_scroll(); update_index(); refresh(); }
static void navigate(int n) {
    if(!doc || n<0 || n>=count) return;
    index_=n; sy=slots[n].y-MARGIN; refresh();
}
static void visible_range(int *a,int *b) { int vw, vh; view_size(&vw,&vh); *a=page_at(sy); *b=page_at(sy+vh); }
static FPDF_BITMAP ensure_bitmap(int i) {
    Slot *s=&slots[i];
    double scale=1, px=(double)s->pw*s->ph;
    if(px>MAX_PIXELS) scale=sqrt(MAX_PIXELS/px);   // huge pages render smaller and are stretched
    int rw=max(1,(int)(s->pw*scale)), rh=max(1,(int)(s->ph*scale));
    if(s->bmp && s->bw==rw && s->bh==rh && s->rot==rotation) return s->bmp;
    free_slot(s);
    if(s->failed) return NULL;
    FPDF_PAGE p=FPDF_LoadPage(doc,i);
    if(!p) { s->failed=TRUE; return NULL; }
    FPDF_BITMAP b=FPDFBitmap_Create(rw,rh,0);
    if(b) {
        FPDFBitmap_FillRect(b,0,0,rw,rh,0xFFFFFFFF);
        FPDF_RenderPageBitmap(b,p,0,0,rw,rh,rotation,FPDF_ANNOT);
        s->bmp=b; s->bw=rw; s->bh=rh; s->rot=rotation;
    }
    FPDF_ClosePage(p);
    return b;
}
// Keep rendered bitmaps only for visible pages and one neighbour on each side.
static void trim_cache(int a,int b) { for(int i=0;i<count;i++) if(i<a-1 || i>b+1) free_slot(&slots[i]); }
static void open_path(const wchar_t *path) {
    // Memory loading preserves Korean and other Unicode Windows paths.
    HANDLE f=CreateFileW(path,GENERIC_READ,FILE_SHARE_READ,NULL,OPEN_EXISTING,0,NULL);
    if(f==INVALID_HANDLE_VALUE) { error(L"파일을 열 수 없습니다."); return; }
    LARGE_INTEGER size; DWORD got;
    if(!GetFileSizeEx(f,&size) || size.QuadPart<=0 || size.QuadPart>512LL*1024*1024) {
        CloseHandle(f); error(L"파일이 비어 있거나 지원 크기(512MB)를 초과합니다."); return;
    }
    void *bytes=malloc((size_t)size.QuadPart);
    if(!bytes || !ReadFile(f,bytes,(DWORD)size.QuadPart,&got,NULL) || got!=(DWORD)size.QuadPart) {
        free(bytes); CloseHandle(f); error(L"파일을 읽지 못했습니다."); return;
    }
    CloseHandle(f);
    FPDF_DOCUMENT next=FPDF_LoadMemDocument64(bytes,(size_t)size.QuadPart,NULL);
    if(!next) {
        unsigned long e=FPDF_GetLastError(); free(bytes);
        error(e==FPDF_ERR_PASSWORD?L"암호로 보호된 PDF는 현재 지원하지 않습니다.":L"올바른 PDF가 아니거나 파일이 손상되었습니다."); return;
    }
    int pages=FPDF_GetPageCount(next);
    FPDF_PAGE first=pages>0?FPDF_LoadPage(next,0):NULL;
    Slot *ns=first?calloc((size_t)pages,sizeof(Slot)):NULL;
    if(first) FPDF_ClosePage(first);
    if(!ns) { FPDF_CloseDocument(next); free(bytes); error(first?L"메모리가 부족합니다.":L"표시할 페이지가 없습니다."); return; }
    for(int i=0;i<pages;i++) {
        double w=0, h=0;
        if(!FPDF_GetPageSizeByIndex(next,i,&w,&h) || w<=0 || h<=0) { w=612; h=792; }
        ns[i].w=w; ns[i].h=h;
    }
    close_doc();
    data=bytes; doc=next; slots=ns; count=pages; index_=rotation=sx=sy=0; fit=TRUE;
    lstrcpynW(filename,path,32768);
    const wchar_t *base=wcsrchr(path,L'\\'); wchar_t title[1024];
    swprintf(title,1024,L"DaPDF — %.950ls",base?base+1:path); SetWindowTextW(window,title);
    layout(); refresh();
}
static void choose(void) {
    wchar_t path[32768]=L"";
    OPENFILENAMEW of={0}; of.lStructSize=sizeof(of); of.hwndOwner=window;
    of.lpstrFilter=L"PDF 파일 (*.pdf)\0*.pdf\0모든 파일\0*.*\0"; of.lpstrFile=path; of.nMaxFile=32768;
    of.Flags=OFN_FILEMUSTEXIST|OFN_PATHMUSTEXIST|OFN_NOCHANGEDIR;
    if(GetOpenFileNameW(&of)) open_path(path);
}
static void command(int id) {
    if(id==OPEN) { choose(); return; } if(!doc) return;
    switch(id) {
    case PREV: navigate(index_-1); return;
    case NEXT: navigate(index_+1); return;
    case MINUS: fit=FALSE; zoom=max(.1,zoom/1.2); relayout(); return;
    case PLUS: fit=FALSE; zoom=min(6.0,zoom*1.2); relayout(); return;
    case FIT: fit=TRUE; relayout(); return;
    case ROTATE: rotation=(rotation+1)%4; layout(); sx=0; navigate(index_); return;
    }
}
static void paint(HDC dc,const RECT *r) {
    RECT bar=*r; bar.bottom=TOP; FillRect(dc,&bar,(HBRUSH)(COLOR_BTNFACE+1));
    RECT view=*r; view.top=TOP;
    HBRUSH bg=CreateSolidBrush(RGB(39,44,54)); FillRect(dc,&view,bg); DeleteObject(bg);
    if(!doc) {
        SelectObject(dc,font); SetBkMode(dc,TRANSPARENT); SetTextColor(dc,RGB(220,225,235));
        DrawTextW(dc,L"DaPDF\n\nPDF 파일을 여기에 놓거나\nPDF 열기 버튼을 누르세요.\n\nCtrl+O  열기  ·  Ctrl+휠  확대/축소",-1,&view,DT_CENTER|DT_VCENTER|DT_WORDBREAK);
        return;
    }
    int vw, vh, a, b; view_size(&vw,&vh); visible_range(&a,&b);
    int ox=doc_w<=vw?(vw-doc_w)/2:-sx, inner=doc_w-2*MARGIN;
    IntersectClipRect(dc,0,TOP,r->right,r->bottom);
    SetStretchBltMode(dc,HALFTONE); SetBrushOrgEx(dc,0,0,NULL);
    for(int i=a;i<=b;i++) {
        Slot *s=&slots[i];
        int x=ox+MARGIN+(inner-s->pw)/2, y=TOP+s->y-sy;
        FPDF_BITMAP bm=ensure_bitmap(i);
        if(bm) {
            BITMAPINFO bi={0}; bi.bmiHeader.biSize=sizeof(BITMAPINFOHEADER); bi.bmiHeader.biWidth=s->bw; bi.bmiHeader.biHeight=-s->bh;
            bi.bmiHeader.biPlanes=1; bi.bmiHeader.biBitCount=32; bi.bmiHeader.biCompression=BI_RGB;
            StretchDIBits(dc,x,y,s->pw,s->ph,0,0,s->bw,s->bh,FPDFBitmap_GetBuffer(bm),&bi,DIB_RGB_COLORS,SRCCOPY);
        } else {
            RECT pr={x,y,x+s->pw,y+s->ph}; FillRect(dc,&pr,(HBRUSH)GetStockObject(GRAY_BRUSH));
        }
    }
    trim_cache(a,b);
}
static LRESULT CALLBACK proc(HWND hwnd,UINT msg,WPARAM wp,LPARAM lp) {
    switch(msg) {
    case WM_CREATE: {
        window=hwnd;
        font=CreateFontW(-16,0,0,0,FW_NORMAL,FALSE,FALSE,FALSE,DEFAULT_CHARSET,0,0,CLEARTYPE_QUALITY,0,L"Malgun Gothic");
        const wchar_t *names[]={L"PDF 열기",L"이전",L"다음",L"−",L"+",L"너비 맞춤",L"회전"};
        int widths[]={100,60,60,42,42,100,60}, x=12;
        for(int i=0;i<7;i++) { buttons[i]=CreateWindowW(L"BUTTON",names[i],WS_CHILD|WS_VISIBLE|BS_PUSHBUTTON,x,12,widths[i],34,hwnd,(HMENU)(INT_PTR)(OPEN+i),NULL,NULL); SendMessageW(buttons[i],WM_SETFONT,(WPARAM)font,TRUE); x+=widths[i]+6; }
        label=CreateWindowW(L"STATIC",L"",WS_CHILD|WS_VISIBLE,620,20,320,24,hwnd,NULL,NULL,NULL); SendMessageW(label,WM_SETFONT,(WPARAM)font,TRUE);
        DragAcceptFiles(hwnd,TRUE); return 0;
    }
    case WM_SIZE: relayout(); return 0;
    case WM_COMMAND: command(LOWORD(wp)); SetFocus(hwnd); return 0;
    case WM_DROPFILES: { wchar_t path[32768]; DragQueryFileW((HDROP)wp,0,path,32768); DragFinish((HDROP)wp); open_path(path); return 0; }
    case WM_KEYDOWN: {
        BOOL ctrl=(GetKeyState(VK_CONTROL)&0x8000)!=0;
        if(ctrl && wp=='O') command(OPEN);
        else if(wp==VK_NEXT || wp==VK_RIGHT) command(NEXT);
        else if(wp==VK_PRIOR || wp==VK_LEFT) command(PREV);
        else if(wp==VK_HOME) navigate(0);
        else if(wp==VK_END) navigate(count-1);
        else if(wp==VK_ADD || wp==VK_OEM_PLUS) command(PLUS);
        else if(wp==VK_SUBTRACT || wp==VK_OEM_MINUS) command(MINUS);
        else if(!ctrl && wp=='F') command(FIT);
        else if(!ctrl && wp=='R') command(ROTATE);
        else if(wp==VK_DOWN || wp==VK_UP) scroll_to(sx,sy+(wp==VK_DOWN?80:-80));
        return 0;
    }
    case WM_MOUSEWHEEL:
        if(GET_KEYSTATE_WPARAM(wp)&MK_CONTROL) command(GET_WHEEL_DELTA_WPARAM(wp)>0?PLUS:MINUS);
        else scroll_to(sx,sy-GET_WHEEL_DELTA_WPARAM(wp));
        return 0;
    case WM_VSCROLL: case WM_HSCROLL: {
        int bar=msg==WM_VSCROLL?SB_VERT:SB_HORZ;
        SCROLLINFO si={0}; si.cbSize=sizeof(si); si.fMask=SIF_ALL; GetScrollInfo(hwnd,bar,&si);
        int pos=si.nPos;
        switch(LOWORD(wp)) { case SB_LINEUP: pos-=50; break; case SB_LINEDOWN: pos+=50; break; case SB_PAGEUP: pos-=si.nPage; break; case SB_PAGEDOWN: pos+=si.nPage; break; case SB_THUMBTRACK: pos=si.nTrackPos; break; case SB_TOP: pos=0; break; case SB_BOTTOM: pos=si.nMax; break; }
        if(bar==SB_VERT) scroll_to(sx,pos); else scroll_to(pos,sy);
        return 0;
    }
    case WM_ERASEBKGND: return 1;
    case WM_PAINT: {
        // Double-buffered so scrolling through many pages does not flicker.
        PAINTSTRUCT ps; HDC dc=BeginPaint(hwnd,&ps); RECT r; GetClientRect(hwnd,&r);
        HDC mem=CreateCompatibleDC(dc); HBITMAP buf=CreateCompatibleBitmap(dc,max(1,r.right),max(1,r.bottom));
        HGDIOBJ old=SelectObject(mem,buf);
        paint(mem,&r);
        BitBlt(dc,0,0,r.right,r.bottom,mem,0,0,SRCCOPY);
        SelectObject(mem,old); DeleteObject(buf); DeleteDC(mem);
        EndPaint(hwnd,&ps); return 0;
    }
    case WM_GETMINMAXINFO: ((MINMAXINFO*)lp)->ptMinTrackSize.x=1000; ((MINMAXINFO*)lp)->ptMinTrackSize.y=400; return 0;
    case WM_DESTROY: close_doc(); DeleteObject(font); PostQuitMessage(0); return 0;
    } return DefWindowProcW(hwnd,msg,wp,lp);
}
int WINAPI wWinMain(HINSTANCE instance,HINSTANCE previous,PWSTR args,int show) {
    (void)previous; (void)args;
    SetProcessDPIAware(); FPDF_InitLibrary();
    WNDCLASSW wc={0}; wc.lpfnWndProc=proc; wc.hInstance=instance; wc.lpszClassName=L"DaPDFWindow"; wc.hCursor=LoadCursor(NULL,IDC_ARROW); wc.hIcon=LoadIcon(NULL,IDI_APPLICATION);
    if(!RegisterClassW(&wc)) { FPDF_DestroyLibrary(); return 1; }
    HWND hwnd=CreateWindowW(wc.lpszClassName,L"DaPDF",WS_OVERLAPPEDWINDOW|WS_CLIPCHILDREN|WS_VSCROLL|WS_HSCROLL,CW_USEDEFAULT,CW_USEDEFAULT,1120,820,NULL,NULL,instance,NULL);
    if(!hwnd) { FPDF_DestroyLibrary(); return 1; }
    ShowWindow(hwnd,show); refresh();
    int argc; LPWSTR *argv=CommandLineToArgvW(GetCommandLineW(),&argc); if(argv && argc>1) open_path(argv[1]); if(argv) LocalFree(argv);
    MSG msg; while(GetMessageW(&msg,NULL,0,0)>0) { TranslateMessage(&msg); DispatchMessageW(&msg); }
    FPDF_DestroyLibrary(); return 0;
}
