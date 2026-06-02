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

    pub fn sel_string(&self, text: &Vec<String>) -> String {
        let (start_x, start_y, end_x, end_y) = self.normalize();
        if start_y == end_y {
            text[start_y][start_x as usize..end_x as usize].to_string()
        } else {
            let mut result = String::new();
            result.push_str(&text[start_y][start_x as usize..]);
            result.push('\n');
            for y in (start_y + 1)..end_y {
                result.push_str(&text[y]);
                result.push('\n');
            }
            result.push_str(&text[end_y][..end_x as usize]);
            result
        }
    }
}
