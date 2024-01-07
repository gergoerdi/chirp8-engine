#[derive(Clone, Copy)]
pub struct Quirks {
    pub shift_vy: bool,
    pub reset_vf: bool,
    pub increment_ptr: bool,
    pub video_wait: bool,
    pub clip_sprites: bool,
}

impl Default for Quirks {
    fn default() -> Self {
        Quirks {
            shift_vy: true,
            reset_vf: true,
            increment_ptr: true,
            video_wait: true,
            clip_sprites: true,
        }
    }
}
