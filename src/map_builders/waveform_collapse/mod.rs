use std::cmp::max;
use super::{MetaMapBuilder, BuilderMap, Map, TileType};
use rltk::RandomNumberGenerator;
mod common;
use common::*;
mod constraints;
use constraints::*;
mod solver;
use solver::*;

/// Provides a map builder using the Wave Function Collapse algorithm.
pub struct WaveformCollapseBuilder {}

impl MetaMapBuilder for WaveformCollapseBuilder {
    fn build_map(&mut self, rng: &mut rltk::RandomNumberGenerator, build_data : &mut BuilderMap) {
        if !self.build(rng, build_data) {
            //This will create a map of ~99% walls, causing it to be rejected
            eprintln!("Scrapped WFC map, restarting build process");
            return;
        }
    }
}

impl WaveformCollapseBuilder {
    /// Constructor for waveform collapse.
    #[allow(dead_code)]
    pub fn new() -> Box<WaveformCollapseBuilder> {
        Box::new(WaveformCollapseBuilder{})
    }

    /// Returns true if generated under 10,000 tries, otherwise return false so that the caller of the function can generate a new map
    fn build(&mut self, rng : &mut RandomNumberGenerator, build_data : &mut BuilderMap) -> bool {
        const CHUNK_SIZE :i32 = 8;
        let mut tries:i32 = 0;
        let max_tries:i32 = 10000;
        build_data.take_snapshot();

        let patterns = build_patterns(&build_data.map, CHUNK_SIZE, true, true);
        let constraints = patterns_to_constraints(patterns, CHUNK_SIZE);
        self.render_tile_gallery(&constraints, CHUNK_SIZE, build_data);

        build_data.map = Map::new(build_data.map.depth, build_data.map.width, build_data.map.height);
        loop {
            let mut solver = Solver::new(constraints.clone(), CHUNK_SIZE, &build_data.map);
            while !solver.iteration(&mut build_data.map, rng) {
                build_data.take_snapshot();
            }
            build_data.take_snapshot();
            if solver.possible { break; } // If it has hit an impossible condition, try again
            tries += 1;
            eprintln!("try num {}", tries);
            
            if tries >= max_tries {
                return false;
            }
        }
        build_data.spawn_list.clear();
        
        true
    }

    fn render_tile_gallery(&mut self, constraints: &[MapChunk], chunk_size: i32, build_data : &mut BuilderMap) {
        build_data.map = Map::new(build_data.map.depth, build_data.map.width, build_data.map.height);
        let mut counter = 0;
        let mut x = 1;
        let mut y = 1;
        while counter < constraints.len() {
            render_pattern_to_map(&mut build_data.map, &constraints[counter], chunk_size, x, y);

            x += chunk_size + 1;
            if x + chunk_size > build_data.map.width {
                // Move to the next row
                x = 1;
                y += chunk_size + 1;

                if y + chunk_size > build_data.map.height {
                    // Move to the next page
                    build_data.take_snapshot();
                    build_data.map = Map::new(build_data.map.depth, build_data.map.width, build_data.map.height);

                    x = 1;
                    y = 1;
                }
            }

            counter += 1;
        }
        build_data.take_snapshot();
    }
}