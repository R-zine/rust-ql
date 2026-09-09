use std::{
    io::{self, BufRead, Write},
    time::Instant,
};

use backend::{Engine, ExecutionResult};

fn main() {
    let mut engine = match Engine::new() {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("Failed to open database: {error}");
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let stderr = io::stderr();
    let mut error_output = stderr.lock();

    if let Err(error) = run_repl(&mut engine, &mut input, &mut output, &mut error_output) {
        eprintln!("I/O error: {error}");
        std::process::exit(1);
    }
}

fn run_repl(
    engine: &mut Engine,
    input: &mut impl BufRead,
    output: &mut impl Write,
    error_output: &mut impl Write,
) -> io::Result<()> {
    writeln!(output, "Rust DB")?;
    writeln!(output, "Type 'exit' or 'quit' to close.")?;
    writeln!(output)?;

    loop {
        write!(output, "db> ")?;
        output.flush()?;

        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();

        if line.eq_ignore_ascii_case("exit") || line.eq_ignore_ascii_case("quit") {
            break;
        }
        if line.is_empty() {
            continue;
        }

        let start = Instant::now();
        let result = if is_seed_command(line) {
            execute_seed(engine, line)
        } else {
            engine.execute(line)
        };

        match result {
            Ok(result) => {
                print_result(output, &result)?;
                writeln!(output, "OK ({:?})", start.elapsed())?;
            }
            Err(error) => writeln!(error_output, "ERROR: {error}")?,
        }
    }

    writeln!(output, "Goodbye.")
}

fn is_seed_command(line: &str) -> bool {
    line.split_whitespace()
        .next()
        .is_some_and(|command| command.eq_ignore_ascii_case(".seed"))
}

fn execute_seed(engine: &mut Engine, line: &str) -> Result<ExecutionResult, backend::BackendError> {
    let parts: Vec<_> = line.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(backend::BackendError::Execution(
            "usage: .seed <table> <count>".into(),
        ));
    }
    let count = parts[2]
        .parse::<usize>()
        .map_err(|_| backend::BackendError::Execution("seed count must be an integer".into()))?;
    engine.seed(parts[1], count)
}

fn print_result(output: &mut impl Write, result: &ExecutionResult) -> io::Result<()> {
    match result {
        ExecutionResult::TableCreated { table } => writeln!(output, "Created table {table}"),
        ExecutionResult::RowsInserted { count } => {
            writeln!(output, "Inserted {count} row(s)")
        }
        ExecutionResult::Query(query) => {
            writeln!(output, "{}", query.columns.join(" | "))?;
            for row in &query.rows {
                let values: Vec<_> = row.iter().map(ToString::to_string).collect();
                writeln!(output, "{}", values.join(" | "))?;
            }
            writeln!(output, "{} row(s)", query.rows.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Cursor,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use backend::Engine;

    use super::run_repl;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unused_database_path() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("rustql-repl-{}-{id}.json", std::process::id()))
    }

    #[test]
    fn exits_cleanly_when_stdin_reaches_eof() {
        let mut engine = Engine::with_path(unused_database_path()).unwrap();
        let mut input = Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        let mut errors = Vec::new();

        run_repl(&mut engine, &mut input, &mut output, &mut errors).unwrap();

        assert!(
            String::from_utf8(output)
                .unwrap()
                .ends_with("db> Goodbye.\n")
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn only_exact_seed_commands_use_the_meta_command() {
        let mut engine = Engine::with_path(unused_database_path()).unwrap();
        let mut input = Cursor::new(b".seedling users 1\nexit\n".to_vec());
        let mut output = Vec::new();
        let mut errors = Vec::new();

        run_repl(&mut engine, &mut input, &mut output, &mut errors).unwrap();

        let errors = String::from_utf8(errors).unwrap();
        assert!(errors.contains("unexpected character '.'"));
        assert!(!errors.contains("table 'users' does not exist"));
    }
}
