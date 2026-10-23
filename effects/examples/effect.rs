//! Standalone no_std Rust example compiled three ways; no native/WASI imports.
#![no_std]
use core::panic::PanicInfo;
static mut FRAME: [u8; 128 * 64 * 3] = [0; 128 * 64 * 3];
static mut PARAMS: [f64; 2] = [1.0, 1.0];
#[panic_handler]
fn panic(_: &PanicInfo) -> ! { loop {} }
#[no_mangle]
pub extern "C" fn set_param(index: i32, value: f64) {
    if (0..2).contains(&index) { unsafe { PARAMS[index as usize] = value; } }
}
#[no_mangle]
pub extern "C" fn render(time: f64, width: i32, height: i32) -> i32 {
    if width <= 0 || height <= 0 || width > 128 || height > 64 { return -1; }
    let w=width as usize;
    let h=height as usize;
    let (speed,brightness)=unsafe {(PARAMS[0],PARAMS[1])};
    let phase=(time * speed * 100.0) as u32;
    for y in 0..h {
        for x in 0..w {
            let a=((x as u32*256/w as u32 + phase + y as u32*2)%256) as u8;
            let rgb=if cfg!(effect="comet") {
                let distance=((x as u32*256/w as u32+256-phase%256)%256) as u8;
                let light=255u8.saturating_sub(distance.saturating_mul(5));
                [light,light/2,light.saturating_add(20)]
            } else if cfg!(effect="prism") {
                [a,255-a,((y*255/h) as u8).wrapping_add((phase/2) as u8)]
            } else {
                let wave=if a<128 {a.saturating_mul(2)} else {(255-a).saturating_mul(2)};
                [wave/3,255-wave/2,wave]
            };
            let i=(y*w+x)*3;
            unsafe {
                FRAME[i]=(rgb[0] as f64*brightness) as u8;
                FRAME[i+1]=(rgb[1] as f64*brightness) as u8;
                FRAME[i+2]=(rgb[2] as f64*brightness) as u8;
            }
        }
    }
    core::ptr::addr_of_mut!(FRAME) as i32
}
