use std::io::{Write};

fn main() {
    let path = std::env::args().skip(1).next().unwrap();
    let image = image::open(path).unwrap();
    let image = image.to_luma8();
    assert_eq!(image.width(), 128);
    assert_eq!(image.height(), 128);
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    for cy in 0..8 {
        for cx in 0..16 {
            for y in 0..16 {
                for x in 0..8 {
                    let byte = image.get_pixel(cx * 8 + x, cy * 16 + y);
                    if byte.0[0] < 25 {
                        assert_eq!(stdout.write(&[0xFF]).unwrap(), 1);
                    } else {
                        assert_eq!(stdout.write(&[0x00]).unwrap(), 1);
                    }
                }
            }
        }
    }
}
