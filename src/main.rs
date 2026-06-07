use std::{
    io::{self, Write},
    time::Instant,
};

use backend::Engine;

fn main() {
    let mut engine = Engine::new();

    println!("Rust DB");
    println!("Type 'exit' or 'quit' to close.");
    println!();

    loop {
        print!("db> ");

        if let Err(err) = io::stdout().flush() {
            eprintln!("Failed to flush stdout: {err}");
            continue;
        }

        let mut input = String::new();

        if let Err(err) = io::stdin().read_line(&mut input) {
            eprintln!("Failed to read input: {err}");
            continue;
        }

        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            break;
        }

        if input.is_empty() {
            continue;
        }

        if input.to_ascii_lowercase().starts_with(".seed") {
            let parts: Vec<&str> = input.split_whitespace().collect();

            if parts.len() != 3 {
                eprintln!("Usage: seed <table> <count>");
                continue;
            }

            let table = parts[1];

            let count = match parts[2].parse::<usize>() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("Invalid count");
                    continue;
                }
            };

            match engine.seed(table, count) {
                Ok(_) => println!("Inserted {count} rows into {table}"),
                Err(err) => eprintln!("ERROR: {err:?}"),
            }

            continue;
        }

        let start = Instant::now();
        match engine.execute(input) {
            Ok(_) => {
                let elapsed = start.elapsed();

                println!("OK ({elapsed:?})");
            }

            Err(err) => {
                eprintln!("ERROR: {err:?}");
            }
        }
    }

    println!("Goodbye.");
}
