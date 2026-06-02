#[derive(Clone)]
pub struct Selection {
    pub start_x: u16,
    pub start_y: usize,
    pub end_x: u16,
    pub end_y: usize,
}

impl Selection {
    pub fn normalize(&self) -> (u16, usize, u16, usize) {
        if self.start_y < self.end_y {
            (self.start_x, self.start_y, self.end_x, self.end_y)
        } else if self.end_y < self.start_y {
            (self.end_x, self.end_y, self.start_x, self.start_y)
        } else {
            if self.start_x <= self.end_x {
                (self.start_x, self.start_y, self.end_x, self.end_y)
            } else {
                (self.end_x, self.start_y, self.start_x, self.end_y)
            }
        }
    }
}
