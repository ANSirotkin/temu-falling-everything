mod body;
mod gravity;
mod material;
mod world;

use std::f32::consts::{PI, TAU};
use std::time::Instant;

use body::{Body, Shape};
use gravity::{GravityWell, UniformGravity, STANDARD_GRAVITY};
use macroquad::prelude::*;
use material::Material;
use world::World;

const WIDTH: i32 = 320;
const HEIGHT: i32 = 180;
const SCALE: f32 = 4.0;

const SIM_DT: f32 = 1.0 / 60.0;
const MAX_STEPS_PER_FRAME: u32 = 5;

const MAX_BODY_SPEED: f32 = 3.0; // cells per tick while held
const ROT_SPEED: f32 = 0.04; // radians per tick while Q/E is held

const NUMBER_KEYS: [KeyCode; 9] = [
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
    KeyCode::Key6,
    KeyCode::Key7,
    KeyCode::Key8,
    KeyCode::Key9,
];

fn window_conf() -> Conf {
    Conf {
        window_title: "Falling Sand".to_owned(),
        window_width: (WIDTH as f32 * SCALE) as i32,
        window_height: (HEIGHT as f32 * SCALE) as i32,
        ..Default::default()
    }
}

fn spawn_body(world: &mut World, bodies: &mut Vec<Body>, shape: Shape, at: (f32, f32)) {
    let mut body = Body::new(shape, at);
    world.move_body(&mut body, at, 0.0); // stamps it into the world
    bodies.push(body);
}

/// Move a held body toward the cursor, keeping the grabbed point under it.
fn hold_body(world: &mut World, body: &mut Body, cursor: (f32, f32), rotate: f32) {
    let Some(offset) = body.grab else { return };
    body.fall_speed = 0.0;

    let angle = body.angle + rotate * ROT_SPEED;
    let (c, s) = (angle.cos(), angle.sin());
    let wanted = (
        cursor.0 - (offset.0 * c - offset.1 * s),
        cursor.1 - (offset.0 * s + offset.1 * c),
    );

    // Limit how far it can move in one tick.
    let (dx, dy) = (wanted.0 - body.pos.0, wanted.1 - body.pos.1);
    let dist = dx.hypot(dy);
    let k = if dist > MAX_BODY_SPEED { MAX_BODY_SPEED / dist } else { 1.0 };
    let target = (
        (body.pos.0 + dx * k).clamp(0.0, WIDTH as f32),
        (body.pos.1 + dy * k).clamp(0.0, HEIGHT as f32),
    );

    if target != body.pos || angle != body.angle {
        world.move_body(body, target, angle);
    }
}

/// A released body falls until it lands, then rights itself.
/// (A stand-in until real rigid-body physics.)
fn drop_body(world: &mut World, body: &mut Body) {
    let angle = body.angle;

    body.fall_speed = (body.fall_speed + STANDARD_GRAVITY).min(3.0);
    let mut remaining = body.fall_speed;
    while remaining > 0.0 {
        let step = remaining.min(1.0);
        let target = (body.pos.0, body.pos.1 + step);
        if world.body_blocked(body, target, angle) {
            body.fall_speed = 0.0;
            break;
        }
        world.move_body(body, target, angle);
        remaining -= step;
    }

    // Ease the angle back toward upright.
    let mut a = body.angle % TAU;
    if a > PI {
        a -= TAU;
    } else if a < -PI {
        a += TAU;
    }
    let target_angle = if a.abs() > 0.005 { a * 0.9 } else { 0.0 };
    if target_angle != body.angle {
        let pos = body.pos;
        world.move_body(body, pos, target_angle);
    }
}

