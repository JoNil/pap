use std::{env, fs, process::Command};

static TEST_CASES: &[&str] = &[
    "listing_0037_single_register_mov",
    "listing_0038_many_register_mov",
    "listing_0039_more_movs",
    "listing_0040_challenge_movs",
];

fn run_test_case(test: &str) {
    let input = format!("perfaware/part1/{test}");

    let original = fs::read(&input).unwrap();

    assert!(Command::new("cargo")
        .args(["run", "-p", "pap86", "--", "-o", "target/test.asm", &input])
        .status()
        .unwrap()
        .success());

    assert!(Command::new("tools/nasm")
        .args(["target/test.asm"])
        .status()
        .unwrap()
        .success());

    let new = fs::read("target/test").unwrap();

    assert_eq!(original, new);
}

fn main() {
    if !env::current_dir().unwrap().ends_with("pap") {
        env::set_current_dir("../").unwrap();
    }

    assert!(Command::new("cargo")
        .args(["build", "-p", "pap86"])
        .status()
        .unwrap()
        .success());

    for test in TEST_CASES {
        run_test_case(test);
    }
}
