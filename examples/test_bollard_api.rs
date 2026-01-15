use bollard::Docker;
use bollard::container::{Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions};
use bollard::image::PullImageOptions;
use bollard::models::ContainerCreateResponse;

use std::process::ExitStatus;

#[tokio::main]
async fn main() {
    println!("=== Testing bollard 0.20 API ===\n");

    // Connect to Docker
    let docker = Docker::connect_with_socket("/var/run/docker.sock").await;

    match docker {
        Ok(client) => {
            println!("✓ Connected to Docker socket");

            // Ping
            match client.ping().await {
                Ok(_) => println!("✓ Docker daemon responding"),
                Err(e) => println!("✗ Ping failed: {:?}", e),
            }

            // Test creating a container
            let config = Config {
                image: Some("alpine:latest".to_string()),
                cmd: Some(vec!["echo", "hello"]),
                ..Default::default()
            };

            let options = CreateContainerOptions {
                name: format!("bollard-test-{}", uuid::Uuid::new_v4()),
                ..Default::default()
            };

            match client.create_container(options, config).await {
                Ok(response) => {
                    println!("✓ Container created: {:?}", response.id);
                    let id = response.id.unwrap_or_default();

                    // Start
                    if client.start_container(&id, None).await.is_ok() {
                        println!("✓ Container started");

                        // Wait
                        let mut stream = client.wait_container(
                            &id,
                            Some(bollard::service::WaitContainerOptions {
                                condition: "not-running",
                            }),
                        );

                        match stream.next().await {
                            Some(Ok(status)) => {
                                let code = status.status_code().unwrap_or(0);
                                println!("✓ Container exited with code: {}", code);
                            }
                            _ => println!("✗ Wait failed"),
                        }

                        // Logs
                        let logs_opts = LogsOptions::<&str> {
                            stdout: true,
                            stderr: true,
                            ..Default::default()
                        };

                        let mut logs = client.logs(&id, Some(logs_opts));
                        while let Some(log) = logs.next().await {
                            match log {
                                Ok(bollard::container::LogOutput::StdOut { message }) => {
                                    print!("stdout: {}", String::from_utf8_lossy(&message));
                                }
                                Ok(bollard::container::LogOutput::StdErr { message }) => {
                                    print!("stderr: {}", String::from_utf8_lossy(&message));
                                }
                                _ => {}
                            }
                        }

                        // Remove
                        let _ = client
                            .remove_container(
                                &id,
                                Some(RemoveContainerOptions {
                                    force: true,
                                    ..Default::default()
                                }),
                            )
                            .await;
                        println!("✓ Container removed");
                    }
                }
                Err(e) => println!("✗ Failed to create container: {:?}", e),
            }
        }
        Err(e) => {
            println!("✗ Failed to connect: {:?}", e);
        }
    }
}
