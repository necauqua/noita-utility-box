use std::{cmp::Ordering, collections::HashSet};

use eframe::egui::{Context, Pos2, pos2};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use smart_default::SmartDefault;
use tracing::Instrument;

use crate::util::{Promise, persist};
use noita_engine_reader::{Seed, rng::NoitaRng};

pub const CHUNK_SIZE: i32 = 512;

#[derive(Debug, Clone, Copy)]
pub enum OrbSource {
    Room,
    Chest,
}

#[derive(Debug, Clone, Copy)]
pub struct Orb {
    #[allow(unused)]
    pub id: u32,
    pub pos: Pos2,
    pub source: OrbSource,
    pub corrupted: bool,
}

impl Orb {
    pub fn parallel_world_id(id: u32, parallel_world: i32) -> u32 {
        id + match parallel_world.cmp(&0) {
            Ordering::Equal => 0,
            Ordering::Less => 128,
            Ordering::Greater => 256,
        }
    }
}

#[derive(Debug, SmartDefault)]
pub struct OrbSearcher {
    #[default(5)]
    search_range: i32,
    #[default(Promise::Taken)]
    search_task: Promise<Vec<Orb>>,

    // Which PW the rooms are listed
    #[default(Option::None)]
    current_rooms_world: Option<i32>,
    known_rooms: Vec<Orb>,

    searched_chunks: HashSet<(i32, i32)>,
    known_orbs: Vec<Orb>,
    pub look_for_sampo_instead: bool,
}

persist!(OrbSearcher {
    look_for_sampo_instead: bool,
});

impl OrbSearcher {
    pub fn known_orbs(&self) -> &[Orb] {
        &self.known_orbs
    }

    pub fn known_rooms(&self) -> &[Orb] {
        &self.known_rooms
    }

    pub fn searched_chunks(&self) -> usize {
        self.searched_chunks.len()
    }

    pub fn reset(&mut self) {
        self.known_orbs.clear();
        self.searched_chunks.clear();
        self.search_task = Promise::Taken;
    }

    pub fn is_searching(&self) -> bool {
        !self.search_task.is_taken()
    }

    /// Return the next chunk to check in a spiral pattern around the player
    fn next_chunk(&mut self, pos: Pos2) -> Option<(i32, i32)> {
        let mut x = pos.x as i32 / CHUNK_SIZE;
        let mut y = pos.y as i32 / CHUNK_SIZE;

        if self.searched_chunks.insert((x, y)) {
            return Some((x, y));
        }

        // Tracking the steps of the marching
        let (mut steps, mut step) = (1, 0);
        // Direction of the next step
        let (mut x_dir, mut y_dir) = (1, 0);
        let mut rotations = 0;

        // Search a square centered on the current chunk
        for _ in 0..(1 + 2 * self.search_range).pow(2) {
            // Step once
            (x, y) = (x + x_dir, y + y_dir);
            step += 1;

            // If we didn't already checked this chunk, stop there
            if self.searched_chunks.insert((x, y)) {
                return Some((x, y));
            }

            if step == steps {
                // Rotate when at the limit
                (x_dir, y_dir) = (-y_dir, x_dir);
                rotations += 1;
                step = 0;

                // Every two direction changes, increase the number of steps
                if rotations % 2 == 0 {
                    steps += 1;
                }
            }
        }

        None
    }

    pub fn poll_search(&mut self, ctx: &Context, seed: Seed, pos: Pos2) {
        // First update the orb rooms of the current PW if necessary
        if self.known_rooms.is_empty()
            || self.current_rooms_world.unwrap_or(0) != parallel_world(seed.ng_count, &pos)
        {
            self.known_rooms = list_orb_rooms(
                seed.world_seed,
                seed.ng_count,
                parallel_world(seed.ng_count, &pos),
            );
            self.current_rooms_world = Some(parallel_world(seed.ng_count, &pos));
        }

        if self.is_searching()
            && let Some(orbs) = self.search_task.poll_take()
        {
            self.known_orbs.extend(orbs);
            return self.poll_search(ctx, seed, pos);
        }

        if let Some((chunk_x, chunk_y)) = self.next_chunk(pos) {
            let (x, y) = (chunk_x * CHUNK_SIZE, chunk_y * CHUNK_SIZE);
            let ctx = ctx.clone();
            let sampo = self.look_for_sampo_instead;
            let parallel_world = parallel_world(seed.ng_count, &pos);
            self.search_task = Promise::spawn(
                async move {
                    // Look for chests in the chunk matching the search parameters (orb/sampo)
                    let orbs: Vec<Orb> = find_chest_orbs(seed.sum(), x, y, sampo)
                        .into_iter()
                        .map(|(x, y)| Orb {
                            id: Orb::parallel_world_id(11, parallel_world),
                            pos: pos2(x as f32, y as f32),
                            source: OrbSource::Chest,
                            corrupted: false,
                        })
                        .collect();

                    ctx.request_repaint();
                    orbs
                }
                .instrument(tracing::trace_span!("search", %seed, x, y)),
            );
        }
    }
}

