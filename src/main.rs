use std::io::{self, Write};

use backend::Engine;

fn main() {
    let mut engine = Engine::new();

    println!("MiniDB");
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

        if input.eq_ignore_ascii_case("exit")
            || input.eq_ignore_ascii_case("quit")
        {
            break;
        }

        if input.is_empty() {
            continue;
        }

        match engine.execute(input) {
            Ok(_) => {
                println!("OK");
            }

            Err(err) => {
                eprintln!("ERROR: {err:?}");
            }
        }
    }

    println!("Goodbye.");
}