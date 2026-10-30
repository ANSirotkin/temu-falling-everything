mod gravity;
mod material;
mod world;

use std::time::Instant;

use gravity::{GravityWell, UniformGravity, STANDARD_GRAVITY};
use macroquad::prelude::*;
use material::Material;
use world::World;

const WIDTH: i32 = 320;
const HEIGHT: i32 = 180;
const SCALE: f32 = 4.0;

/// The simulation always advances in steps of exactly this much time.
const SIM_DT: f32 = 1.0 / 60.0;
/// If we fall behind, run at most this many steps per frame, then give up.
const MAX_STEPS_PER_FRAME: u32 = 5;

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

#[macroquad::main(window_conf)]
async fn main() {
    let mut world = World::new(WIDTH, HEIGHT);

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

    // Built once from the material table: "1 Sand  2 Water  3 Gas ..."
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
        let gx = (mx / SCALE) as i32;
        let gy = (my / SCALE) as i32;

        if is_key_pressed(KeyCode::G) {
            world.add_source(Box::new(GravityWell {
                x: gx as f32,
                y: gy as f32,
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
                accumulator = 0.0; // too slow to catch up: drop the backlog
            }
            n
        };

        for _ in 0..steps_to_run {
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
            "G well | C zero-g | U gravity | V speed glow | space pause | . step | [ ] sim speed",
            10.0,
            64.0,
            20.0,
            LIGHTGRAY,
        );

        next_frame().await;
    }
}