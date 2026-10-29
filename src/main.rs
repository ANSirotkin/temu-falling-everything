mod gravity;
mod material;
mod world;

use gravity::{GravityWell, UniformGravity, STANDARD_GRAVITY};
use macroquad::prelude::*;
use material::Material;
use world::World;

const WIDTH: i32 = 320;
const HEIGHT: i32 = 180;
const SCALE: f32 = 4.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "Temu Falling Everything".to_owned(),
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

    loop {
        // ----- input -----
        if is_key_pressed(KeyCode::Key1) { brush = Material::Sand; }
        if is_key_pressed(KeyCode::Key2) { brush = Material::Water; }
        if is_key_pressed(KeyCode::Key3) { brush = Material::Gas; }
        if is_key_pressed(KeyCode::Key4) { brush = Material::Wall; }
        if is_key_pressed(KeyCode::Key5) { brush = Material::Glass; }
        if is_key_pressed(KeyCode::V) { show_speed = !show_speed; }

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

        // ----- simulate -----
        world.step();

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
                "{} fps | 1 sand 2 water 3 gas 4 wall 5 glass | G well, C zero-g, U gravity, V speed | wheel: brush {}",
                get_fps(),
                radius
            ),
            10.0,
            20.0,
            20.0,
            WHITE,
        );

        next_frame().await;
    }
}