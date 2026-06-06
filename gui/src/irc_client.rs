//! Bridges an asynchronous `irc-repartee` connection into an iced
//! `Subscription`, following halloy's pattern: the connection runs inside an
//! `iced::stream::channel` task and emits [`Event`]s into the iced update loop.

use futures::{SinkExt, Stream, StreamExt};

use crate::config::Config;

/// Events surfaced from the IRC connection task to the iced app.
#[derive(Clone)]
pub enum Event {
    /// Connection registered; carries the command sender and our nick.
    Connected(irc::client::Sender, String),
    /// A raw protocol message from the server.
    Message(Box<irc::proto::Message>),
    /// Connection ended (with a human-readable reason).
    Disconnected(String),
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connected(_, nick) => write!(f, "Connected({nick})"),
            Self::Message(_) => f.write_str("Message(..)"),
            Self::Disconnected(reason) => write!(f, "Disconnected({reason})"),
        }
    }
}

/// A `Subscription`-friendly stream that connects and yields [`Event`]s.
///
/// Reads `~/.repartee/gui.toml` for the target server. On disconnect it parks
/// (no auto-reconnect in the MVP — see the design doc) so iced does not respawn
/// it in a tight loop.
pub fn connect() -> impl Stream<Item = Event> {
    iced::stream::channel(256, |mut output| async move {
        let cfg = Config::load_or_init();
        let result = run(&cfg, &mut output).await;
        let reason = match result {
            Ok(()) => "connection closed".to_string(),
            Err(e) => e.to_string(),
        };
        let _ = output.send(Event::Disconnected(reason)).await;
        // Park so the subscription stays alive instead of being respawned.
        futures::future::pending::<()>().await;
    })
}

async fn run(
    cfg: &Config,
    output: &mut futures::channel::mpsc::Sender<Event>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use irc::client::prelude::{Client, Config as IrcConfig};

    let irc_config = IrcConfig {
        nickname: Some(cfg.nick.clone()),
        username: Some(cfg.username.clone()),
        realname: Some(cfg.realname.clone()),
        server: Some(cfg.server.clone()),
        port: Some(cfg.port),
        use_tls: Some(cfg.tls),
        channels: cfg.channels.clone(),
        ..IrcConfig::default()
    };

    let mut client = Client::from_config(irc_config).await?;
    client.identify()?;
    let sender = client.sender();
    let mut stream = client.stream()?;

    output
        .send(Event::Connected(sender, cfg.nick.clone()))
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    while let Some(message) = stream.next().await.transpose()? {
        if output.send(Event::Message(Box::new(message))).await.is_err() {
            break;
        }
    }
    Ok(())
}