/// Compute the parallel_world of the current position depending on if we are in NG+ or not.
fn parallel_world(ng_count: u32, pos: &Pos2) -> i32 {
    if ng_count == 0 {
        (pos.x / CHUNK_SIZE as f32 - 35.0) as i32 / 70
    } else {
        (pos.x / CHUNK_SIZE as f32 - 32.0) as i32 / 64
    }
}

/// Find all chests producing a Greater Chest Orb or Sampo in the chunk given
fn find_chest_orbs(world_seed: u32, x: i32, y: i32, sampo: bool) -> Vec<(i32, i32)> {
    (0..CHUNK_SIZE * CHUNK_SIZE)
        .into_par_iter()
        .filter_map(|i| {
            let xi = x + (i % CHUNK_SIZE);
            let yi = y + (i / CHUNK_SIZE);

            let mut rng = NoitaRng::from_pos(world_seed, xi as f64, yi as f64);

            if (rng.random() * 100001.0) as u32 == 100000
                && sampo ^ ((rng.random() * 1001.0) as u32 == 999)
            {
                tracing::debug!(x = xi, y = yi, "orb found");
                return Some((xi, yi));
            }
            None
        })
        .collect()
}

/// In the main NG world the rooms are fixed.
const KNOWN_ORB_ROOMS: &[(u32, (i32, i32))] = &[
    (0, (1, -3)),   // Altar => Sea of Lava
    (1, (19, -3)),  // Pyramid => Earthquake
    (2, (-20, 5)),  // Frozen Vault => Tentacles
    (3, (6, 3)),    // Lava Lake => Nuke
    (4, (19, 5)),   // Sandcaves => Necromancy
    (5, (-9, 7)),   // Magical Temple => Holy Bomb
    (6, (-8, 19)),  // Luki Lair => Spiral Shot
    (7, (8, 1)),    // Lava Bridge => Thundercloud
    (8, (-1, 31)),  // Hell => Fireworks
    (9, (-18, 28)), // Snowy Chasm => Summon Deercoy
    (10, (20, 31)), // Wizard's Den => Cement
];

/// Find the orb rooms for the current world_seed, ng_count, parallel_world.
fn list_orb_rooms(world_seed: u32, ng_count: u32, parallel_world: i32) -> Vec<Orb> {
    // First check for known orbs for NG
    let rooms = if ng_count == 0 {
        KNOWN_ORB_ROOMS
            .iter()
            .filter(|(id, _)| *id != 3 || parallel_world == 0) // Lava Lake orb doesn't spawn in PW
            .cloned()
            .collect::<Vec<_>>()
    } else {
        list_orb_rooms_ng_plus(world_seed, ng_count)
    };

    rooms
        .iter()
        .map(|(id, (x, y))| {
            (
                // ID of PW orbs are different
                Orb::parallel_world_id(*id, parallel_world),
                // The offset of X chunks to the room generation
                (x + parallel_world * if ng_count == 0 { 70 } else { 64 }, y),
            )
        })
        .map(|(id, (x, y))| Orb {
            id,
            pos: pos2(
                (x as f32 + 0.5) * CHUNK_SIZE as f32,
                (*y as f32 + 0.75) * CHUNK_SIZE as f32,
            ), // FIXME: Known orbs are not all orb rooms, therefore some are offset
            source: OrbSource::Room,
            corrupted: parallel_world != 0,
        })
        .collect()
}

