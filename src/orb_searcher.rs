use std::collections::HashSet;

use eframe::egui::{pos2, Context, Pos2};
use noita_utility_box::noita::{rng::NoitaRng, Seed};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use smart_default::SmartDefault;
use tracing::Instrument;

use crate::util::{persist, Promise};

#[derive(Debug, SmartDefault)]
pub struct OrbSearcher {
    #[default(1024)]
    chunk_size: u32,
    #[default([10, 3])]
    search_range: [i32; 2],
    pub look_for_sampo_instead: bool,
    searched_chunks: HashSet<(i32, i32)>,
    known_orbs: Vec<Pos2>,
    #[default(Promise::Taken)]
    search_task: Promise<Vec<(i32, i32)>>,
}

persist!(OrbSearcher {
    look_for_sampo_instead: bool,
});

impl OrbSearcher {
    pub fn known_orbs(&self) -> &[Pos2] {
        &self.known_orbs
    }

    pub fn searched_chunks(&self) -> usize {
        self.searched_chunks.len()
    }

    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }

    pub fn reset(&mut self) {
        self.known_orbs.clear();
        self.searched_chunks.clear();
        self.search_task = Promise::Taken;
    }

    pub fn is_searching(&self) -> bool {
        !self.search_task.is_taken()
    }

    fn next_chunk(&mut self, pos: Pos2) -> Option<(i32, i32)> {
        let xc = pos.x as i32 / self.chunk_size as i32;
        let yc = pos.y as i32 / self.chunk_size as i32;
        //meh
        for x in xc - self.search_range[0]..=xc + self.search_range[0] {
            for y in yc - self.search_range[1]..=yc + self.search_range[1] {
                if self.searched_chunks.insert((x, y)) {
                    return Some((x, y));
                }
            }
        }
        None
    }

    pub fn poll_search(&mut self, ctx: &Context, seed: Seed, pos: Pos2) {
        if self.search_task.is_taken() {
            if let Some((x, y)) = self.next_chunk(pos) {
                let size = self.chunk_size;
                let x = x * size as i32;
                let y = y * size as i32;
                let ctx = ctx.clone();
                let sampo = self.look_for_sampo_instead;
                self.search_task = Promise::spawn(
                    async move {
                        let orbs = find_orbs(seed.sum(), x, y, size, size, sampo);
                        ctx.request_repaint();
                        orbs
                    }
                    .instrument(tracing::trace_span!("search", %seed, x, y, size)),
                );
            }
        } else if let Some(orbs) = self.search_task.poll_take() {
            self.known_orbs
                .extend(orbs.into_iter().map(|(x, y)| pos2(x as f32, y as f32)));
            return self.poll_search(ctx, seed, pos);
        }
        self.known_orbs.sort_unstable_by_key(|orb| {
            let dir = *orb - pos;
            dir.length_sq() as i32
        });
    }
}

fn find_orbs(
    world_seed: u32,
    x: i32,
    y: i32,
    x_size: u32,
    y_size: u32,
    sampo: bool,
) -> Vec<(i32, i32)> {
    (0..x_size * y_size)
        .into_par_iter()
        .filter_map(|i| {
            let xi = x + (i % x_size) as i32;
            let yi = y + (i / x_size) as i32;

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
