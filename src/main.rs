use anyhow::Result;
use clap::Parser;
use pokedex::cli::*;
use pokedex::discover;
use pokedex::output::{OutputFormat, ErrorResponse, Action};

fn main() -> Result<()> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            if e.use_stderr() {
                let message = e.to_string().lines().next().unwrap_or("Invalid command").to_string();
                let _ = ErrorResponse::invalid_parameter(
                    &message,
                    vec![Action::new("discover", "pokedex --discover"),
                         Action::new("help", "pokedex --help")],
                ).print();
                unreachable!()
            } else {
                e.exit()
            }
        }
    };

    if cli.discover {
        return discover::print_discovery();
    }

    let format: OutputFormat = cli.format.parse().unwrap_or_default();

    let mut conn = pokedex::db::open()?;
    pokedex::dispatch(cli.command, &format, &mut conn)
}
