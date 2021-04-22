use std::path::PathBuf;
use structopt::StructOpt;

pub fn parse_args() -> Command {
    Command::from_args()
}

#[derive(Debug, StructOpt)]
#[structopt(name = "reshare-cli-client", about = "Reshare cli client")]
pub enum Command {
    /// Configures client to work with an appropriate reshare server
    Conf(ConfigArgs),
    /// List all available files
    Ls(ListArgs),
    /// Upload files
    Put(PutArgs),
    /// Download files
    Get(GetArgs),
}

#[derive(Debug, StructOpt)]
pub struct ConfigArgs {
    #[structopt(long)]
    /// Specify a server url
    pub server_url: Option<String>,
}

#[derive(Debug, StructOpt)]
pub struct ListArgs {
    #[structopt(long)]
    /// A key phrase to list some private storage
    pub key_phrase: Option<String>,
}

#[derive(Debug, StructOpt)]
pub struct PutArgs {
    #[structopt(long)]
    /// A key phrase to put files into a hidden private storage
    pub key_phrase: Option<String>,

    /// Paths to files to upload
    pub file_list: Vec<PathBuf>,
}

#[derive(Debug, StructOpt)]
pub struct GetArgs {
    #[structopt(long)]
    /// A key phrase to get files from some private storage
    pub key_phrase: Option<String>,

    /// File names to download
    pub file_list: Vec<String>,
}
