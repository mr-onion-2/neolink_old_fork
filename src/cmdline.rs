use std::path::PathBuf;
use structopt::{clap::AppSettings, StructOpt};

/// A standards-compliant bridge to Reolink IP cameras
#[derive(StructOpt, Debug)]
#[structopt(name = "neolink", setting(AppSettings::ArgRequiredElseHelp))]
pub struct Opt {
    #[structopt(short, long, parse(from_os_str))]
    pub config: Option<PathBuf>,
    #[structopt(subcommand)]
    pub cmd: Option<Command>,
}

#[derive(StructOpt, Debug)]
pub enum Command {
    Rtsp(super::rtsp::Opt),
    StatusLight(super::statusled::Opt),
    Reboot(super::reboot::Opt),
    Talk(super::talk::Opt),
}
