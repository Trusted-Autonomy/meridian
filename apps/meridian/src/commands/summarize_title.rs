use anyhow::Result;
use clap::Args;
use meridian_config::MeridianConfig;
use meridian_report::suggest as suggest_lib;
use std::io::Read;
use std::path::Path;

#[derive(Args)]
pub struct SummarizeTitleArgs {
    /// Raw text to summarize. If not provided, reads from stdin.
    #[arg(long)]
    pub text: Option<String>,
}

pub fn run(args: SummarizeTitleArgs, config_path: &Path) -> Result<()> {
    let cfg = MeridianConfig::load_or_default(config_path);

    let raw = match args.text {
        Some(t) => t,
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| anyhow::anyhow!("Failed to read stdin: {e}"))?;
            buf
        }
    };

    let api_key = cfg
        .suggest
        .api_key
        .clone()
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok());
    let use_claude_cli = api_key.is_none() && suggest_lib::claude_cli_available();

    if api_key.is_none() && !use_claude_cli {
        anyhow::bail!(
            "Set ANTHROPIC_API_KEY or install the claude CLI (https://claude.ai/code) \
             to enable title summarization."
        );
    }

    let suggest_cfg = suggest_lib::SuggestConfig {
        threshold: cfg.suggest.threshold,
        sample_size: cfg.suggest.sample_size,
        model: cfg.suggest.model.clone(),
        api_key,
        use_claude_cli,
    };

    let title = suggest_lib::summarize_title(&suggest_cfg, &raw)?;
    println!("{title}");
    Ok(())
}
