use rltk::{RGB, Rltk, BaseMap, Algorithm2D, Point, SmallVec};
use specs::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashSet;

pub mod tiletype;
pub mod map_indexing_system;
pub mod themes;
pub mod dungeon;

pub use tiletype::{TileType, tile_walkable, tile_opaque};
use crate::map::dungeon::MasterDungeonMap;
use crate::map::tiletype::tile_cost;
use crate::{Position, Viewshed};
use crate::map_builders::level_builder;

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Map {
    pub tiles : Vec<TileType>,
    pub width : i32,
    pub height : i32,
    pub revealed_tiles : Vec<bool>,
    pub visible_tiles : Vec<bool>,
    pub blocked : Vec<bool>,
    pub depth : i32,
    pub bloodstains : HashSet<usize>,
    pub view_blocked : HashSet<usize>,
    pub name : String,
    pub outdoors : bool,
    pub light : Vec<rltk::RGB>,

    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub tile_content : Vec<Vec<Entity>>
}

impl Map {
    pub fn xy_idx(&self, x: i32, y: i32) -> usize {
        (y as usize * self.width as usize) + x as usize
    }

    fn is_exit_valid(&self, x:i32, y:i32) -> bool {
        if x < 1 || x > self.width-1 || y < 1 || y > self.height-1 { return false; }
        let idx = self.xy_idx(x, y);
        !self.blocked[idx]
    }

    pub fn populate_blocked(&mut self) {
        for tile in &mut self.blocked {
            *tile = false; // Reset blocked state
        }
        for (i,tile) in self.tiles.iter_mut().enumerate() {
            self.blocked[i] = !tile_walkable(*tile);
        }
    }

    pub fn clear_content_index(&mut self) {
        for content in self.tile_content.iter_mut() {
            content.clear();
        }
    }

    /// Generates an empty map, consisting entirely of solid walls
    pub fn new<S : ToString>(new_depth : i32, width: i32, height: i32, name: S) -> Map {
        let map_tile_count = (width*height) as usize;
        Map{
            tiles : vec![TileType::Wall; map_tile_count],
            width,
            height,
            revealed_tiles : vec![false; map_tile_count],
            visible_tiles : vec![false; map_tile_count],
            blocked : vec![false; map_tile_count],
            tile_content : vec![Vec::new(); map_tile_count],
            depth: new_depth,
            bloodstains: HashSet::new(),
            view_blocked : HashSet::new(),
            name : name.to_string(),
            outdoors : true,
            light: vec![rltk::RGB::from_f32(0.0, 0.0, 0.0); map_tile_count]
        }
    }
}

impl BaseMap for Map {
    fn is_opaque(&self, idx:usize) -> bool {
        let idx_u = idx as usize;
        if idx_u > 0 && idx_u < self.tiles.len() {
            tile_opaque(self.tiles[idx_u]) || self.view_blocked.contains(&idx_u)
        } else {
            true
        }
    }
    
    fn get_pathing_distance(&self, idx1:usize, idx2:usize) -> f32 {
        let w = self.width as usize;
        let p1 = Point::new(idx1 % w, idx1 / w);
        let p2 = Point::new(idx2 % w, idx2 / w);
        rltk::DistanceAlg::Pythagoras.distance2d(p1, p2)
    }

