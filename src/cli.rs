use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "winproc-tui",
    version,
    about = "Windows process investigation TUI"
)]
pub(crate) struct Cli {
    #[arg(long, hide = true)]
    pub(crate) file_users_helper: Option<String>,
}
