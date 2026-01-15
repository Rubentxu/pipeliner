#!/usr/bin/env rust-script
//!
//! # Mi Primer Pipeline con Rustline
//!
//! Este es un ejemplo de pipeline CI/CD usando el DSL de Rustline,
//! compatible con la sintaxis de Jenkins Pipeline.
//!
//! ## Uso
//!
//! ```bash
//! rust-script examples/mi_pipeline.rs
//! ```
//!
//! ## Dependencias
//!
//! ```cargo
//! [package]
//! name = "mi-pipeline"
//! version = "0.1.0"
//! edition = "2024"
//!
//! [dependencies]
//! rustline = { path = "/home/rubentxu/Proyectos/rust/rustline" }
//! ```

use rustline::LocalExecutor;
use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================");
    println!("   Mi Pipeline CI/CD con Rustline");
    println!("========================================\n");

    // Definir el pipeline usando macros estilo Jenkins
    let pipeline = pipeline! {
        agent {
            docker("rust:latest")
        }
        stages {
            stage!("Checkout", steps!(
                echo!("📦 Obteniendo código fuente..."),
                sh!("echo 'Git clone completo'"),
                sh!("pwd")
            ))
            stage!("Build", steps!(
                echo!("🔨 Compilando el proyecto..."),
                sh!("cargo --version"),
                sh!("echo 'Compilación exitosa!'")
            ))
            stage!("Test", steps!(
                echo!("🧪 Ejecutando tests..."),
                sh!("echo 'Todos los tests pasaron!'")
            ))
            stage!("Deploy", steps!(
                echo!("🚀 Desplegando a producción..."),
                sh!("echo 'Despliegue completado!'")
            ))
        }
    };

    // Mostrar información del pipeline
    println!("Pipeline definido:");
    println!("  Agente: Docker (rust:latest)");
    println!("  Etapas: {}", pipeline.stages.len());
    for (i, stage) in pipeline.stages.iter().enumerate() {
        println!(
            "    {} - {} ({} pasos)",
            i + 1,
            stage.name,
            stage.steps.len()
        );
    }
    println!();

    // Ejecutar el pipeline
    println!("Ejecutando pipeline...\n");

    let executor = LocalExecutor::new();
    match executor.execute(&pipeline) {
        Ok(result) => {
            println!("\n========================================");
            println!("   Resultado: {:?}", result);
            println!("========================================");
            println!("\nPipeline ejecutado exitosamente!");
        }
        Err(e) => {
            eprintln!("\n❌ Error al ejecutar pipeline: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
