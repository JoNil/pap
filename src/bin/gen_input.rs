use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Pair {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Pairs {
    pub pairs: Vec<Pair>,
}

fn main() {
    let mut pairs = Pairs::default();

    for _ in 0..10_000_000 {
        pairs.pairs.push(Pair {
            x0: rand::random(),
            y0: rand::random(),
            x1: rand::random(),
            y1: rand::random(),
        });
    }

    fs::write("input.json", serde_json::to_string(&pairs).unwrap()).unwrap();
}
