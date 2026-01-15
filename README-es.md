# Pipeliner

Una biblioteca de orquestación de pipelines basada en Rust, construida con Arquitectura Hexagonal, diseñada para crear sistemas de pipelines robustos, mantenibles y probables.

## Descripción General

Pipeliner proporciona un marco flexible y extensible para definir y ejecutar pipelines con etapas, pasos y plugins. Sigue principios de arquitectura limpia para garantizar la separación de responsabilidades y la máxima flexibilidad.

## Características

- **Definición de Pipelines**: Crea pipelines complejos con etapas y pasos
- **Sistema de Plugins**: Arquitectura de plugins extensible para añadir funcionalidad personalizada
- **Gestión de Artefactos**: Maneja artefactos y salidas intermedias entre etapas
- **Soporte de Concurrencia**: Ejecución eficiente con control de concurrencia adecuado
- **Gestión de Errores**: Manejo robusto de errores y mecanismos de recuperación
- **Configuración**: Sistema de configuración flexible para personalizar el comportamiento del pipeline
- **Interfaz CLI**: Interfaz de línea de comandos integrada para gestionar pipelines

## Arquitectura

Pipeliner sigue **Arquitectura Hexagonal** (también conocida como Puertos y Adaptadores), organizada en tres capas principales:

```
┌─────────────────────────────────────────────────────────────┐
│                    Capa de Aplicación                       │
│  (Casos de Uso, Servicios, Orquestación)                    │
├─────────────────────────────────────────────────────────────┤
│                      Capa de Dominio                        │
│  (Entidades, Reglas de Negocio, Interfaces)                 │
├─────────────────────────────────────────────────────────────┤
│                  Capa de Infraestructura                    │
│  (Sistemas Externos, Base de Datos, Clientes HTTP, CLI)     │
└─────────────────────────────────────────────────────────────┘
```

### Capa de Dominio

Contiene la lógica de negocio central y las entidades:

- `Pipeline`: La estructura principal del pipeline
- `Stage`: Etapas individuales en un pipeline
- `Step`: Unidades ejecutables dentro de las etapas
- `Agent`: Agentes de ejecución que ejecutan pasos

### Capa de Aplicación

Implementa casos de uso y orquesta el dominio:

- Orquestación de ejecución de pipelines
- Gestión de plugins
- Manejo de artefactos
- Recuperación de errores

### Capa de Infraestructura

Adaptadores para sistemas externos e interfaces:

- Gestión de configuración
- Interfaz CLI
- Ejecutores de plugins
- Adaptadores de almacenamiento

## Estructura de Crates

```
rustline/
├── src/
│   ├── cli/                 # Interfaz de línea de comandos
│   ├── config/              # Gestión de configuración
│   ├── executor/            # Ejecución de pasos y plugins
│   ├── pipeline/            # Lógica central del pipeline
│   └── lib.rs               # Raíz de la biblioteca
├── crates/
│   ├── pipeliner-cli/       # Aplicación CLI
│   └── pipeliner-core/      # Biblioteca principal
├── tests/                   # Pruebas de integración
└── docs/                    # Documentación
```

## Instalación

### Desde el Código Fuente

```bash
git clone https://github.com/pipeliner-org/pipeliner.git
cd pipeliner
cargo build --release
```

### Desde Crates.io

```bash
cargo install pipeliner
```

## Uso

### Definición Básica de Pipeline

```rust
use pipeliner::prelude::*;

let pipeline = Pipeline::builder()
    .name("my-pipeline")
    .stage(Stage::builder("build")
        .step(Step::builder("compile")
            .command("cargo build")
            .build())
        .step(Step::builder("test")
            .command("cargo test")
            .build())
        .build())
    .stage(Stage::builder("deploy")
        .step(Step::builder("deploy")
            .command("kubectl apply -f k8s/")
            .build())
        .build())
    .build();
```

### Ejecutando un Pipeline

```rust
use pipeliner::executor::PipelineExecutor;

let executor = PipelineExecutor::new();
executor.execute(&pipeline).await?;
```

### Usando Plugins

```rust
use pipeliner::pipeline::plugins::PluginRegistry;

let mut registry = PluginRegistry::default();
registry.register("docker", DockerPlugin::new());
registry.register("kubernetes", KubernetesPlugin::new());
```

## Configuración

Crea un archivo de configuración `pipeliner.yaml`:

```yaml
pipeline:
  name: my-pipeline
  stages:
    - name: build
      steps:
        - name: compile
          command: cargo build --release
        - name: test
          command: cargo test

execution:
  concurrency: 4
  retry:
    max_attempts: 3
    delay: 5s

artifacts:
  path: ./target/pipeliner
  retention: 7d
```

## Contribuyendo

1. Haz un fork del repositorio
2. Crea una rama de características (`git checkout -b feature/caracteristica-increible`)
3. Confirma tus cambios (`git commit -m 'feat: añadir caracteristica increíble'`)
4. Envía la rama (`git push origin feature/caracteristica-increible`)
5. Abre un Pull Request

Por favor, lee [CONTRIBUTING.md](docs/CONTRIBUTING.md) para detalles sobre nuestro código de conducta y proceso de desarrollo.

## Configuración de Desarrollo

```bash
# Instalar dependencias
cargo fetch

# Ejecutar pruebas
cargo test

# Ejecutar lints
cargo clippy

# Generar documentación
cargo doc --no-deps
```

## Licencia

Este proyecto está licenciado bajo la Licencia MIT - consulta el archivo [LICENSE](LICENSE) para más detalles.
