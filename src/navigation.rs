//! Layered walking graph built from the same geometry as collision, cached per thread.
use crate::{
    game::{Block, V3},
    movement,
};
use std::{
    cell::RefCell,
    collections::{hash_map::DefaultHasher, VecDeque},
    hash::{Hash, Hasher},
};

const W: usize = ((crate::level::MAX_X - crate::level::MIN_X - 1.0) * 2.0) as usize + 1;
const H: usize = ((crate::level::MAX_Z - crate::level::MIN_Z - 1.0) * 2.0) as usize + 1;

struct Navigation {
    points: Vec<V3>,
    edges: Vec<Vec<usize>>,
}

impl Navigation {
    fn build(map: &[Block]) -> Self {
        // Keep grid nodes off exact wall/column tangencies. A path that merely
        // grazes a corner can stall the axis-separated movement sweep when the
        // correction on one axis is smaller than its movement epsilon.
        let clearance: Vec<Block> = map
            .iter()
            .cloned()
            .map(|mut b| {
                b.size.x += 0.1;
                b.size.z += 0.1;
                b
            })
            .collect();
        let map = clearance.as_slice();
        let mut points = Vec::new();
        let mut columns = vec![Vec::new(); W * H];
        for (cell, column) in columns.iter_mut().enumerate() {
            let p = V3::new(
                (cell % W) as f32 * 0.5 + crate::level::MIN_X + 0.5,
                0.0,
                (cell / W) as f32 * 0.5 + crate::level::MIN_Z + 0.5,
            );
            let mut heights = vec![0.0];
            for b in map {
                let lo = b.min();
                let hi = b.max();
                if p.x > lo.x - 0.399
                    && p.x < hi.x + 0.399
                    && p.z > lo.z - 0.399
                    && p.z < hi.z + 0.399
                {
                    heights.push(hi.y);
                }
            }
            heights.sort_by(f32::total_cmp);
            heights.dedup_by(|a, b| (*a - *b).abs() < 0.01);
            for y in heights {
                let p = V3::new(p.x, y, p.z);
                if !movement::blocked(map, p) {
                    column.push(points.len());
                    points.push(p);
                }
            }
        }
        let mut edges = vec![Vec::new(); points.len()];
        for (cell, column) in columns.iter().enumerate() {
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let x = (cell % W) as i32 + dx;
                let z = (cell / W) as i32 + dz;
                if x < 0 || z < 0 || x >= W as i32 || z >= H as i32 {
                    continue;
                }
                for &a in column {
                    for &b in &columns[x as usize + z as usize * W] {
                        let rise = points[b].y - points[a].y;
                        if (-4.0..=movement::STEP_HEIGHT).contains(&rise)
                            && movement::walkable_line(map, points[a], points[b])
                        {
                            edges[a].push(b);
                        }
                    }
                }
            }
        }
        Self { points, edges }
    }

    fn nearest(&self, p: V3) -> Option<usize> {
        self.points
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let distance = |v: V3| (v - p).flat().length() + (v.y - p.y).abs() * 3.0;
                distance(**a).total_cmp(&distance(**b))
            })
            .map(|(i, _)| i)
    }

    fn next(&self, map: &[Block], from: V3, to: V3) -> V3 {
        let (Some(start), Some(end)) = (self.nearest(from), self.nearest(to)) else {
            return from;
        };
        if start == end {
            return to;
        }
        let mut previous = vec![usize::MAX; self.points.len()];
        previous[start] = start;
        let mut queue = VecDeque::from([start]);
        while let Some(n) = queue.pop_front() {
            if n == end {
                break;
            }
            for &next in &self.edges[n] {
                if previous[next] == usize::MAX {
                    previous[next] = n;
                    queue.push_back(next);
                }
            }
        }
        if previous[end] == usize::MAX {
            return from;
        }
        let mut path = vec![end];
        let mut n = end;
        while previous[n] != start {
            n = previous[n];
            path.push(n);
        }
        path.reverse();
        for &n in path.iter().take(5).rev() {
            if movement::walkable_line(map, from, self.points[n]) {
                return self.points[n];
            }
        }
        self.points[path[0]]
    }
}

thread_local! {
    static CACHE: RefCell<Option<(u64, Navigation)>> = const { RefCell::new(None) };
}

pub fn next_waypoint(map: &[Block], from: V3, to: V3) -> V3 {
    if movement::walkable_line(map, from, to) {
        return to;
    }
    let mut hash = DefaultHasher::new();
    for b in map {
        for f in [
            b.center.x, b.center.y, b.center.z, b.size.x, b.size.y, b.size.z,
        ] {
            f.to_bits().hash(&mut hash);
        }
    }
    let key = hash.finish();
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.as_ref().is_none_or(|(old, _)| *old != key) {
            *cache = Some((key, Navigation::build(map)));
        }
        cache.as_ref().unwrap().1.next(map, from, to)
    })
}
