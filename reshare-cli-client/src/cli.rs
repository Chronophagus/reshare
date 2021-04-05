use structopt::StructOpt;

pub fn parse_args() -> Command {
    Command::from_args()
}

#[derive(Debug, StructOpt)]
#[structopt(name = "reshare-cli-client", about = "Reshare cli client")]
pub enum Command {
    /// Configures client to work with an appropriate reshare server
    Config(ConfigArgs),
    /// Lists all public files on the server
    List,
    /// Uploads files on the server
    Put(PutArgs),
    /// Downloads files from the server
    Get(GetArgs),
}

#[derive(Debug, StructOpt)]
pub struct ConfigArgs {
    #[structopt(long)]
    pub server_addr: Option<String>,
}

#[derive(Debug, StructOpt)]
pub struct PutArgs {
    pub key_phrase: Option<String>,
    pub ttl: Option<u32>,
    pub public: Option<String>,
}

#[derive(Debug, StructOpt)]
pub struct GetArgs {
    pub key_phrase: Option<String>,
    pub public: Option<String>,
}
