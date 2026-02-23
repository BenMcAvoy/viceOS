use crate::FramebufferInfo;
use crate::drivers::keyboard;
use crate::arch;

use tiny_skia::*;

pub fn test_render_loop(fb: FramebufferInfo) -> ! {
    let mut pm = Pixmap::new(fb.width, fb.height).unwrap();

    let mut rectx: i32 = 0;
    let mut recty: i32 = 0;

    loop {
        // clear screen
        pm.fill(Color::from_rgba8(0, 0, 0, 255));

        // process input
        while let Some(key) = keyboard::get_char() {
            match key {
                'w' => recty -= 10,
                's' => recty += 10,
                'a' => rectx -= 10,
                'd' => rectx += 10,
                _ => {}
            }
        }

        // draw moving square
        let mut pb = PathBuilder::new();
        let x = rectx as f32;
        let y = recty as f32;
        pb.push_rect(Rect::from_xywh(x, y, 100.0, 100.0).unwrap());
        pb.close();
        let path = pb.finish().unwrap();

        let mut paint = Paint::default();
        paint.set_color_rgba8(255, 255, 255, 255);
        pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);

        // blit to framebuffer
        unsafe {
            let fb_addr = fb.address as *mut u32;
            let fb_width = fb.width as usize;
            let fb_height = fb.height as usize;

            let pixels = pm.pixels();

            for yy in 0..fb_height {
                for xx in 0..fb_width {
                    let pixel = pixels[yy * pm.width() as usize + xx];
                    let color = (pixel.red() as u32) << 16
                        | (pixel.green() as u32) << 8
                        | (pixel.blue() as u32);
                    *fb_addr.add(yy * fb_width + xx) = color;
                }
            }
        }
    }
}
