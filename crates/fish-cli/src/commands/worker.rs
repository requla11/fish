use std::process::ExitCode;

use crate::args::WorkerArgs;
use fish_worker::WorkerServer;

pub fn run_worker(args: WorkerArgs) -> ExitCode {
    println!("🦀 Fish Distributed Worker Node");
    println!("Worker name:  {}", args.name);
    println!("Listening on: {}", args.listen);
    println!("Concurrency:  {}", args.max_concurrency);
    if args.auth_token.is_none()
        && !args.listen.starts_with("127.0.0.1")
        && !args.listen.starts_with("localhost")
        && !args.listen.starts_with("[::1]")
    {
        eprintln!(
            "warning: worker listening on non-loopback address without an auth token; remote tasks from external IPs will be rejected"
        );
    }
    let server = WorkerServer::with_options(
        args.listen,
        args.auth_token,
        args.name,
        args.max_concurrency,
    );
    match server.run_blocking() {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: worker daemon failed: {err}");
            ExitCode::FAILURE
        }
    }
}
