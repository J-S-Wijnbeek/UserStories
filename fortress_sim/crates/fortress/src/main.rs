use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use sim_core::{SimState, JobKind, JobPriority};
use world::{WorldGen, TileKind, Material, DesignationKind};
use gameplay::GameplayState;
use persistence::{save, load};

const TILE_SIZE: usize = 12;
const MAP_W: usize = 64;
const MAP_H: usize = 64;
const WIN_W: usize = 800;
const WIN_H: usize = 600;
const PAN_SPEED: f32 = 4.0;

// Pack RGB bytes into minifb's 0x00RRGGBB u32 format.
fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn draw_rect(buf: &mut Vec<u32>, bw: usize, bh: usize, x: i32, y: i32, w: usize, h: usize, color: u32) {
    for row in 0..h {
        let py = y + row as i32;
        if py < 0 || py >= bh as i32 { continue; }
        for col in 0..w {
            let px = x + col as i32;
            if px < 0 || px >= bw as i32 { continue; }
            buf[py as usize * bw + px as usize] = color;
        }
    }
}

fn draw_circle(buf: &mut Vec<u32>, bw: usize, bh: usize, cx: i32, cy: i32, r: i32, color: u32) {
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 && px < bw as i32 && py < bh as i32 {
                    buf[py as usize * bw + px as usize] = color;
                }
            }
        }
    }
}

