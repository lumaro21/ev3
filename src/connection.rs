use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;

pub struct Ev3Connection {
    session: Session,
}

impl Ev3Connection {
    pub fn connect(ip: &str, user: &str, password: &str) -> anyhow::Result<Self> {
        let tcp = TcpStream::connect(format!("{}:22", ip))?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        session.userauth_password(user, password)?;

        println!("✅ Conectado al EV3 en {}", ip);
        Ok(Self { session })
    }

    pub fn read_file(&self, path: &str) -> anyhow::Result<String> {
        let mut channel = self.session.channel_session()?;
        channel.exec(&format!("cat {}", path))?;
        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;
        Ok(output.trim().to_string())
    }

    pub fn exec(&self, cmd: &str) -> anyhow::Result<String> {
        let mut channel = self.session.channel_session()?;
        channel.exec(cmd)?;
        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;
        Ok(output.trim().to_string())
    }
}