    // Multiplied diagonals by 1.5 to encourage more natural looking movement
    fn get_available_exits(&self, idx:usize) -> rltk::SmallVec<[(usize, f32); 10]> {
        const DIAGONAL_COST : f32 = 1.5;
        let mut exits = rltk::SmallVec::new();
        let x = idx as i32 % self.width;
        let y = idx as i32 / self.width;
        let tt = self.tiles[idx as usize];
        let w = self.width  as usize;

        // Cardinal directions
        if self.is_exit_valid(x-1, y) { exits.push((idx-1, tile_cost(tt))) };
        if self.is_exit_valid(x+1, y) { exits.push((idx+1, tile_cost(tt))) };
        if self.is_exit_valid(x, y-1) { exits.push((idx-w, tile_cost(tt))) };
        if self.is_exit_valid(x, y+1) { exits.push((idx+w, tile_cost(tt))) };

        // Diagonals
        if self.is_exit_valid(x-1, y-1) { exits.push(((idx-w)-1, tile_cost(tt) * DIAGONAL_COST)); }
        if self.is_exit_valid(x+1, y-1) { exits.push(((idx-w)+1, tile_cost(tt) * DIAGONAL_COST)); }
        if self.is_exit_valid(x-1, y+1) { exits.push(((idx+w)-1, tile_cost(tt) * DIAGONAL_COST)); }
        if self.is_exit_valid(x+1, y+1) { exits.push(((idx+w)+1, tile_cost(tt) * DIAGONAL_COST)); }

        exits
    }
}

impl Algorithm2D for Map {
    fn dimensions(&self) -> Point {
        Point::new(self.width, self.height)
    }
}

fn is_revealed_and_wall(map: &Map, x: i32, y: i32) -> bool {
    let idx = map.xy_idx(x, y);
    map.tiles[idx] == TileType::Wall && map.revealed_tiles[idx]
}

fn wall_glyph(map : &Map, x: i32, y:i32) -> rltk::FontCharType {
    if x < 1 || x > map.width-2 || y < 1 || y > map.height-2 as i32 { return 35; }
    let mut mask : u8 = 0;

    if is_revealed_and_wall(map, x, y - 1) { mask +=1; }
    if is_revealed_and_wall(map, x, y + 1) { mask +=2; }
    if is_revealed_and_wall(map, x - 1, y) { mask +=4; }
    if is_revealed_and_wall(map, x + 1, y) { mask +=8; }

    match mask {
        0 => { 9 } // Pillar because we can't see neighbors
        1 => { 186 } // Wall only to the north
        2 => { 186 } // Wall only to the south
        3 => { 186 } // Wall to the north and south
        4 => { 205 } // Wall only to the west
        5 => { 188 } // Wall to the north and west
        6 => { 187 } // Wall to the south and west
        7 => { 185 } // Wall to the north, south and west
        8 => { 205 } // Wall only to the east
        9 => { 200 } // Wall to the north and east
        10 => { 201 } // Wall to the south and east
        11 => { 204 } // Wall to the north, south and east
        12 => { 205 } // Wall to the east and west
        13 => { 202 } // Wall to the east, west, and south
        14 => { 203 } // Wall to the east, west, and north
        15 => { 206 }  // ╬ Wall on all sides
        _ => { 35 } // We missed one?
    }
}

pub fn draw_map(map : &Map, ctx : &mut Rltk) {
    let mut y = 0;
    let mut x = 0;
    for (idx,tile) in map.tiles.iter().enumerate() {
        // Render a tile depending upon the tile type

        if map.revealed_tiles[idx] {
            let glyph;
            let mut fg;
            let mut bg = RGB::from_f32(0., 0., 0.);
            match tile {
                TileType::Floor => {
                    glyph = rltk::to_cp437('.');
                    fg = RGB::from_f32(0.0, 0.5, 0.5);
                }
                TileType::Wall => {
                    glyph = wall_glyph(&*map, x, y);
                    fg = RGB::from_f32(0., 1.0, 0.);
                }
                TileType::DownStairs => {
                    glyph = rltk::to_cp437('>');
                    fg = RGB::from_f32(0., 1.0, 1.0);
                }
                _ => { panic!("UNKNOWN TILE") }
            }
            if map.bloodstains.contains(&idx) { bg = RGB::from_f32(0.75, 0., 0.); }
            if !map.visible_tiles[idx] {
                fg = fg.to_greyscale();
                bg = RGB::from_f32(0., 0., 0.); // Don't show stains out of visual range
            }
            ctx.set(x, y, fg, bg, glyph);
        }

        // Move coordinates
        x += 1;
        if x > (map.width * map.height) as i32 -1 {
            x = 0;
            y += 1;
        }
    }
}