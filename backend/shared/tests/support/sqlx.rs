use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static CONTAINER_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct SqlxPostgres {
    pub name: String,
    host: String,
    host_port: u16,
}

impl Drop for SqlxPostgres {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.name])
            .status();
    }
}

pub fn setup_sqlx_postgres() -> SqlxPostgres {
    let sequence = CONTAINER_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let host = docker_host_gateway();
    let host_port = host_port(sequence);
    let name = format!(
        "linkdrop-processing-{}-{suffix}-{sequence}",
        std::process::id()
    );
    let port_mapping = format!("0.0.0.0:{host_port}:5432");

    let output = Command::new("docker")
        .args([
            "run",
            "-d",
            "--rm",
            "--name",
            &name,
            "-p",
            &port_mapping,
            "-e",
            "POSTGRES_PASSWORD=postgres",
            "postgres:16-alpine",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "docker run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    wait_for_host_port(&host, host_port);
    wait_for_psql(&name);
    SqlxPostgres {
        name,
        host,
        host_port,
    }
}

pub fn database_url(container: &SqlxPostgres) -> String {
    format!(
        "postgres://postgres:postgres@{}:{}/postgres?sslmode=disable",
        container.host, container.host_port
    )
}

pub fn psql(container_name: &str, sql: &str) -> Output {
    Command::new("docker")
        .args(psql_command_args(container_name, sql))
        .output()
        .unwrap()
}

fn wait_for_host_port(host: &str, host_port: u16) {
    let address = SocketAddr::new(host.parse().unwrap(), host_port);
    for _ in 0..60 {
        if TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    panic!("Postgres host port did not become ready");
}

fn wait_for_psql(container_name: &str) {
    for _ in 0..60 {
        if psql(container_name, "SELECT 1;").status.success() {
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    panic!("Postgres did not become ready for psql");
}

fn psql_command_args(container_name: &str, sql: &str) -> Vec<String> {
    let mut args = psql_network_args(container_name, "psql");
    args.extend(["-d", "postgres", "-v", "ON_ERROR_STOP=1", "-qAt", "-c"].map(str::to_string));
    args.push(sql.to_string());
    args
}

fn psql_network_args(container_name: &str, command: &str) -> Vec<String> {
    vec![
        "run".to_string(),
        "--rm".to_string(),
        "--network".to_string(),
        format!("container:{container_name}"),
        "-e".to_string(),
        "PGPASSWORD=postgres".to_string(),
        "postgres:16-alpine".to_string(),
        command.to_string(),
        "-h".to_string(),
        "127.0.0.1".to_string(),
        "-U".to_string(),
        "postgres".to_string(),
    ]
}

fn host_port(sequence: u64) -> u16 {
    let base = 30_000 + (u64::from(std::process::id()) % 10_000);
    (base + sequence) as u16
}

fn docker_host_gateway() -> String {
    fs::read_to_string("/proc/net/route")
        .unwrap()
        .lines()
        .skip(1)
        .find_map(default_gateway)
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

fn default_gateway(line: &str) -> Option<String> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.get(1) != Some(&"00000000") {
        return None;
    }
    let raw = u32::from_str_radix(fields.get(2)?, 16).ok()?;
    Some(IpAddr::V4(Ipv4Addr::from(raw.to_le_bytes())).to_string())
}
