use super::*;
use crate::utils::OptionExt;
use dialoguer::Input;

pub fn execute(args: ConfigArgs) -> Result<()> {
    let url = args.server_url.ok_or_try(prompt)?;
    configure(&url)?;

    println!("Configuration successful");
    Ok(())
}

fn prompt() -> Result<String> {
    let url = Input::new().with_prompt("Enter server url").interact()?;
    Ok(url)
}
