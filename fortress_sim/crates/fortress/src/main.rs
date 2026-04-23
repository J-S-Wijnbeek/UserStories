use macroquad::prelude::*;
use sim_core::{SimState, JobKind, JobPriority};
use world::{WorldGen, TileKind, Material, DesignationKind};
use gameplay::GameplayState;
use persistence::{save, load};

const TILE_SIZE: f32 = 12.0;
const PAN_SPEED: f32 = 4.0;

#[macroquad::main("Fortress Sim")]
async fn main() {
    let seed = 42u64;
    let mut map = WorldGen::generate(seed);
    let mut state = SimState::new(seed);
    let mut gameplay = GameplayState::new();
    let mut offset_x: f32 = 0.0;
    let mut offset_y: f32 = 0.0;

    loop {
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            offset_x += PAN_SPEED;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            offset_x -= PAN_SPEED;
        }
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            offset_y += PAN_SPEED;
        }
        if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
            offset_y -= PAN_SPEED;
        }

        if is_key_pressed(KeyCode::F5) {
            let _ = save(
                "./saves/default",
                seed,
                &state.time,
                &map,
                &state.dwarves,
                &state.items,
                &state.job_queue,
                &gameplay.stockpiles,
            );
            println!("Game saved.");
        }

        if is_key_pressed(KeyCode::F9) {
            if let Ok((_meta, data)) = load("./saves/default") {
                map = data.map;
                state.dwarves = data.dwarves;
                state.items = data.items;
                state.job_queue = data.job_queue;
                gameplay.stockpiles = data.stockpiles;
                println!("Game loaded.");
            }
        }

        if is_key_pressed(KeyCode::Escape) {
            std::process::exit(0);
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            let tx = ((mx - offset_x) / TILE_SIZE) as i32;
            let ty = ((my - offset_y) / TILE_SIZE) as i32;
            if tx >= 0 && ty >= 0 {
                let tx = tx as u32;
                let ty = ty as u32;
                if let Some(tile) = map.get(tx, ty) {
                    if matches!(tile.kind, TileKind::Solid | TileKind::Wall) {
                        let already_designated = tile.designated == Some(DesignationKind::Mine);
                        if !already_designated {
                            if let Some(t) = map.get_mut(tx, ty) {
                                t.designated = Some(DesignationKind::Mine);
                            }
                            state.job_queue.push(JobKind::Dig { pos: (tx, ty) }, JobPriority::Basic);
                        }
                    }
                }
            }
        }

        if is_mouse_button_pressed(MouseButton::Right) {
            let (mx, my) = mouse_position();
            let tx = ((mx - offset_x) / TILE_SIZE) as i32;
            let ty = ((my - offset_y) / TILE_SIZE) as i32;
            if tx >= 0 && ty >= 0 {
                if let Some(tile) = map.get(tx as u32, ty as u32) {
                    println!("Tile ({}, {}): {:?}, material: {:?}, designated: {:?}", tx, ty, tile.kind, tile.material, tile.designated);
                }
            }
        }

        for _ in 0..10 {
            state.step(&mut map);
        }

        clear_background(BLACK);

        let visible_w = (screen_width() / TILE_SIZE) as u32 + 2;
        let visible_h = (screen_height() / TILE_SIZE) as u32 + 2;
        let start_x = ((-offset_x) / TILE_SIZE) as i32;
        let start_y = ((-offset_y) / TILE_SIZE) as i32;

        for ty in start_y..(start_y + visible_h as i32 + 1) {
            for tx in start_x..(start_x + visible_w as i32 + 1) {
                if tx < 0 || ty < 0 || tx >= 64 || ty >= 64 {
                    continue;
                }
                let tx = tx as u32;
                let ty = ty as u32;
                if let Some(tile) = map.get(tx, ty) {
                    let sx = tx as f32 * TILE_SIZE + offset_x;
                    let sy = ty as f32 * TILE_SIZE + offset_y;

                    let color = match tile.kind {
                        TileKind::Solid => Color::from_rgba(80, 80, 80, 255),
                        TileKind::Wall => Color::from_rgba(120, 120, 120, 255),
                        TileKind::Floor => Color::from_rgba(200, 200, 200, 255),
                        TileKind::Open => Color::from_rgba(0, 0, 0, 255),
                    };
                    draw_rectangle(sx, sy, TILE_SIZE - 1.0, TILE_SIZE - 1.0, color);

                    if tile.designated == Some(DesignationKind::Mine) {
                        draw_rectangle(sx, sy, TILE_SIZE - 1.0, TILE_SIZE - 1.0, Color::from_rgba(200, 100, 0, 100));
                    }
                }
            }
        }

        for item in &state.items {
            let sx = item.pos.0 as f32 * TILE_SIZE + offset_x + TILE_SIZE / 2.0;
            let sy = item.pos.1 as f32 * TILE_SIZE + offset_y + TILE_SIZE / 2.0;
            let color = match item.material {
                Material::Stone => GRAY,
                Material::Wood => BROWN,
                Material::Food => GREEN,
                Material::Ore => GOLD,
                Material::None => WHITE,
            };
            draw_circle(sx, sy, 2.0, color);
        }

        for dwarf in &state.dwarves {
            let sx = dwarf.pos.0 as f32 * TILE_SIZE + offset_x + 1.0;
            let sy = dwarf.pos.1 as f32 * TILE_SIZE + offset_y + 10.0;
            draw_text("@", sx, sy, TILE_SIZE, YELLOW);
        }

        let hud = format!(
            "Tick: {}  Day: {}  Dwarves: {}  Jobs: {}",
            state.time.tick,
            state.time.day(),
            state.dwarves.len(),
            state.job_queue.jobs.len(),
        );
        draw_text(&hud, 5.0, 15.0, 16.0, WHITE);
        draw_text("LClick:Mine  RClick:Info  F5:Save  F9:Load  ESC:Quit  WASD/Arrows:Pan", 5.0, 30.0, 12.0, LIGHTGRAY);

        next_frame().await;
    }
}
