extern crate imap;
extern crate native_tls;
extern crate config;
extern crate serde;
extern crate lettre;

use std::error::Error;
use native_tls::TlsConnector;
use config::{ConfigBuilder, File, builder::DefaultState};
use serde::Deserialize;
use std::net::TcpStream;
use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::authentication::Credentials;
use imap::Session;
use std::thread::sleep;
use std::time::Duration;

#[derive(Deserialize)]
struct EmailConfig {
    username: String,
    password: String,
    sender_email: String,
    imap_server: String,
    imap_port: u16,
    smtp_server: String,
    smtp_port: u16,
}

fn callback_fn(email_text: &str) -> String {
    // Process the email text and return the response
    format!("Processed: {}", email_text)
}

fn main() -> Result<(), Box<dyn Error>> {
    // Load configuration
    let settings: config::Config = ConfigBuilder::<DefaultState>::default()
        .add_source(File::with_name("secrets"))
        .build()?;
    let email_config: EmailConfig = settings.get::<EmailConfig>("email")?;

    // Establish a secure connection to the IMAP server
    let tls = TlsConnector::builder().build()?;
    let tcp_stream = TcpStream::connect((email_config.imap_server.as_str(), email_config.imap_port))?;
    let tls_stream = tls.connect(&email_config.imap_server, tcp_stream)?;
    let client = imap::Client::new(tls_stream);

    // Login using credentials from the configuration file
    let mut session = client.login(&email_config.username, &email_config.password).map_err(|e| e.0)?;

    // Prepare the SMTP client for sending responses
    let creds = Credentials::new(email_config.username.clone(), email_config.password.clone());
    let mailer = SmtpTransport::relay(&email_config.smtp_server)?
        .credentials(creds)
        .build();

    loop {
        // Select the inbox
        session.select("INBOX")?;

        // Fetch unseen emails from the specified sender
        let query = format!("(UNSEEN FROM {})", email_config.sender_email);
        let messages = session.search(query)?;
        for msg_id in messages.iter() {
            let message = session.fetch(msg_id.to_string(), "BODY.PEEK[]")?;
            for message in message.iter() {
                if let Some(body) = message.body() {
                    let email_text = std::str::from_utf8(body)?;
                    let response = callback_fn(email_text);

                    // Send response back to the sender
                    let email = Message::builder()
                        .from(email_config.username.parse()?)
                        .to(email_config.sender_email.parse()?)
                        .subject("Response")
                        .body(response)?;

                    mailer.send(&email)?;
                }
            }
        }

        // Sleep for a while before checking again
        sleep(Duration::from_secs(60));
    }

    // Logout (This will not be reached due to the infinite loop)
    // session.logout()?;

    // This line will not be reached
    // Ok(())
}
