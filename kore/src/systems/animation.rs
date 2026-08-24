pub mod export;

use nyanko::graphics::rig::Animation;
use nyanko::graphics::tools::math;

pub const IDX_WALK: usize = 0;
pub const IDX_IDLE: usize = 1;
pub const IDX_ATTACK: usize = 2;
pub const IDX_KB: usize = 3;
pub const IDX_SPIRIT: usize = 4;
pub const IDX_BURROW: usize = 5;
pub const IDX_SURFACE: usize = 6;
pub const IDX_MODEL: usize = 99;
pub const IDX_NONE: usize = 999;

const LOOP_CEILING: i32 = 999_999;

pub fn loop_frame(animation: &Animation, frame: f32) -> f32 {
    let boundary = true_loop(animation).unwrap_or_else(|| furthest_frame(animation));

    if boundary > 0 { frame.rem_euclid(boundary as f32 + 1.0) } else { frame }
}

pub fn furthest_frame(animation: &Animation) -> i32 {
    animation.modifications.iter()
        .filter_map(|modification| modification.keyframes.last())
        .fold(0, |furthest, keyframe| furthest.max(keyframe.frame))
}

pub fn true_loop(animation: &Animation) -> Option<i32> {
    let furthest = furthest_frame(animation);
    let mut combined: i32 = 1;
    let mut looping = false;

    for modification in &animation.modifications {
        if modification.loop_count == 1 { return None; }

        let (Some(first), Some(last)) = (modification.keyframes.first(), modification.keyframes.last()) else {
            continue;
        };

        let span = last.frame - first.frame;
        if span <= 0 { continue; }

        combined = (combined / math::gcd(combined, span)).checked_mul(span)?;
        if combined > LOOP_CEILING { return None; }

        looping = true;
    }

    if !looping { return Some(furthest); }

    (combined >= furthest).then_some(combined)
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


