use iced::mouse;
use iced::widget::canvas as canvas_widget;
use iced::widget::canvas::{self, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, Vector};

use super::Message;

const HANDLE: f32 = 7.0;
const GRAB: f32 = 10.0;
const INSET: f32 = HANDLE / 2.0;
const EDGE_SHARE: f32 = 1.0 / 3.0;
const LEAD: f32 = 22.0;
const KNOB: f32 = 5.5;
const KNOB_GRAB: f32 = 11.0;
const ORBIT: f32 = 1.0;
const OUTLINE: f32 = 2.5;
const FLAT: f32 = 1.0;
const CLICK_SLOP: f32 = 3.0;
const OPACITY_STEP: f32 = 0.04;
const INK: Color = Color::from_rgb(0.64, 0.42, 0.94);
const LIVE_INK: Color = Color::from_rgb(0.79, 0.60, 1.0);
const TURN_INK: Color = Color::from_rgb(0.38, 0.20, 0.62);
const TURN_LIVE: Color = Color::from_rgb(0.55, 0.32, 0.85);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grip {
    Move,
    Rotate,
    Scale { across: i8, down: i8 },
}

impl Grip {
    pub fn corners(self) -> Option<(usize, usize)> {
        let Grip::Scale { across, down } = self else {
            return None;
        };

        let seat = |right: bool, bottom: bool| usize::from(right) * 2 + usize::from(bottom);
        let (grabbed_right, grabbed_bottom) = (across > 0, down > 0);
        let flip = |side: i8, held: bool| if side == 0 { held } else { !held };

        Some((
            seat(grabbed_right, grabbed_bottom),
            seat(flip(across, grabbed_right), flip(down, grabbed_bottom)),
        ))
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sweep {
    pub grip: Grip,
    pub travel: Vector,
    pub spun: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Turn {
    Halt,
    Pick(usize),
    Grab(usize),
    Begin(usize, Grip),
    Drag(Sweep),
    Drop,
    Fade(i32),
}

#[derive(Default)]
pub(super) struct State {
    shown: bool,
    live: bool,
}

impl State {
    pub(super) fn show(&mut self, shown: bool) {
        self.shown = shown;

        if !shown {
            self.live = false;
        }
    }

    pub(super) fn seize(&mut self, live: bool) {
        self.live = live;
    }

    pub(super) fn view<'a>(
        &self,
        picked: Option<usize>,
        pivot: (f32, f32),
        posed: Vec<(usize, [f32; 8])>,
        camera: (Vector, f32),
    ) -> Element<'a, Message> {
        canvas_widget(Gizmo { picked, shown: self.shown, pivot, posed, camera, live: self.live })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

#[derive(Default)]
pub(super) struct Track {
    grip: Option<Grip>,
    from: Point,
    anchor: Point,
    idle: Option<Point>,
}

struct Gizmo {
    picked: Option<usize>,
    shown: bool,
    pivot: (f32, f32),
    posed: Vec<(usize, [f32; 8])>,
    camera: (Vector, f32),
    live: bool,
}

impl Gizmo {
    fn seat(&self, bounds: Rectangle) -> impl Fn(f32, f32) -> Point + '_ {
        let (pan, zoom) = self.camera;
        let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);

        move |x, y| Point::new(centre.x + (x + pan.x) * zoom, centre.y + (y + pan.y) * zoom)
    }

    fn quad(&self, bounds: Rectangle, part: usize) -> Option<[Point; 4]> {
        let seat = self.seat(bounds);
        let (_, vertices) = self.posed.iter().find(|(at, _)| *at == part)?;

        Some(std::array::from_fn(|at| seat(vertices[at * 2], vertices[at * 2 + 1])))
    }

    fn held(&self, bounds: Rectangle) -> Option<[Point; 4]> {
        self.picked.and_then(|part| self.quad(bounds, part))
    }

    fn claimed(&self, bounds: Rectangle, at: Point) -> Option<Grip> {
        let quad = self.held(bounds)?;
        let grip = reachable(&quad, at).then(|| grip_at(&quad, at))?;

        if grip == Grip::Move && topmost(&self.posed, at, &self.seat(bounds)) != self.picked {
            return None;
        }

        Some(grip)
    }
}

pub(super) fn topmost(posed: &[(usize, [f32; 8])], at: Point, seat: &impl Fn(f32, f32) -> Point) -> Option<usize> {
    posed
        .iter()
        .rev()
        .find(|(_, vertices)| {
            let quad: [Point; 4] = std::array::from_fn(|i| seat(vertices[i * 2], vertices[i * 2 + 1]));

            inside(&quad, at)
        })
        .map(|(part, _)| *part)
}

pub(super) fn grip_at(quad: &[Point; 4], at: Point) -> Grip {
    if turning(quad, at) {
        return Grip::Rotate;
    }

    let [top_left, bottom_left, top_right, _] = *quad;
    let side = |ratio: f32, from: Point, to: Point| {
        let reach = (to.x - from.x).hypot(to.y - from.y);
        let band = INSET.min(reach * EDGE_SHARE);

        match (ratio * reach, (1.0 - ratio) * reach) {
            (near, _) if (-GRAB..=band).contains(&near) => -1,
            (_, far) if (-GRAB..=band).contains(&far) => 1,
            _ => 0,
        }
    };

    match (
        side(span(top_left, top_right, at), top_left, top_right),
        side(span(top_left, bottom_left, at), top_left, bottom_left),
    ) {
        (0, 0) => Grip::Move,
        (across, down) => Grip::Scale { across, down },
    }
}

pub(super) fn reachable(quad: &[Point; 4], at: Point) -> bool {
    if turning(quad, at) {
        return true;
    }

    if collapsed(quad) {
        let centre = centroid(quad);

        return (at.x - centre.x).hypot(at.y - centre.y) <= GRAB;
    }

    let [top_left, bottom_left, top_right, _] = *quad;
    let over = |ratio: f32, from: Point, to: Point| {
        let reach = (to.x - from.x).hypot(to.y - from.y);

        (if ratio < 0.0 { -ratio } else { ratio - 1.0 }).max(0.0) * reach
    };

    over(span(top_left, top_right, at), top_left, top_right) <= GRAB
        && over(span(top_left, bottom_left, at), top_left, bottom_left) <= GRAB
}

pub(super) fn turning(quad: &[Point; 4], at: Point) -> bool {
    let knob = lever(quad).1;

    (at.x - knob.x).hypot(at.y - knob.y) <= KNOB_GRAB
}

pub(super) fn lever(quad: &[Point; 4]) -> (Point, Point) {
    let [top_left, _, top_right, bottom_right] = *quad;
    let axis = Vector::new(top_right.x - top_left.x, top_right.y - top_left.y);
    let reach = axis.x.hypot(axis.y);
    let step = match reach > f32::EPSILON {
        true => Vector::new(axis.x / reach, axis.y / reach),
        false => Vector::new(1.0, 0.0),
    };

    let edge = Point::new((top_right.x + bottom_right.x) / 2.0, (top_right.y + bottom_right.y) / 2.0);

    (edge, Point::new(edge.x + step.x * LEAD, edge.y + step.y * LEAD))
}

pub(super) fn fade(delta: f32) -> i32 {
    (delta * OPACITY_STEP * 1000.0).round() as i32
}

pub(super) fn anchor_of(quad: &[Point; 4], pivot: (f32, f32)) -> Point {
    let [top_left, bottom_left, top_right, _] = *quad;
    let (across, down) = pivot;

    Point::new(
        top_left.x + across * (top_right.x - top_left.x) + down * (bottom_left.x - top_left.x),
        top_left.y + across * (top_right.y - top_left.y) + down * (bottom_left.y - top_left.y),
    )
}

pub(super) fn swept(anchor: Point, from: Point, at: Point, grip: Grip) -> Sweep {
    let orbiting = (from.x - anchor.x).hypot(from.y - anchor.y) > ORBIT
        && (at.x - anchor.x).hypot(at.y - anchor.y) > ORBIT;

    Sweep {
        grip,
        travel: Vector::new(at.x - from.x, at.y - from.y),
        spun: match orbiting {
            true => wrapped(turn_at(anchor, at) - turn_at(anchor, from)),
            false => 0.0,
        },
    }
}

fn turn_at(centre: Point, at: Point) -> f32 {
    (at.y - centre.y).atan2(at.x - centre.x)
}

fn wrapped(mut angle: f32) -> f32 {
    while angle > std::f32::consts::PI {
        angle -= std::f32::consts::TAU;
    }

    while angle < -std::f32::consts::PI {
        angle += std::f32::consts::TAU;
    }

    angle
}

fn span(from: Point, to: Point, at: Point) -> f32 {
    let axis = Vector::new(to.x - from.x, to.y - from.y);
    let reach = axis.x * axis.x + axis.y * axis.y;

    if reach <= f32::EPSILON {
        return 0.5;
    }

    ((at.x - from.x) * axis.x + (at.y - from.y) * axis.y) / reach
}

fn collapsed(quad: &[Point; 4]) -> bool {
    let [top_left, bottom_left, top_right, _] = *quad;
    let across = Vector::new(top_right.x - top_left.x, top_right.y - top_left.y);
    let down = Vector::new(bottom_left.x - top_left.x, bottom_left.y - top_left.y);

    (across.x * down.y - across.y * down.x).abs() <= FLAT
}

fn inside(quad: &[Point; 4], at: Point) -> bool {
    if collapsed(quad) {
        return false;
    }

    let [top_left, bottom_left, top_right, _] = *quad;
    let across = span(top_left, top_right, at);
    let down = span(top_left, bottom_left, at);

    (0.0..=1.0).contains(&across) && (0.0..=1.0).contains(&down)
}

fn centroid(quad: &[Point; 4]) -> Point {
    let sum = quad.iter().fold((0.0, 0.0), |held, point| (held.0 + point.x, held.1 + point.y));

    Point::new(sum.0 / 4.0, sum.1 / 4.0)
}

impl canvas::Program<Message> for Gizmo {
    type State = Track;

