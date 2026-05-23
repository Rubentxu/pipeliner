# CI Pipeline - Pure DSL, no compilation needed!
# Run with: pipeliner run ci.dsl

pipeline {
    name = "CI Pipeline"
    agent = any
    
    environment {
        RUST_VERSION = "1.70"
        CARGO_TERM_COLOR = "always"
    }
    
    stages {
        stage("Build") {
            steps {
                sh "cargo build --release"
            }
        }
        
        stage("Test") {
            steps {
                timeout(30) {
                    retry(3) {
                        sh "cargo test"
                    }
                }
            }
        }
        
        stage("Clippy") {
            steps {
                sh "cargo clippy -- -D warnings"
            }
        }
    }
    
    post {
        always {
            echo "Pipeline completed"
        }
        success {
            echo "Build succeeded!"
        }
        failure {
            echo "Build failed!"
        }
    }
}
