use specs::prelude::*;
use rltk::Point;
use crate::{Bystander, EntityMoved, MyTurn, Name, Position, Quips, RunState, Viewshed};
use crate::gamelog::GameLog;
use crate::map::Map;

pub struct BystanderAI {}


impl<'a> System<'a> for BystanderAI {
    #[allow(clippy::type_complexity)]
    type SystemData = ( WriteExpect<'a, Map>,
                        ReadExpect<'a, RunState>,
                        Entities<'a>,
                        WriteStorage<'a, Viewshed>,
                        ReadStorage<'a, Bystander>,
                        WriteStorage<'a, Position>,
                        WriteStorage<'a, EntityMoved>,
                        WriteExpect<'a, rltk::RandomNumberGenerator>,
                        ReadExpect<'a, Point>,
                        WriteExpect<'a, GameLog>,
                        WriteStorage<'a, Quips>,
                        ReadStorage<'a, Name>,
                        ReadStorage<'a, MyTurn>);

    fn run(&mut self, data : Self::SystemData) {
        let (mut map, runstate, entities, mut viewshed, bystander, mut position,
            mut entity_moved, mut rng, player_pos, mut gamelog, mut quips, names, turns) = data;

        for (entity, mut viewshed,_bystander,mut pos, _turn) in (&entities, &mut viewshed, &bystander, &mut position, &turns).join() {
            // Try to move randomly
            let mut x = pos.x;
            let mut y = pos.y;
            let move_roll = rng.roll_dice(1, 6);
            match move_roll {
                1 => x -= 1,
                2 => x += 1,
                3 => y -= 1,
                4 => y += 1,
                _ => {}
            }

            if x > 0 && x < map.width-1 && y > 0 && y < map.height-1 {
                let dest_idx = map.xy_idx(x, y);
                if !map.blocked[dest_idx] {
                    let idx = map.xy_idx(pos.x, pos.y);
                    map.blocked[idx] = false;
                    pos.x = x;
                    pos.y = y;
                    entity_moved.insert(entity, EntityMoved{}).expect("Unable to insert marker");
                    map.blocked[dest_idx] = true;
                    viewshed.dirty = true;
                }
            }
        }
    }
}