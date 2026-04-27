use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

pub struct Ev3Connection {
    session: Session,
}

impl Ev3Connection {
    pub fn connect(ip: &str, user: &str, password: &str) -> anyhow::Result<Self> {
        let tcp = TcpStream::connect(format!("{}:22", ip))?;
        tcp.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
        tcp.set_write_timeout(Some(std::time::Duration::from_secs(5)))?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        session.userauth_password(user, password)?;
        session.set_timeout(5000); // 5s timeout por operación
        println!("✅ Conectado al EV3 en {}", ip);
        Ok(Self { session })
    }

    /// Verifica si la sesión sigue viva
    pub fn is_alive(&self) -> bool {
        self.session.authenticated()
    }

    pub fn exec(&self, cmd: &str) -> anyhow::Result<String> {
        let mut channel = self.session.channel_session()?;
        channel.exec(cmd)?;
        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;
        Ok(output.trim().to_string())
    }

    pub fn read_file(&self, path: &str) -> anyhow::Result<String> {
        self.exec(&format!("cat {}", path))
    }

    /// Escribe un archivo en el EV3 usando echo/printf (más rápido que SCP para archivos pequeños)
    pub fn write_file(&self, remote_path: &str, content: &str) -> anyhow::Result<()> {
        // Escapar el contenido para pasarlo por shell
        let escaped = content
            .replace('\\', "\\\\")
            .replace('\'', "'\\''");
        self.exec(&format!("printf '%s' '{}' > {}", escaped, remote_path))?;
        Ok(())
    }
}

// ─── Conexión compartida persistente ─────────────────────────────────────────
// Un único Arc<Mutex<Option<Ev3Connection>>> que todos los módulos comparten.
// Si la conexión se cae, cualquiera puede reconectar y el resto la reutiliza.

pub type SharedConn = Arc<Mutex<Option<Ev3Connection>>>;

pub fn make_shared_conn() -> SharedConn {
    Arc::new(Mutex::new(None))
}

/// Intenta usar la conexión existente; si no hay o se cayó, reconecta.
/// Devuelve true si hay conexión activa después de la llamada.
pub fn ensure_connected(conn: &SharedConn, ip: &str, user: &str, pass: &str) -> bool {
    let mut guard = conn.lock().unwrap();
    if guard.as_ref().map(|c| c.is_alive()).unwrap_or(false) {
        return true; // ya conectado
    }
    // Reconectar
    match Ev3Connection::connect(ip, user, pass) {
        Ok(c) => { *guard = Some(c); true }
        Err(e) => { println!("❌ No se pudo conectar: {}", e); *guard = None; false }
    }
}

/// Ejecuta un comando usando la conexión compartida.
/// Si falla por conexión caída, limpia para forzar reconexión en el próximo ciclo.
pub fn exec_shared(conn: &SharedConn, cmd: &str) -> anyhow::Result<String> {
    let guard = conn.lock().unwrap();
    match guard.as_ref() {
        Some(c) => c.exec(cmd).map_err(|e| {
            drop(guard);
            e
        }),
        None => Err(anyhow::anyhow!("Sin conexión")),
    }
}

pub fn write_file_shared(conn: &SharedConn, path: &str, content: &str) -> anyhow::Result<()> {
    let guard = conn.lock().unwrap();
    match guard.as_ref() {
        Some(c) => c.write_file(path, content),
        None => Err(anyhow::anyhow!("Sin conexión")),
    }
}