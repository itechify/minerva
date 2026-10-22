//! Screen capture of the Title Region (Windows): a GDI BitBlt from the desktop DC.
//! It copies screen pixels only; it never uses PrintWindow (which would send the game
//! WM_PRINT) or any capture session tied to the game window (ADR 0001).

/// Capture `w`x`h` physical pixels at virtual-screen `(x, y)` as tightly packed RGBA,
/// top-down rows.
#[cfg(windows)]
pub fn capture_rgba(x: i32, y: i32, w: i32, h: i32) -> Result<Vec<u8>, String> {
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap,
        CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
        SRCCOPY, SelectObject,
    };
    if w <= 0 || h <= 0 {
        return Err(format!("empty capture {w}x{h}"));
    }
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp.into());
        let blt = BitBlt(mem, 0, 0, w, h, Some(screen), x, y, SRCCOPY | CAPTUREBLT);
        SelectObject(mem, old);
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let lines =
            GetDIBits(mem, bmp, 0, h as u32, Some(buf.as_mut_ptr().cast()), &mut bmi, DIB_RGB_COLORS);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        blt.map_err(|e| format!("BitBlt: {e}"))?;
        if lines != h {
            return Err(format!("GetDIBits returned {lines} of {h} rows"));
        }
        for px in buf.as_chunks_mut::<4>().0 {
            px.swap(0, 2); // BGRA -> RGBA
            px[3] = 0xFF;
        }
        Ok(buf)
    }
}

#[cfg(not(windows))]
pub fn capture_rgba(_x: i32, _y: i32, _w: i32, _h: i32) -> Result<Vec<u8>, String> {
    Err("screen capture is only implemented on Windows so far".into())
}
