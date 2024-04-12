use anyhow::Result;
use std::ffi::OsStr;
use clap::Args;
use fossilizer::config;
use std::error::Error;
use std::net::SocketAddr;

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Host at which to serve files
    #[arg(long, short = 'n', default_value = "127.0.0.1")]
    host: String,
    /// Port at which to serve files
    #[arg(long, short = 'p', default_value = "8881")]
    port: u16,
    /// Open a web browser after starting the server
    #[arg(long)]
    open: bool,
}

pub async fn command(args: &ServeArgs) -> Result<(), Box<dyn Error>> {
    let open_browser = args.open;

    let config = config::config()?;
    let build_path = config.build_path;

    let addr = format!("{}:{}", args.host.clone(), args.port);
    let addr: SocketAddr = addr.parse().unwrap();

    let serving_url = format!("http://{}", addr);

    info!(
        "Serving up {} at {}",
        build_path.to_str().unwrap(),
        serving_url
    );

    if open_browser {
        open(serving_url);
    }

    warp::serve(warp::fs::dir(build_path)).run(addr).await;
    Ok(())
}

fn open<P: AsRef<OsStr>>(path: P) {
    info!("Opening web browser");
    if let Err(e) = opener::open(path) {
        error!("Error opening web browser: {}", e);
    }
}
