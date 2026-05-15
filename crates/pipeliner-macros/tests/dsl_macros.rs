use pipeliner_core::{
    Pipeline, Stage, Step, pipeline::StageOrParallel,
};
use pipeliner_macros::pipeline_def;

#[test]
fn test_pipeline_macro_simple() {
    let pipeline = pipeline_def! {
        name = "Test Pipeline"
        stages {
            stage!("Build") {
                steps {
                    sh!("cargo build")
                    echo!("Build done")
                }
            }
        }
    };
    
    assert_eq!(pipeline.name(), Some("Test Pipeline"));
    assert_eq!(pipeline.stages.len(), 1);
    
    let stage = pipeline.stages[0].as_stage().unwrap();
    assert_eq!(stage.name, "Build");
    assert_eq!(stage.steps.len(), 2);
}

#[test]
fn test_pipeline_macro_multiple_stages() {
    let pipeline = pipeline_def! {
        name = "CI Pipeline"
        stages {
            stage!("Build") {
                steps {
                    sh!("cargo build --release")
                }
            }
            
            stage!("Test") {
                steps {
                    sh!("cargo test")
                }
            }
            
            stage!("Verify") {
                steps {
                    sh!("ls -lh target/release")
                    echo!("Done!")
                }
            }
        }
    };
    
    assert_eq!(pipeline.name(), Some("CI Pipeline"));
    assert_eq!(pipeline.stages.len(), 3);
    
    assert!(pipeline.stages[0].as_stage().is_some());
    assert_eq!(pipeline.stages[0].as_stage().unwrap().name, "Build");
    assert_eq!(pipeline.stages[1].as_stage().unwrap().name, "Test");
    assert_eq!(pipeline.stages[2].as_stage().unwrap().name, "Verify");
}

#[test]
fn test_pipeline_macro_with_parallel() {
    // Jenkins-style: parallel {} inside stage! generates a ParallelGroup
    // The outer stage with parallel {} becomes a ParallelGroup directly
    let pipeline = pipeline_def! {
        name = "Parallel Pipeline"
        stages {
            stage!("Build") {
                steps {
                    sh!("cargo build")
                }
            }
            
            stage!("Test") {
                parallel {
                    stage!("Linux") {
                        steps { sh!("cargo test --linux") }
                    }
                    stage!("Windows") {
                        steps { sh!("cargo test --windows") }
                    }
                }
            }
        }
    };
    
    assert_eq!(pipeline.name(), Some("Parallel Pipeline"));
    assert_eq!(pipeline.stages.len(), 2);
    
    // First stage is regular
    assert!(pipeline.stages[0].as_stage().is_some());
    assert_eq!(pipeline.stages[0].as_stage().unwrap().name, "Build");
    
    // Second item is a parallel group (stage with parallel content becomes ParallelGroup)
    assert!(pipeline.stages[1].is_parallel());
    let parallel = pipeline.stages[1].as_parallel().unwrap();
    assert_eq!(parallel.stages.len(), 2);
}

#[test]
fn test_pipeline_macro_petclinic_style() {
    let pipeline = pipeline_def! {
        name = "PetClinic CI"
        stages {
            stage!("Prerequisites") {
                steps {
                    sh!("which git")
                    sh!("which java")
                    sh!("which mvn")
                }
            }
            
            stage!("Clone") {
                steps {
                    sh!("git clone https://github.com/spring-projects/spring-petclinic.git /tmp/petclinic")
                }
            }
            
            stage!("Build") {
                steps {
                    sh!("cd /tmp/petclinic && ./mvnw package -DskipTests")
                }
            }
            
            stage!("Test") {
                steps {
                    sh!("cd /tmp/petclinic && ./mvnw test")
                }
            }
            
            stage!("Report") {
                steps {
                    sh!("cd /tmp/petclinic && find . -name '*.jar' -type f")
                    sh!("echo 'Pipeline SUCCESS'")
                }
            }
        }
    };
    
    assert_eq!(pipeline.name(), Some("PetClinic CI"));
    assert_eq!(pipeline.stages.len(), 5);
    
    // Verify stage names
    assert_eq!(pipeline.stages[0].as_stage().unwrap().name, "Prerequisites");
    assert_eq!(pipeline.stages[1].as_stage().unwrap().name, "Clone");
    assert_eq!(pipeline.stages[2].as_stage().unwrap().name, "Build");
    assert_eq!(pipeline.stages[3].as_stage().unwrap().name, "Test");
    assert_eq!(pipeline.stages[4].as_stage().unwrap().name, "Report");
}
