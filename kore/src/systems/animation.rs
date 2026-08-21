pub mod export;

use nyanko::graphics::rig::Animation;

pub const IDX_WALK: usize = 0;
pub const IDX_IDLE: usize = 1;
pub const IDX_ATTACK: usize = 2;
pub const IDX_KB: usize = 3;
pub const IDX_SPIRIT: usize = 4;
pub const IDX_BURROW: usize = 5;
pub const IDX_SURFACE: usize = 6;
pub const IDX_MODEL: usize = 99;
pub const IDX_NONE: usize = 999;

pub fn loop_frame(animation: &Animation, frame: f32) -> f32 {
    let boundary = frame_count(animation);

    if boundary > 0 { frame.rem_euclid(boundary as f32) } else { frame }
}

pub fn frame_count(animation: &Animation) -> i32 {
    animation.playback_frames()
}

pub fn last_frame(animation: &Animation) -> i32 {
    frame_count(animation).saturating_sub(1).max(0)
}

#[inline(always)]
pub fn multiply_mat3(matrix_a: &[f32; 9], matrix_b: &[f32; 9]) -> [f32; 9] {
    [
        matrix_a[0]*matrix_b[0] + matrix_a[3]*matrix_b[1] + matrix_a[6]*matrix_b[2],
        matrix_a[1]*matrix_b[0] + matrix_a[4]*matrix_b[1] + matrix_a[7]*matrix_b[2],
        matrix_a[2]*matrix_b[0] + matrix_a[5]*matrix_b[1] + matrix_a[8]*matrix_b[2],

        matrix_a[0]*matrix_b[3] + matrix_a[3]*matrix_b[4] + matrix_a[6]*matrix_b[5],
        matrix_a[1]*matrix_b[3] + matrix_a[4]*matrix_b[4] + matrix_a[7]*matrix_b[5],
        matrix_a[2]*matrix_b[3] + matrix_a[5]*matrix_b[4] + matrix_a[8]*matrix_b[5],

        matrix_a[0]*matrix_b[6] + matrix_a[3]*matrix_b[7] + matrix_a[6]*matrix_b[8],
        matrix_a[1]*matrix_b[6] + matrix_a[4]*matrix_b[7] + matrix_a[7]*matrix_b[8],
        matrix_a[2]*matrix_b[6] + matrix_a[5]*matrix_b[7] + matrix_a[8]*matrix_b[8],
    ]
}