fn main() {
    let seed = 42u64;
    let mut map = WorldGen::generate(seed);
    let mut state = SimState::new(seed);
    let mut gameplay = GameplayState::new();

    let mut offset_x: f32 = 0.0;
    let mut offset_y: f32 = 0.0;

    let mut window = Window::new(
        "Fortress Sim",
        WIN_W,
        WIN_H,
        WindowOptions::default(),
    )
    .expect("Failed to create window");

    // ~60 fps cap
    window.set_target_fps(60);

    let mut buf: Vec<u32> = vec![0; WIN_W * WIN_H];

    // Track previous key states for press detection
    let mut prev_f5 = false;
    let mut prev_f9 = false;
    let mut prev_esc = false;
    let mut prev_lmb = false;
    let mut prev_rmb = false;

    while window.is_open() {
        // ---------- Input ----------
        let f5_down  = window.is_key_down(Key::F5);
        let f9_down  = window.is_key_down(Key::F9);
        let esc_down = window.is_key_down(Key::Escape);

        if window.is_key_down(Key::Left)  || window.is_key_down(Key::A) { offset_x += PAN_SPEED; }
        if window.is_key_down(Key::Right) || window.is_key_down(Key::D) { offset_x -= PAN_SPEED; }
        if window.is_key_down(Key::Up)    || window.is_key_down(Key::W) { offset_y += PAN_SPEED; }
        if window.is_key_down(Key::Down)  || window.is_key_down(Key::S) { offset_y -= PAN_SPEED; }

        if f5_down && !prev_f5 {
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

        if f9_down && !prev_f9 {
            if let Ok((_meta, data)) = load("./saves/default") {
                map = data.map;
                state.dwarves = data.dwarves;
                state.items = data.items;
                state.job_queue = data.job_queue;
                gameplay.stockpiles = data.stockpiles;
                println!("Game loaded.");
            }
        }

        if esc_down && !prev_esc {
            break;
        }

        if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Discard) {
            let lmb = window.get_mouse_down(MouseButton::Left);
            let rmb = window.get_mouse_down(MouseButton::Right);

            if lmb && !prev_lmb {
                let tx = ((mx - offset_x) / TILE_SIZE as f32) as i32;
                let ty = ((my - offset_y) / TILE_SIZE as f32) as i32;
                if tx >= 0 && ty >= 0 {
                    let (tx, ty) = (tx as u32, ty as u32);
                    if let Some(tile) = map.get(tx, ty) {
                        if matches!(tile.kind, TileKind::Solid | TileKind::Wall)
                            && tile.designated != Some(DesignationKind::Mine)
                        {
                            if let Some(t) = map.get_mut(tx, ty) {
                                t.designated = Some(DesignationKind::Mine);
                            }
                            state.job_queue.push(JobKind::Dig { pos: (tx, ty) }, JobPriority::Basic);
                        }
                    }
                }
            }

            if rmb && !prev_rmb {
                let tx = ((mx - offset_x) / TILE_SIZE as f32) as i32;
                let ty = ((my - offset_y) / TILE_SIZE as f32) as i32;
                if tx >= 0 && ty >= 0 {
                    if let Some(tile) = map.get(tx as u32, ty as u32) {
                        println!(
                            "Tile ({}, {}): {:?}, material: {:?}, designated: {:?}",
                            tx, ty, tile.kind, tile.material, tile.designated
                        );
                    }
                }
            }

            prev_lmb = lmb;
            prev_rmb = rmb;
        }

        prev_f5  = f5_down;
        prev_f9  = f9_down;
        prev_esc = esc_down;

        // ---------- Simulate ----------
        for _ in 0..10 {
            state.step(&mut map);
        }

        // ---------- Render ----------
        buf.fill(0x00_00_00_00); // black

        let start_x = ((-offset_x) / TILE_SIZE as f32) as i32;
        let start_y = ((-offset_y) / TILE_SIZE as f32) as i32;
        let visible_w = WIN_W / TILE_SIZE + 2;
        let visible_h = WIN_H / TILE_SIZE + 2;

        for ty in start_y..(start_y + visible_h as i32 + 1) {
            for tx in start_x..(start_x + visible_w as i32 + 1) {
                if tx < 0 || ty < 0 || tx >= MAP_W as i32 || ty >= MAP_H as i32 { continue; }
                let (tx, ty) = (tx as u32, ty as u32);
                if let Some(tile) = map.get(tx, ty) {
                    let sx = (tx as f32 * TILE_SIZE as f32 + offset_x) as i32;
                    let sy = (ty as f32 * TILE_SIZE as f32 + offset_y) as i32;
                    let tile_px = TILE_SIZE - 1;

                    let color = match tile.kind {
                        TileKind::Solid => rgb(80, 80, 80),
                        TileKind::Wall  => rgb(120, 120, 120),
                        TileKind::Floor => rgb(200, 200, 200),
                        TileKind::Open  => rgb(0, 0, 0),
                    };
                    draw_rect(&mut buf, WIN_W, WIN_H, sx, sy, tile_px, tile_px, color);

                    if tile.designated == Some(DesignationKind::Mine) {
                        // Orange tint overlay (blend manually)
                        for row in 0..tile_px {
                            let py = sy + row as i32;
                            if py < 0 || py >= WIN_H as i32 { continue; }
                            for col in 0..tile_px {
                                let px = sx + col as i32;
                                if px < 0 || px >= WIN_W as i32 { continue; }
                                let idx = py as usize * WIN_W + px as usize;
                                let base = buf[idx];
                                let br = ((base >> 16) & 0xff) as u16;
                                let bg = ((base >> 8)  & 0xff) as u16;
                                let bb = (base & 0xff) as u16;
                                // blend with orange (200,100,0) at 40% alpha
                                let r = ((br * 60 + 200 * 40) / 100).min(255) as u8;
                                let g = ((bg * 60 + 100 * 40) / 100).min(255) as u8;
                                let b = ((bb * 60) / 100).min(255) as u8;
                                buf[idx] = rgb(r, g, b);
                            }
                        }
                    }
                }
            }
        }

        // Items
        for item in &state.items {
            let cx = (item.pos.0 as f32 * TILE_SIZE as f32 + offset_x + TILE_SIZE as f32 / 2.0) as i32;
            let cy = (item.pos.1 as f32 * TILE_SIZE as f32 + offset_y + TILE_SIZE as f32 / 2.0) as i32;
            let color = match item.material {
                Material::Stone => rgb(150, 150, 150),
                Material::Wood  => rgb(139, 90, 43),
                Material::Food  => rgb(0, 200, 0),
                Material::Ore   => rgb(212, 175, 55),
                Material::None  => rgb(255, 255, 255),
            };
            draw_circle(&mut buf, WIN_W, WIN_H, cx, cy, 2, color);
        }

        // Dwarves as yellow '@' squares (simple representation)
        for dwarf in &state.dwarves {
            let sx = (dwarf.pos.0 as f32 * TILE_SIZE as f32 + offset_x) as i32;
            let sy = (dwarf.pos.1 as f32 * TILE_SIZE as f32 + offset_y) as i32;
            draw_rect(&mut buf, WIN_W, WIN_H, sx + 2, sy + 2, TILE_SIZE - 4, TILE_SIZE - 4, rgb(255, 220, 0));
        }

        // HUD bar (dark background strip)
        draw_rect(&mut buf, WIN_W, WIN_H, 0, 0, WIN_W, 20, rgb(20, 20, 20));
        draw_rect(&mut buf, WIN_W, WIN_H, 0, 20, WIN_W, 14, rgb(10, 10, 10));

        // Print HUD to terminal each second (~600 ticks)
        if state.time.tick % 600 == 0 {
            println!(
                "[HUD] Tick: {}  Day: {}  Dwarves: {}  Jobs: {}",
                state.time.tick,
                state.time.day(),
                state.dwarves.len(),
                state.job_queue.jobs.len(),
            );
        }

        window
            .update_with_buffer(&buf, WIN_W, WIN_H)
            .expect("Failed to update window");
    }
}

