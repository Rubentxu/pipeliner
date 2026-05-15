# Jenkins DSL Syntax Reference

## Jenkins Declarative Pipeline

```groovy
pipeline {
    agent any
    stages {
        stage('Build') {
            steps {
                sh 'make build'
            }
        }
        stage('Test') {
            parallel {
                stage('Linux') {
                    steps {
                        sh 'make test-linux'
                    }
                }
                stage('Windows') {
                    steps {
                        sh 'make test-windows'
                    }
                }
            }
        }
    }
}
```

## Pipeliner DSL Actual (INCORRECTO)
```rust
pipeline! {
    stages {
        stage!("Build") {
            steps {
                sh!("make build")
            }
        }
        parallel! {          // ❌ WRONG: paralelo va DENTRO de stage
            stage!("Linux") { steps { sh!("linux") } }
            stage!("Windows") { steps { sh!("win") } }
        }
    }
}
```

## Pipeliner DSL Correcto (como Jenkins)
```rust
pipeline! {
    stages {
        stage!("Build") {
            steps {
                sh!("make build")
            }
        }
        stage!("Test") {
            parallel {
                stage!("Linux") {
                    steps { sh!("test-linux") }
                }
                stage!("Windows") {
                    steps { sh!("test-win") }
                }
            }
        }
    }
}
```

## Regla
- `parallel { }` **reemplaza** `steps { }` dentro de un stage
- No es item de `stages { }`, es **contenido** de `stage!`
