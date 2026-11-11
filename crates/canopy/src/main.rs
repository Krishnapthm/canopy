mod cli;
mod client;
mod daemon;
mod fake_agent;
mod harness;
mod jsonrpc;
mod mcp_stdio;
mod paths;
mod tui;

use anyhow::Result;

const HELP: &str = "canopy - run coding agents from different vendors as one live tree

usage:
  canopy run <harness> [--cwd DIR] [--model M] [--policy ask|auto_edit|yolo]
             [--title T] [--parent ID] <prompt...>     start an agent, print its id
  canopy ls                                            show the tree
  canopy tail <id> [-f]                                print a transcript (-f: follow until it settles)
  canopy send <id> [--interrupt] <text...>             message an agent
  canopy permit <id> <request> allow|deny|always       answer a permission request
  canopy stop <id>                                     stop an agent and everything it spawned
  canopy doctor                                        check installed harnesses
  canopy daemon [--foreground] [--acp NAME=CMD]... [--max-depth N] [--max-live N] [--budget TOKENS]

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
        "fake-agent" => block_on(fake_agent::run()),
        "-h" | "--help" | "help" => {
            println!("{HELP}");
            Ok(())
        }
        _ => block_on(cli::run(cmd, p)),
    }
}

fn block_on<F: Future<Output = Result<()>>>(f: F) -> Result<()> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(f)
}
