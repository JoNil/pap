use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{fs, str, time::Instant};

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

fn next_colon(input: &[u8], index: &mut usize) {
    while unsafe { *input.get_unchecked(*index) } != b':' {
        *index += 1;
    }
}

fn next_comma(input: &[u8], index: &mut usize) {
    while unsafe { *input.get_unchecked(*index) } != b',' {
        *index += 1;
    }
}

fn next_end_curly(input: &[u8], index: &mut usize) {
    while unsafe { *input.get_unchecked(*index) } != b'}' {
        *index += 1;
    }
}

fn parse(input: &str) -> Pairs {
    let mut res = Pairs { pairs: Vec::new() };
    res.pairs.reserve(10_000_000);

    let input = input
        .trim_start_matches("{\"pairs\":[")
        .trim_end_matches("]}")
        .as_bytes();

    let mut index = 0;

    while index + 16 < input.len() {
        next_colon(input, &mut index);
        let colon = index;
        next_comma(input, &mut index);
        let comma = index;
        let part = &input[colon + 1..comma];
        let x0 = fast_float::parse(part).unwrap();

        next_colon(input, &mut index);
        let colon = index;
        next_comma(input, &mut index);
        let comma = index;
        let part = &input[colon + 1..comma];
        let y0 = fast_float::parse(part).unwrap();

        next_colon(input, &mut index);
        let colon = index;
        next_comma(input, &mut index);
        let comma = index;
        let part = &input[colon + 1..comma];
        let x1 = fast_float::parse(part).unwrap();

        next_colon(input, &mut index);
        let colon = index;
        next_end_curly(input, &mut index);
        let comma = index;
        let part = &input[colon + 1..comma];
        let y1 = fast_float::parse(part).unwrap();

        res.pairs.push(Pair { x0, y0, x1, y1 });
    }

    res
}

fn main() {
    let input = fs::read_to_string("input.json").unwrap();

    let start_time = Instant::now();
    //let parsed_input = serde_json::from_str::<Pairs>(input.as_str()).unwrap();
    let parsed_input = parse(&input);
    let mid_time = Instant::now();

    let sum = parsed_input
        .pairs
        .par_iter()
        .map(haversine_of_degrees)
        .sum::<f32>();

    let average = sum / parsed_input.pairs.len() as f32;

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
        parsed_input.pairs.len() as f32 / (end_time - start_time).as_secs_f32()
    );
}
