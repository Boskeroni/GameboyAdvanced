use crate::ppu::LCD_WIDTH;

pub fn window_line() -> (Vec<u16>, Vec<u16>) {
    return (vec![0; LCD_WIDTH], vec![3; LCD_WIDTH]);
}