use serde::{Deserialize, Serialize};
use std::{fs, time::Instant};


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

const EARTH_RADIUS_KM: f32 = 6371.0;

fn haversine_of_degrees(p: &Pair) -> f32 {
    let dy = (p.y1 - p.y0).to_radians();
    let dx = (p.x1 - p.x0).to_radians();
    let y0 = p.y0.to_radians();
    let y1 = p.y1.to_radians();

    let sin_dy = f32::sin(dy / 2.0);
    let sin_dx = f32::sin(dx / 2.0);

    let root_term = (sin_dy * sin_dy) + f32::cos(y0) * f32::cos(y1) * (sin_dx * sin_dx);
    2.0 * EARTH_RADIUS_KM * 2.0 * f32::asin(f32::sqrt(root_term))
}

fn main() {
    let mut input = fs::read_to_string("input.json").unwrap();

    let start_time = Instant::now();
    let parsed_input = serde_json::from_str::<Pairs>(input.as_mut_str()).unwrap();
    let mid_time = Instant::now();

    let mut sum = 0.0;
    let mut count = 0;

    dbg!(parsed_input.pairs.len());

    for p in parsed_input.pairs {
        sum += haversine_of_degrees(&p);
        count += 1;
    }

    let average = sum / count as f32;

    let end_time = Instant::now();

    println!("Result: {average}");
    println!(
        "Input = {} seconds",
        (mid_time - start_time).as_millis() as f32 / 1000.0
    );
    println!(
        "Math = {} seconds",
        (end_time - mid_time).as_millis() as f32 / 1000.0
    );
    println!(
        "Total = {} seconds",
        (end_time - start_time).as_millis() as f32 / 1000.0
    );
    println!(
        "Throughput = {} haversines/second",
        count as f32 / (end_time - start_time).as_secs_f32()
    );
}