fn update_bodies(
    world: &mut World,
    bodies: &mut [Body],
    held: Option<usize>,
    cursor: (f32, f32),
    rotate: f32,
) {
    for (i, body) in bodies.iter_mut().enumerate() {
        if Some(i) == held {
            hold_body(world, body, cursor, rotate);
        } else {
            drop_body(world, body);
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut world = World::new(WIDTH, HEIGHT);

    // A floor to stand things on.
    for y in (HEIGHT - 3)..HEIGHT {
        for x in 0..WIDTH {
            world.set(x, y, Material::Wall);
        }
    }

    let mut bodies: Vec<Body> = Vec::new();
    let mut held: Option<usize> = None;

    let mut image = Image::gen_image_color(WIDTH as u16, HEIGHT as u16, BLACK);
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Nearest);

    let mut brush = Material::Sand;
    let mut radius: i32 = 3;
    let mut show_speed = false;

    let mut paused = false;
    let mut sim_speed: f32 = 1.0;
    let mut accumulator: f32 = 0.0;
    let mut step_ms: f32 = 0.0;

    let brush_help = Material::PLACEABLE
        .iter()
        .enumerate()
        .map(|(i, m)| format!("{} {}", i + 1, m.name()))
        .collect::<Vec<_>>()
        .join("  ");

    loop {
        // ----- input -----
        for (i, key) in NUMBER_KEYS.iter().enumerate() {
            if is_key_pressed(*key) {
                if let Some(&material) = Material::PLACEABLE.get(i) {
                    brush = material;
                }
            }
        }
        if is_key_pressed(KeyCode::V) { show_speed = !show_speed; }
        if is_key_pressed(KeyCode::Space) { paused = !paused; }
        if is_key_pressed(KeyCode::LeftBracket) {
            sim_speed = (sim_speed * 0.5).max(0.125);
        }
        if is_key_pressed(KeyCode::RightBracket) {
            sim_speed = (sim_speed * 2.0).min(4.0);
        }

        let (mx, my) = mouse_position();
        let cursor = (mx / SCALE, my / SCALE);
        let gx = cursor.0 as i32;
        let gy = cursor.1 as i32;

        if is_key_pressed(KeyCode::G) {
            world.add_source(Box::new(GravityWell {
                x: cursor.0,
                y: cursor.1,
                strength: 40.0,
            }));
        }
        if is_key_pressed(KeyCode::C) {
            world.clear_sources(); // zero-g
        }
        if is_key_pressed(KeyCode::U) {
            world.add_source(Box::new(UniformGravity {
                gx: 0.0,
                gy: STANDARD_GRAVITY,
            }));
        }

        // Containers: spawn, grab/drop, rotate.
        if is_key_pressed(KeyCode::B) {
            spawn_body(&mut world, &mut bodies, Shape::cup(10.0, 10.0, 26, 3.0), cursor);
        }
        if is_key_pressed(KeyCode::N) {
            spawn_body(&mut world, &mut bodies, Shape::cup(14.0, 3.0, 26, 3.0), cursor);
        }
        if is_key_pressed(KeyCode::F) {
            if let Some(i) = held.take() {
                bodies[i].grab = None; // drop it
            } else if let Some(i) = bodies.iter().position(|b| b.hit_test(cursor)) {
                let local = bodies[i].to_local(cursor);
                bodies[i].grab = Some(local);
                held = Some(i);
            }
        }
        let rotate = (is_key_down(KeyCode::E) as i32 - is_key_down(KeyCode::Q) as i32) as f32;

        if is_mouse_button_down(MouseButton::Left) {
            world.paint(gx, gy, radius, brush);
        }
        if is_mouse_button_down(MouseButton::Right) {
            world.paint(gx, gy, radius, Material::Empty);
        }

        let (_, wheel_y) = mouse_wheel();
        if wheel_y > 0.0 { radius = (radius + 1).min(20); }
        if wheel_y < 0.0 { radius = (radius - 1).max(1); }

        // ----- simulate (fixed timestep) -----
        let steps_to_run = if paused {
            accumulator = 0.0;
            if is_key_pressed(KeyCode::Period) { 1 } else { 0 }
        } else {
            accumulator += get_frame_time().min(0.25) * sim_speed;
            let mut n = 0;
            while accumulator >= SIM_DT && n < MAX_STEPS_PER_FRAME {
                accumulator -= SIM_DT;
                n += 1;
            }
            if accumulator >= SIM_DT {
                accumulator = 0.0;
            }
            n
        };

        for _ in 0..steps_to_run {
            update_bodies(&mut world, &mut bodies, held, cursor, rotate);
            let start = Instant::now();
            world.step();
            step_ms = start.elapsed().as_secs_f32() * 1000.0;
        }

        // ----- draw -----
        world.write_pixels(&mut image.bytes, show_speed);
        texture.update(&image);

        clear_background(BLACK);
        draw_texture_ex(
            &texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(WIDTH as f32 * SCALE, HEIGHT as f32 * SCALE)),
                ..Default::default()
            },
        );

        for (wx, wy) in world.focus_points() {
            draw_circle_lines(wx * SCALE, wy * SCALE, 6.0, 2.0, YELLOW);
        }

        draw_text(
            &format!(
                "{} fps | step {:.2} ms | sim x{:.2} {}",
                get_fps(),
                step_ms,
                sim_speed,
                if paused { "PAUSED" } else { "" }
            ),
            10.0,
            20.0,
            20.0,
            WHITE,
        );
        draw_text(
            &format!("brush: {} (size {}) | {}", brush.name(), radius, brush_help),
            10.0,
            42.0,
            20.0,
            WHITE,
        );
        draw_text(
            "B tumbler | N cone | F grab/drop | Q/E rotate | G well | C zero-g | U gravity | V speed | space pause",
            10.0,
            64.0,
            20.0,
            LIGHTGRAY,
        );

        next_frame().await;
    }
}