    fn draw(
        &self,
        _state: &Track,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let Some(quad) = self.held(bounds).filter(|_| self.shown) else {
            return Vec::new();
        };

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let ink = if self.live { LIVE_INK } else { INK };

        let [top_left, bottom_left, top_right, bottom_right] = quad;
        let outline = Path::new(|builder| {
            builder.move_to(top_left);
            builder.line_to(top_right);
            builder.line_to(bottom_right);
            builder.line_to(bottom_left);
            builder.close();
        });

        frame.stroke(&outline, Stroke::default().with_color(ink).with_width(OUTLINE));

        let turn = if self.live { TURN_LIVE } else { TURN_INK };
        let (edge, knob) = lever(&quad);

        frame.stroke(&Path::line(edge, knob), Stroke::default().with_color(turn).with_width(OUTLINE));
        frame.fill(&Path::circle(knob, KNOB), turn);
        frame.stroke(&Path::circle(knob, KNOB), Stroke::default().with_color(Color::BLACK).with_width(1.0));

        let midpoint = |a: Point, b: Point| Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        let handles = [
            top_left,
            top_right,
            bottom_left,
            bottom_right,
            midpoint(top_left, top_right),
            midpoint(bottom_left, bottom_right),
            midpoint(top_left, bottom_left),
            midpoint(top_right, bottom_right),
        ];

        for handle in handles {
            let box_at = Path::rectangle(
                Point::new(handle.x - HANDLE / 2.0, handle.y - HANDLE / 2.0),
                iced::Size::new(HANDLE, HANDLE),
            );

            frame.fill(&box_at, ink);
            frame.stroke(&box_at, Stroke::default().with_color(Color::BLACK).with_width(1.0));
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        track: &mut Track,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let iced::Event::Mouse(mouse) = event else {
            return None;
        };

        match mouse {
            mouse::Event::ButtonPressed(button) => {
                let Some(at) = cursor.position_in(bounds) else {
                    return self
                        .picked
                        .map(|_| canvas::Action::publish(Message::Gizmo(Turn::Halt)));
                };

                if *button == mouse::Button::Left {
                    track.idle = Some(at);

                    return None;
                }

                if *button != mouse::Button::Right {
                    return None;
                }

                let seat = self.seat(bounds);

                match (self.claimed(bounds, at), self.picked) {
                    (Some(grip), Some(part)) => {
                        track.grip = Some(grip);
                        track.from = at;
                        track.anchor = self.held(bounds).map_or(at, |quad| anchor_of(&quad, self.pivot));

                        Some(canvas::Action::publish(Message::Gizmo(Turn::Begin(part, grip))).and_capture())
                    }
                    _ => {
                        let turn = topmost(&self.posed, at, &seat).map_or(Turn::Halt, Turn::Grab);

                        Some(canvas::Action::publish(Message::Gizmo(turn)).and_capture())
                    }
                }
            }
            mouse::Event::ButtonReleased(mouse::Button::Right) => {
                track.grip.take()?;

                Some(canvas::Action::publish(Message::Gizmo(Turn::Drop)).and_capture())
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                let from = track.idle.take()?;
                let at = cursor.position_in(bounds)?;

                if (at.x - from.x).hypot(at.y - from.y) > CLICK_SLOP {
                    return None;
                }

                let seat = self.seat(bounds);
                let turn = topmost(&self.posed, at, &seat).map_or(Turn::Halt, Turn::Pick);

                Some(canvas::Action::publish(Message::Gizmo(turn)))
            }
            mouse::Event::CursorMoved { .. } => {
                let grip = track.grip?;
                let at = cursor.position_in(bounds)?;
                let sweep = swept(track.anchor, track.from, at, grip);

                track.from = at;

                Some(canvas::Action::publish(Message::Gizmo(Turn::Drag(sweep))).and_capture())
            }
            mouse::Event::WheelScrolled { delta } if track.grip.is_some() => {
                let step = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };

                Some(canvas::Action::publish(Message::Gizmo(Turn::Fade(fade(step)))).and_capture())
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Track,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let Some(at) = cursor.position_in(bounds) else {
            return mouse::Interaction::None;
        };

        let Some(grip) = self.claimed(bounds, at).filter(|_| self.shown) else {
            return mouse::Interaction::None;
        };

        match grip {
            Grip::Move => mouse::Interaction::Move,
            Grip::Rotate => mouse::Interaction::Grab,
            Grip::Scale { .. } => mouse::Interaction::ResizingDiagonallyDown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad() -> [Point; 4] {
        [Point::new(0.0, 0.0), Point::new(0.0, 100.0), Point::new(100.0, 0.0), Point::new(100.0, 100.0)]
    }

    #[test]
    fn the_scale_band_bleeds_outward_from_the_edge_not_inward() {
        // The handles sit on the edge, so the band reaches `GRAB` px out and only far
        // enough in to cover the half of the handle that overlaps the part.
        assert_eq!(grip_at(&quad(), Point::new(-6.0, 50.0)), Grip::Scale { across: -1, down: 0 });
        assert_eq!(grip_at(&quad(), Point::new(2.0, 50.0)), Grip::Scale { across: -1, down: 0 });
        assert_eq!(grip_at(&quad(), Point::new(10.0, 50.0)), Grip::Move);
        assert_eq!(grip_at(&quad(), Point::new(50.0, 50.0)), Grip::Move);
        assert_eq!(grip_at(&quad(), Point::new(103.0, 50.0)), Grip::Scale { across: 1, down: 0 });
        assert_eq!(grip_at(&quad(), Point::new(2.0, 103.0)), Grip::Scale { across: -1, down: 1 });
    }

    #[test]
    fn a_part_too_small_to_spare_the_band_keeps_a_middle_to_move_by() {
        let small = [Point::new(0.0, 0.0), Point::new(0.0, 16.0), Point::new(16.0, 0.0), Point::new(16.0, 16.0)];

        assert_eq!(grip_at(&small, Point::new(8.0, 8.0)), Grip::Move);
        assert_eq!(grip_at(&small, Point::new(1.0, 8.0)), Grip::Scale { across: -1, down: 0 });
    }

    #[test]
    fn rotating_needs_the_knob_and_nothing_else_outside_the_box() {
        // The old ring claimed everything within tens of pixels of the part; now the
        // only rotate target is the ball on the end of the lever.
        let (edge, knob) = lever(&quad());

        assert_eq!(edge, Point::new(100.0, 50.0));
        assert_eq!(knob, Point::new(100.0 + LEAD, 50.0));
        assert_eq!(grip_at(&quad(), knob), Grip::Rotate);
        assert!(reachable(&quad(), knob));

        assert!(!turning(&quad(), Point::new(-30.0, 50.0)));
        assert!(!reachable(&quad(), Point::new(-30.0, 50.0)));
    }

    #[test]
    fn the_lever_hangs_off_the_box_and_turns_with_it_whatever_the_pivot_does() {
        // Parts whose pivot sits outside their own sprite used to fling the knob far
        // away from the box, or behind it when the bias went past one.
        let turned = [Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(0.0, 100.0), Point::new(100.0, 100.0)];
        let (_, sideways) = lever(&turned);

        assert_eq!(sideways, Point::new(50.0, 100.0 + LEAD));
    }

    #[test]
    fn a_cursor_sitting_on_the_rotation_centre_sweeps_nothing() {
        let stuck = swept(Point::new(50.0, 50.0), Point::new(50.0, 50.0), Point::new(50.2, 50.0), Grip::Rotate);

        assert_eq!(stuck.spun, 0.0);
    }

    #[test]
    fn a_collapsed_part_claims_its_own_pixels_rather_than_the_whole_canvas() {
        // Scaling a part to zero used to make every span read 0.5 and swallow every click,
        // which is what left the viewer feeling frozen. It stays grabbable up close so
        // the part can be dragged back out, and picking it never sees it at all.
        let squashed = [Point::new(50.0, 50.0); 4];

        assert!(!inside(&squashed, Point::new(400.0, 12.0)));
        assert!(!reachable(&squashed, Point::new(400.0, 12.0)));
        assert!(reachable(&squashed, Point::new(56.0, 50.0)));
        assert_eq!(topmost(&[(3, [50.0; 8])], Point::new(50.0, 50.0), &|x, y| Point::new(x, y)), None);
    }

    #[test]
    fn the_grabbed_corner_and_its_anchor_sit_opposite_each_other() {
        // Corners are [top left, bottom left, top right, bottom right].
        assert_eq!(Grip::Scale { across: 1, down: 1 }.corners(), Some((3, 0)));
        assert_eq!(Grip::Scale { across: -1, down: -1 }.corners(), Some((0, 3)));
        assert_eq!(Grip::Scale { across: -1, down: 0 }.corners(), Some((0, 2)));
        assert_eq!(Grip::Move.corners(), None);
    }

    #[test]
    fn a_quarter_turn_around_the_anchor_reads_as_a_quarter_turn() {
        let sweep = swept(Point::new(50.0, 50.0), Point::new(150.0, 50.0), Point::new(50.0, 150.0), Grip::Rotate);

        assert!((sweep.spun - std::f32::consts::FRAC_PI_2).abs() < 1.0e-5);
    }

    #[test]
    fn the_anchor_follows_the_pivot_bias_across_the_quad() {
        assert_eq!(anchor_of(&quad(), (0.5, 0.5)), Point::new(50.0, 50.0));
        assert_eq!(anchor_of(&quad(), (0.0, 1.0)), Point::new(0.0, 100.0));
    }

    #[test]
    fn the_topmost_part_wins_where_two_overlap() {
        // `part::resolve` hands parts back in draw order, so the last one that
        // contains the cursor is the one drawn on top.
        let posed = vec![
            (3, [0.0, 0.0, 0.0, 100.0, 100.0, 0.0, 100.0, 100.0]),
            (7, [50.0, 50.0, 50.0, 150.0, 150.0, 50.0, 150.0, 150.0]),
        ];
        let seat = |x: f32, y: f32| Point::new(x, y);

        assert_eq!(topmost(&posed, Point::new(75.0, 75.0), &seat), Some(7));
        assert_eq!(topmost(&posed, Point::new(10.0, 10.0), &seat), Some(3));
        assert_eq!(topmost(&posed, Point::new(400.0, 400.0), &seat), None);
    }
}
