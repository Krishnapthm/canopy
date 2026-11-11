mod cli;
mod client;
mod daemon;
mod demo;
mod fake_agent;
mod harness;
mod jsonrpc;
mod mcp_stdio;
mod paths;
mod tui;
mod web;

use anyhow::Result;

const HELP: &str = "canopy - run coding agents from different vendors as one live tree

usage:
  canopy                 open the TUI control room
  canopy web             serve the browser control room until Ctrl+C
  canopy doctor          check installed harnesses
  canopy demo            try a disposable fake-agent tree
  canopy help            show this help (-h, --help)
  canopy --version       show the version

env: CANOPY_HOME (default ~/.canopy)";

fn main() {
    if let Err(e) = run() {
        eprintln!("canopy: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().cloned() else {
        return block_on(tui::run());
    };
    let rest = args[1..].to_vec();
    let mut p = pico_args::Arguments::from_vec(rest.iter().map(Into::into).collect());
    match cmd.as_str() {
        "daemon" => daemon::run(
            daemon::Opts {
                foreground: p.contains("--foreground"),
                acp: p.values_from_str("--acp")?,
                max_depth: p.opt_value_from_str("--max-depth")?.unwrap_or(3),
                max_live: p.opt_value_from_str("--max-live")?.unwrap_or(16),
                root_budget: p.opt_value_from_str("--budget")?,
            },
            std::iter::once("daemon".to_owned()).chain(rest).collect(),
        ),
        "mcp-stdio" => {
            let sock: String = p.value_from_str("--sock")?;
            let session: String = p.value_from_str("--session")?;
            block_on(mcp_stdio::run(sock, session))
        }
        "demo" => demo::run(),
        "web" => block_on(web::run()),
        "fake-agent" => block_on(fake_agent::run()),
        "-h" | "--help" | "help" => {
            println!("{HELP}");
            Ok(())
        }
        "doctor" => block_on(cli::doctor()),
        "--version" => {
            println!("canopy {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => anyhow::bail!("unknown command {cmd}; try canopy help"),
    }
}

fn block_on<F: Future<Output = Result<()>>>(f: F) -> Result<()> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(f)
}
