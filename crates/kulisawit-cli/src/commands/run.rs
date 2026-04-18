use anyhow::Result;

use crate::RunArgs;

pub async fn run(args: RunArgs) -> Result<()> {
    println!(
        "run: db={} repo={} task={} agent={} batch={}",
        args.db.display(),
        args.repo.display(),
        args.task,
        args.agent,
        args.batch
    );
    Ok(())
}
