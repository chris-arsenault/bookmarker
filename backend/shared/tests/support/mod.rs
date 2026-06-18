use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static CONTAINER_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct DockerPostgres {
    pub name: String,
}

impl Drop for DockerPostgres {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.name])
            .status();
    }
}

pub fn setup_postgres() -> DockerPostgres {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = CONTAINER_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let name = format!("linkdrop-model-{}-{suffix}-{sequence}", std::process::id());

    let output = Command::new("docker")
        .args([
            "run",
            "-d",
            "--rm",
            "--name",
            &name,
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

    let container = DockerPostgres { name };
    wait_for_postgres(&container.name);
    container
}

fn wait_for_postgres(container_name: &str) {
    for _ in 0..60 {
        let output = Command::new("docker")
            .args(psql_network_args(container_name, "pg_isready"))
            .output()
            .unwrap();

        if output.status.success() {
            return;
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    panic!("Postgres did not become ready");
}

pub fn psql(container_name: &str, sql: &str) -> Output {
    Command::new("docker")
        .args(psql_command_args(container_name, sql))
        .output()
        .unwrap()
}

fn psql_network_args(container_name: &str, command: &str) -> Vec<String> {
    vec![
        "run".to_string(),
        "--rm".to_string(),
        "--network".to_string(),
        container_network(container_name),
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

fn psql_command_args(container_name: &str, sql: &str) -> Vec<String> {
    let mut args = psql_network_args(container_name, "psql");
    args.extend(["-d", "postgres", "-v", "ON_ERROR_STOP=1", "-qAt", "-c"].map(str::to_string));
    args.push(sql.to_string());
    args
}

fn container_network(container_name: &str) -> String {
    format!("container:{container_name}")
}