/// Find the orb rooms for the current world_seed, ng_count, parallel_world.
fn list_orb_rooms_ng_plus(world_seed: u32, ng_count: u32) -> Vec<(u32, (i32, i32))> {
    let mut rooms: Vec<(u32, (i32, i32))> = Vec::new();

    // This function is lifted from kaliuresis/noa (noita-orb-atlas)
    // Source: https://github.com/kaliuresis/noa > orbs.js#L163
    let mut rng = NoitaRng::from_pos(world_seed + ng_count, 4573.0, 4621.0);

    // Shorthand to jump the RNG state
    fn pain_cave(rng: &mut NoitaRng, length: i32) {
        for i in 1..=length {
            #[allow(clippy::needless_if)]
            if i > 4 {
                rng.skip(1);
            }

            if i > 3 {
                rng.skip(1);
            }

            if i > 6 {
                rng.skip(4);
            }
        }
    }

    fn paint_biome_area_split(rng: &mut NoitaRng, x: i32, _y: i32, w: i32, _h: i32, buffer: i32) {
        let extra_width = rng.in_range(0, buffer);
        let x = x - extra_width;
        let w = w + extra_width + rng.in_range(0, buffer);

        rng.skip(1);
        for _ in x..x + w {
            rng.skip(1);
        }
    }

    // NG+3,6,... biomes
    if ng_count % 3 == 0 {
        rng.skip(12);
    }

    // Roll to swap biomes
    rng.skip(6);

    // Biomes suffs
    // NOTE: Compared to the JS code a lot of stuffs are omitted as they have no side effects
    for _ in 0..4 {
        if rng.in_range(0, 100) < 65 {
            let length = rng.in_range(4, 50);
            pain_cave(&mut rng, length);
        }
    }
    for _ in 0..4 {
        if rng.in_range(0, 100) < 65 {
            rng.skip(1);
            let length = rng.in_range(5, 50);
            pain_cave(&mut rng, length);
        }
    }

    rng.skip(6);

    paint_biome_area_split(&mut rng, 28, 20, 7, 6, 3);
    paint_biome_area_split(&mut rng, 28, 27, 7, 4, 4);
    paint_biome_area_split(&mut rng, 28, 29, 7, 5, 4);

    rng.skip(2);

    if ng_count % 5 == 0 {
        rng.skip(6);
    }

    // NOTE: The magic number are from the original JS code.
    // All generated numbers are offset by (32, 14) chunks in the code, and then later edited.
    // Therefore here the math is pre-computed and it match the online map ranges.

    // WARNING: I'm really not sure about the IDs in NG+, the JS code was mixing them and there
    // was 2 different mixing commented...

    // Altar => Sea of Lava
    rooms.push((0, KNOWN_ORB_ROOMS[1].1)); // Altar in NG+ has ID 1?
    // Pyramid => Earthquake
    rooms.push((1, KNOWN_ORB_ROOMS[0].1)); // Pyramd in NG+ has ID 0?

    // Frozen Vault => Tentacles
    // > x = Random( 0, 5 ) + 10; y = Random( 0, 2 ) + 18;
    rooms.push((2, (rng.in_range(0, 5) - 22, rng.in_range(0, 2) + 4)));

    // Sandcaves => Necromancy
    // > x = Random( 0, 5 ) + 49; y = Random( 0, 3 ) + 17;
    rooms.push((4, (rng.in_range(0, 5) + 17, rng.in_range(0, 3) + 3)));

    // Hell => Fireworks
    // > x = Random( 0, 9 ) + 27, y = Random( 0, 2 ) + 44
    // > if( ng == 3 || ng >= 25 ) { y = 47 }
    let mut hell_orb = (8, (rng.in_range(0, 9) - 5, rng.in_range(0, 2) + 30));
    if ng_count == 3 || ng_count >= 25 {
        hell_orb.1.1 = 33
    }
    rooms.push(hell_orb);

    // Snowy Chasm => Summon Deercoy
    // > x = Random( 0, 6 ) + 12; y = Random( 0, 3 ) + 40;
    rooms.push((9, (rng.in_range(0, 6) - 20, rng.in_range(0, 3) + 26)));

    // Wizard's Den => Cement
    // > x = Random( 0, 4 ) + 51; y = Random( 0, 5 ) + 41;
    rooms.push((10, (rng.in_range(0, 4) + 19, rng.in_range(0, 5) + 27)));

    // Lava Lake => Nuke
    // > x = Random( 0, 5 ) + 58; y = Random( 0, 5 ) + 34;
    rooms.push((3, (rng.in_range(0, 5) + 26, rng.in_range(0, 5) + 20)));

    // Magical Temple => Holy Bomb
    // > x = Random( 0, 9 ) + 40; y = Random( 0, 11 ) + 21;
    rooms.push((5, (rng.in_range(0, 9) + 8, rng.in_range(0, 11) + 7)));

    // Luki Lair => Spiral Shot
    // > x = Random( 0, 7 ) + 17; y = Random( 0, 8 ) + 21;
    rooms.push((6, (rng.in_range(0, 7) - 15, rng.in_range(0, 8) + 7)));

    // Lake Orb => Thundercloud
    // > x = Random( 0, 7 ) + 1; y = Random( 0, 9 ) + 24;
    rooms.push((7, (rng.in_range(0, 7) - 31, rng.in_range(0, 9) + 10)));

    rooms
}
