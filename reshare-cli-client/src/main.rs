mod cli;
mod command;
mod utils;

type Result<T, E = anyhow::Error> = std::result::Result<T, E>;

fn main() -> Result<()> {
    let command = cli::parse_args();

    match command {
        cli::Command::Get(get_args) => command::get::execute(get_args)?,
        cli::Command::Put(put_args) => command::put::execute(put_args)?,
        cli::Command::Conf(config_args) => command::config::execute(config_args)?,
        cli::Command::Ls(list_args) => command::list::execute(list_args)?,
    }

    Ok(())
}
