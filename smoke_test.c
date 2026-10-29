/* Integration check: real PDF loading, continuous layout, rendering and viewer transitions. */
#define wWinMain viewer_entry
#include "main.c"
#undef wWinMain
#include <assert.h>
static int dark_pixels(FPDF_BITMAP b) {
    unsigned char *pixels=FPDFBitmap_GetBuffer(b);
    int stride=FPDFBitmap_GetStride(b), w=FPDFBitmap_GetWidth(b), h=FPDFBitmap_GetHeight(b), dark=0;
    for(int y=0;y<h;y++) for(int x=0;x<w;x++) if(pixels[y*stride+x*4]<100) dark++;
    return dark;
}
int main(void) {
    FPDF_InitLibrary();
    WNDCLASSW wc={0}; wc.lpfnWndProc=proc; wc.hInstance=GetModuleHandleW(NULL); wc.lpszClassName=L"DaPDFTest";
    assert(RegisterClassW(&wc));
    window=CreateWindowW(wc.lpszClassName,L"Test",WS_OVERLAPPEDWINDOW,0,0,1100,800,NULL,NULL,wc.hInstance,NULL);
    assert(window);
    int vw, vh; view_size(&vw,&vh);

    open_path(L"test-한글.pdf"); assert(doc && slots && count==2 && index_==0);
    // Continuous layout: pages stacked top to bottom, width fitted to the window.
    assert(slots[0].y==MARGIN && slots[1].y==slots[0].y+slots[0].ph+GAP);
    assert(doc_h==slots[1].y+slots[1].ph+MARGIN && doc_w<=vw && slots[0].pw>100);

    int a, b; visible_range(&a,&b); assert(a==0);
    FPDF_BITMAP bm=ensure_bitmap(0); assert(bm && slots[0].bw==slots[0].pw && dark_pixels(bm)>20);
    assert(ensure_bitmap(0)==bm);                       // cached, not re-rendered
    ensure_bitmap(1); trim_cache(1,1); assert(slots[0].bmp && slots[1].bmp);
    // Scrolling moves the current page indicator.
    scroll_to(0,doc_h); assert(sy==max(0,doc_h-vh) && index_==1);
    scroll_to(0,-500); assert(sy==0 && index_==0);
    // Page navigation scrolls to the page top and respects bounds.
    command(NEXT); assert(index_==1 && sy==min(slots[1].y-MARGIN,max(0,doc_h-vh)));
    command(NEXT); assert(index_==1);
    command(PREV); assert(index_==0 && sy==0);
    // Zoom keeps the current page and re-renders at the new size.
    double z=zoom; command(PLUS); assert(!fit && fabs(zoom-z*1.2)<1e-9 && index_==0);
    assert(slots[0].pw>slots[0].bw && ensure_bitmap(0) && slots[0].bw==slots[0].pw);
    command(NEXT); int y1=sy; command(MINUS); command(PLUS); assert(index_==1 && abs(sy-y1)<=3);
    command(ROTATE); assert(rotation==1 && index_==1 && ensure_bitmap(1) && slots[1].rot==1);
    command(FIT); assert(fit && doc_w<=vw);
    // A huge zoom is capped per page, not per document.
    zoom=6.0; fit=FALSE; relayout(); ensure_bitmap(index_);
    assert((double)slots[index_].bw*slots[index_].bh<=MAX_PIXELS);
    DestroyWindow(window); assert(!doc && !slots);
    FPDF_DestroyLibrary();
    puts("PASS: Unicode path, continuous layout, render cache, scroll index, navigation, zoom anchor, rotation, fit, pixel cap, cleanup");
    return 0;
}
