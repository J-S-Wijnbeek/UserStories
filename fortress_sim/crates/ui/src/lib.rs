// UI helper types — macroquad rendering is in the fortress binary

pub struct Camera {
    pub offset_x: f32,
    pub offset_y: f32,
    pub tile_size: f32,
}

impl Camera {
    pub fn new() -> Self {
        Camera {
            offset_x: 0.0,
            offset_y: 0.0,
            tile_size: 12.0,
        }
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.offset_x += dx;
        self.offset_y += dy;
    }

    pub fn world_to_screen(&self, wx: f32, wy: f32) -> (f32, f32) {
        (wx * self.tile_size + self.offset_x, wy * self.tile_size + self.offset_y)
    }

    pub fn screen_to_world(&self, sx: f32, sy: f32) -> (i32, i32) {
        (
            ((sx - self.offset_x) / self.tile_size) as i32,
            ((sy - self.offset_y) / self.tile_size) as i32,
        )
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HudInfo {
    pub tick: u64,
    pub day: u64,
    pub dwarf_count: usize,
    pub job_count: usize,
}
