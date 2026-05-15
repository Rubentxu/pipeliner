# Migración a StageOrParallel

## Resumen

`Pipeline.stages` ahora es `Vec<StageOrParallel>` en vez de `Vec<Stage>`.

## Cambios Requeridos

### 1. Acceso a campos de Stage

Antes:
```rust
for stage in &pipeline.stages {
    println!("{}", stage.name);
    for step in &stage.steps { ... }
}
```

Ahora:
```rust
for item in &pipeline.stages {
    match item {
        StageOrParallel::Stage(stage) => {
            println!("{}", stage.name);
            for step in &stage.steps { ... }
        }
        StageOrParallel::Parallel(group) => {
            println!("Parallel: {}", group.name.as_deref().unwrap_or("unnamed"));
            for stage in &group.stages {
                // ...
            }
        }
    }
}
```

### 2. Alternativa: Helper methods

Usa los métodos de ayuda:

```rust
// Iterar sobre todos los steps (incluyendo parallel)
for item in &pipeline.stages {
    for step in item.all_steps() {
        // ...
    }
}

// Obtener nombre
let name = item.name(); // Option<&str>

// Saber si es parallel
if item.is_parallel() { ... }
```

### 3. Methods disponibles en StageOrParallel

- `name()` - Option<&str>
- `is_stage()` / `is_parallel()`
- `as_stage()` - Option<&Stage>
- `as_parallel()` - Option<&ParallelGroup>
- `all_steps()` - Vec<&Step> (recursivo)
- `step_count()` - usize
- `for_each_stage(f)` - closure para cada stage

### 4. Pipeline.with_stage()

Ahora acepta `impl Into<StageOrParallel>`:

```rust
Pipeline::new()
    .with_stage(stage)              // Stage -> StageOrParallel::Stage
    .with_stage(parallel_group)     // ParallelGroup -> StageOrParallel::Parallel
    .with_parallel(parallel_group) // Alternativa
```

### 5. Parallel Group

```rust
Pipeline::parallel(vec![
    Stage::new("Linux").with_step(Step::shell("make test")),
    Stage::new("Windows").with_step(Step::shell("make test")),
])
```

## Archivos a actualizar

- `pipeliner-executor/src/local.rs`
- `pipeliner-executor/src/report.rs`
- `pipeliner-executor/src/formatters.rs`
- Cualquier otro que acceda a `pipeline.stages[i].name` o `pipeline.stages[i].steps`
