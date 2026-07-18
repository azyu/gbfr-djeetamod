#![recursion_limit = "256"]

use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use futures::sink::SinkExt;
use interprocess::os::windows::named_pipe::tokio::{PipeListenerOptionsExt, SendPipeStream};
use interprocess::os::windows::named_pipe::{pipe_mode, PipeListenerOptions, PipeMode};
use log::{info, warn};
use tokio::sync::broadcast;

mod event;
mod hooks;
mod process;

use protocol::{HookStatus, Message};
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};

async fn send_message(
    stream: &mut FramedWrite<SendPipeStream<pipe_mode::Bytes>, LengthDelimitedCodec>,
    message: &Message,
) -> Result<()> {
    let bytes = protocol::bincode::serialize(message)?;
    stream.send(bytes.into()).await?;
    Ok(())
}

async fn handle_client(
    mut stream: FramedWrite<SendPipeStream<pipe_mode::Bytes>, LengthDelimitedCodec>,
    mut rx: event::Rx,
    hook_status: HookStatus,
) -> Result<()> {
    send_message(&mut stream, &Message::HookStatus(hook_status)).await?;
    while let Ok(message) = rx.recv().await {
        send_message(&mut stream, &message).await?;
    }

    Ok(())
}

#[derive(Debug)]
struct Server {
    tx: event::Tx,
}

impl Server {
    fn new() -> Self {
        let (tx, _) = broadcast::channel::<Message>(1024);
        Server { tx }
    }

    async fn run(&self, hook_status: HookStatus) {
        if let Ok(listener) = PipeListenerOptions::new()
            .path(protocol::PIPE_NAME)
            .mode(PipeMode::Bytes)
            .accept_remote(false)
            .create_tokio_send_only()
        {
            loop {
                let read_pipe = listener.accept().await;
                match read_pipe {
                    Ok(stream) => {
                        let rx = self.tx.subscribe();
                        tokio::spawn(async move {
                            let encoder = LengthDelimitedCodec::new();
                            let writer = FramedWrite::new(stream, encoder);

                            let _ = handle_client(writer, rx, hook_status).await;
                        });
                    }
                    Err(e) => {
                        warn!("Error accepting client: {:?}", e);
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn setup() {
    info!("Setting up named pipe listener");

    let server = Server::new();
    let tx = server.tx.clone();

    info!("Setting up hooks...");

    let hook_status = match hooks::setup_hooks(tx) {
        Ok(()) => {
            info!("Hooks initialized");
            HookStatus::Ready
        }
        Err(error) => {
            warn!("Required meter hook unavailable: {error:?}");
            HookStatus::Unsupported
        }
    };

    #[cfg(feature = "console")]
    println!("Hook library initialized");

    let _ = std::io::stdout().flush();

    server.run(hook_status).await;
}

fn initialize_logger() -> anyhow::Result<()> {
    let application_data_dir = dirs::data_dir().context("Could not find data folder")?;
    let mut log_file = PathBuf::new();

    log_file.push(application_data_dir);
    log_file.push("gbfr-logs-awa");
    std::fs::create_dir_all(log_file.as_path())?;
    log_file.push("gbfr-logs-awa.txt");

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(fern::log_file(log_file)?)
        .apply()?;

    Ok(())
}

#[ctor::ctor]
fn entry() {
    #[cfg(feature = "console")]
    unsafe {
        let _ = windows::Win32::System::Console::AllocConsole();
    }

    let _ = initialize_logger();
    std::thread::spawn(setup);
}
