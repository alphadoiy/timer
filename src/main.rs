use timer::cli::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse_args();
    timer::run(cli)
}
