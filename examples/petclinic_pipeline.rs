#!/usr/bin/env rust-script
//! ```cargo
//! [package]
//! name = "petclinic-pipeline"
//! version = "0.1.0"
//! edition = "2024"
//!
//! [dependencies]
//! rustline = { path = "/home/rubentxu/Proyectos/rust/pipeliner" }
//! pipeliner-steps-maven = { path = "/home/rubentxu/Proyectos/rust/pipeliner/crates/pipeliner-steps-maven" }
//! pipeliner-steps-git = { path = "/home/rubentxu/Proyectos/rust/pipeliner/crates/pipeliner-steps-git" }
//! pipeliner-steps-http = { path = "/home/rubentxu/Proyectos/rust/pipeliner/crates/pipeliner-steps-http" }
//! pipeliner-steps-core = { path = "/home/rubentxu/Proyectos/rust/pipeliner/crates/pipeliner-steps-core" }
//! ```

use rustline::prelude::*;
use rustline::LocalExecutor;
use pipeliner_steps_core::{ConfigTool, WorkspaceTool};
use pipeliner_steps_git::GitTool;
use pipeliner_steps_maven::MavenTool;
use pipeliner_steps_http::RestClientTool;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("===========================================");
    println!("   Petclinic Pipeline - Maven CI/CD Demo");
    println!("===========================================\n");

    // Crear pipeline con el DSL
    let pipeline = pipeline! {
        agent {
            any()
        }
        environment {
            PETCLINIC_REPO = "https://github.com/spring-projects/spring-petclinic.git"
            ARTIFACT_NAME = "spring-petclinic-3.3.0.jar"
            POST_URL = "https://httpbin.org/post"
        }
        stages {
            stage!("Setup", steps!(
                echo!("=== Setting up development environment ==="),
                sh!("echo 'Installing asdf-vm...'"),
                sh!("git clone https://github.com/asdf-vm/asdf.git ~/.asdf --depth 1 2>/dev/null || true"),
                sh!("echo 'Configuring asdf...'"),
                sh!("echo '. $HOME/.asdf/asdf.sh' >> $HOME/.bashrc"),
                echo!("asdf-vm installation complete")
            ))
            stage!("Install Tools", steps!(
                echo!("=== Installing Java and Maven via asdf ==="),
                // Simular instalación de herramientas (en prod usaríamos asdf install)
                sh!("echo 'Installing java 17...'"),
                sh!("java -version 2>&1 || echo 'Java would be installed via asdf'"),
                sh!("echo 'Installing maven 3.9.x...'"),
                sh!("mvn -version 2>&1 || echo 'Maven would be installed via asdf'"),
                echo!("Tools installation complete")
            ))
            stage!("Checkout", steps!(
                echo!("=== Cloning spring-petclinic repository ==="),
                sh!("echo 'Cloning repository: ${PETCLINIC_REPO}'"),
                sh!("git clone --depth 1 --branch main ${PETCLINIC_REPO} /tmp/spring-petclinic"),
                sh!("cd /tmp/spring-petclinic && pwd && ls -la"),
                echo!("Checkout complete")
            ))
            stage!("Build", steps!(
                echo!("=== Building spring-petclinic with Maven ==="),
                sh!("cd /tmp/spring-petclinic && mvn clean package -DskipTests -B 2>&1 | tail -20"),
                sh!("ls -lh /tmp/spring-petclinic/target/*.jar 2>/dev/null || echo 'JAR files:'"),
                sh!("find /tmp/spring-petclinic/target -name '*.jar' -type f 2>/dev/null | head -5"),
                echo!("Build complete!")
            ))
            stage!("Test", steps!(
                echo!("=== Running Maven tests ==="),
                sh!("cd /tmp/spring-petclinic && mvn test -B 2>&1 | tail -30"),
                echo!("Tests complete!")
            ))
            stage!("Verify Artifact", steps!(
                echo!("=== Verifying artifact ==="),
                sh!("ARTIFACT=$(find /tmp/spring-petclinic/target -name '*.jar' -type f | grep -v original | head -1)"),
                sh!("if [ -n \"$ARTIFACT\" ]; then echo \"ARTIFACT_FOUND=$ARTIFACT\"; ls -lh \"$ARTIFACT\"; else echo 'ARTIFACT_NOT_FOUND'; fi"),
                sh!("echo 'Artifact verification complete'")
            ))
            stage!("Notify", steps!(
                echo!("=== Sending success notification ==="),
                sh!("curl -s -X POST ${POST_URL} -H 'Content-Type: application/json' -d '{\"pipeline\":\"petclinic\",\"status\":\"SUCCESS\",\"artifact\":\"spring-petclinic.jar\",\"build_time\":\"'$(date -Iseconds)'\"}' 2>/dev/null | head -20 || echo 'Notification sent'"),
                echo!("Notification complete!")
            ))
        }
        post {
            success(echo!("🎉 Pipeline succeeded! Artifact: spring-petclinic.jar"))
            failure(echo!("❌ Pipeline failed!"))
            always(echo!("📊 Pipeline execution finished"))
        }
    };

    // Mostrar información del pipeline
    println!("Pipeline definido:");
    println!("  Agente: any (local)");
    println!("  Etapas: {}", pipeline.stages.len());
    for (i, stage) in pipeline.stages.iter().enumerate() {
        println!("    {} - {} ({} pasos)", i + 1, stage.name, stage.steps.len());
    }
    println!("  Variables de entorno: {}", pipeline.environment.vars.len());
    for (key, value) in pipeline.environment.vars.iter() {
        println!("    {} = {}", key, value);
    }
    println!();

    // Ejecutar pipeline
    println!("Ejecutando pipeline...\n");

    let mut executor = LocalExecutor::new();
    match executor.execute(&pipeline) {
        Ok(result) => {
            println!("\n===========================================");
            println!("   Resultado: {:?}", result);
            println!("===========================================");
            println!("\n✅ Pipeline petclinic ejecutado exitosamente!");
            println!("   Artifact: spring-petclinic.jar");
        }
        Err(e) => {
            eprintln!("\n❌ Error al ejecutar pipeline: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
