//! Declarative macros for pipeline DSL
//!
//! This module contains macros for defining pipelines in a
//! declarative syntax similar to Jenkins Pipeline.

/// Creates an "any" agent
#[macro_export]
macro_rules! agent_any {
    () => {
        $crate::pipeline::AgentType::Any
    };
}

/// Creates a "label" agent
#[macro_export]
macro_rules! agent_label {
    ($label:expr) => {
        $crate::pipeline::AgentType::label($label)
    };
}

/// Creates a "docker" agent
#[macro_export]
macro_rules! agent_docker {
    ($image:expr) => {
        $crate::pipeline::AgentType::docker($image)
    };
}

/// Creates a "kubernetes" agent
#[macro_export]
macro_rules! agent_kubernetes {
    ($image:expr) => {
        $crate::pipeline::AgentType::kubernetes($image)
    };
}

/// Creates a shell command step
#[macro_export]
macro_rules! sh {
    ($cmd:expr) => {
        $crate::pipeline::Step::shell($cmd)
    };
}

/// Creates an echo step
#[macro_export]
macro_rules! echo {
    ($msg:expr) => {
        $crate::pipeline::Step::echo($msg)
    };
}

/// Creates a timeout step
#[macro_export]
macro_rules! timeout {
    ($secs:expr, $step:expr) => {
        $crate::pipeline::Step::timeout(std::time::Duration::from_secs($secs), $step)
    };
}

/// Creates a stage
#[macro_export]
macro_rules! stage {
    ($name:expr, $steps:expr) => {
        $crate::pipeline::Stage::new($name, $steps)
    };
}

/// Creates a list of steps
#[macro_export]
macro_rules! steps {
    ($($step:expr),* $(,)?) => {
        vec![$($step),*]
    };
}

/// Creates post-conditions
#[macro_export]
macro_rules! post {
    ( $( $cond:ident ( $steps:expr ) ),* ) => {{
        vec![
            $(
                $crate::pipeline::PostCondition::$cond(vec![$steps]),
            )*
        ]
    }};
}

/// Creates a when condition
#[macro_export]
macro_rules! when {
    (branch($branch:expr)) => {
        $crate::pipeline::WhenCondition::branch($branch)
    };
}

/// Creates parallel execution
#[macro_export]
macro_rules! branch {
    ($name:expr, $stage:expr) => {
        $crate::pipeline::ParallelBranch {
            name: $name.to_string(),
            stage: $stage,
        }
    };
}

/// Creates parallel execution
#[macro_export]
macro_rules! parallel {
    ( $( $branch:expr ),* $(,)? ) => {{
        vec![$($branch),*]
    }};
}

/// Creates a matrix configuration
#[macro_export]
macro_rules! matrix {
    ( $( $axis_name:ident = [ $($value:expr),* $(,)? ] ),* $(,)? ) => {{
        let mut matrix = $crate::pipeline::MatrixConfig::new();
        $(
            matrix = matrix.add_axis(stringify!($axis_name), vec![$($value.to_string()),*]);
        )*
        matrix
    }};
}

/// Creates a pipeline using declarative block syntax
#[macro_export]
macro_rules! pipeline {
    // Simple pipeline with any agent
    (
        agent {
            any()
        }
        stages {
            $(stage!( $stage_name:expr, $stage_steps:expr $(,)? ))*
        }
    ) => {{
        $crate::pipeline::Pipeline::builder()
            .agent($crate::pipeline::AgentType::Any)
            .stages({
                let mut stages_vec = vec![];
                $(
                    stages_vec.push($crate::pipeline::Stage::new($stage_name.to_string(), $stage_steps));
                )*
                stages_vec
            })
            .build_unchecked()
    }};
    // Pipeline with label agent
    (
        agent {
            label($label:expr)
        }
        stages {
            $(stage!( $stage_name:expr, $stage_steps:expr $(,)? ))*
        }
    ) => {{
        $crate::pipeline::Pipeline::builder()
            .agent($crate::pipeline::AgentType::label($label))
            .stages({
                let mut stages_vec = vec![];
                $(
                    stages_vec.push($crate::pipeline::Stage::new($stage_name.to_string(), $stage_steps));
                )*
                stages_vec
            })
            .build_unchecked()
    }};
    // Pipeline with docker agent
    (
        agent {
            docker($image:expr)
        }
        stages {
            $(stage!( $stage_name:expr, $stage_steps:expr $(,)? ))*
        }
    ) => {{
        $crate::pipeline::Pipeline::builder()
            .agent($crate::pipeline::AgentType::docker($image))
            .stages({
                let mut stages_vec = vec![];
                $(
                    stages_vec.push($crate::pipeline::Stage::new($stage_name.to_string(), $stage_steps));
                )*
                stages_vec
            })
            .build_unchecked()
    }};
    // Pipeline with kubernetes agent
    (
        agent {
            kubernetes { image($image:expr) }
        }
        stages {
            $(stage!( $stage_name:expr, $stage_steps:expr $(,)? ))*
        }
    ) => {{
        $crate::pipeline::Pipeline::builder()
            .agent($crate::pipeline::AgentType::kubernetes($image))
            .stages({
                let mut stages_vec = vec![];
                $(
                    stages_vec.push($crate::pipeline::Stage::new($stage_name.to_string(), $stage_steps));
                )*
                stages_vec
            })
            .build_unchecked()
    }};
    // Pipeline with kubernetes agent and label
    (
        agent {
            kubernetes { image($image:expr) label($label:expr) }
        }
        stages {
            $(stage!( $stage_name:expr, $stage_steps:expr $(,)? ))*
        }
    ) => {{
        let config = $crate::pipeline::KubernetesConfig {
            image: $image.to_string(),
            label: Some($label.to_string()),
            ..Default::default()
        };
        $crate::pipeline::Pipeline::builder()
            .agent($crate::pipeline::AgentType::Kubernetes(config))
            .stages({
                let mut stages_vec = vec![];
                $(
                    stages_vec.push($crate::pipeline::Stage::new($stage_name.to_string(), $stage_steps));
                )*
                stages_vec
            })
            .build_unchecked()
    }};
    // Pipeline with environment
    (
        agent {
            any()
        }
        environment {
            $($env_key:ident = $env_value:expr),* $(,)?
        }
        stages {
            $(stage!( $stage_name:expr, $stage_steps:expr $(,)? ))*
        }
    ) => {{
        let mut env = $crate::pipeline::Environment::new();
        $(
            env = env.set(stringify!($env_key), $env_value.to_string());
        )*
        $crate::pipeline::Pipeline::builder()
            .agent($crate::pipeline::AgentType::Any)
            .with_environment(env)
            .stages({
                let mut stages_vec = vec![];
                $(
                    stages_vec.push($crate::pipeline::Stage::new($stage_name.to_string(), $stage_steps));
                )*
                stages_vec
            })
            .build_unchecked()
    }};
    // Pipeline with parameters
    (
        agent {
            any()
        }
        parameters {
            boolean($bool_name:expr, $bool_value:expr)
            string($str_name:expr, $str_value:expr)
            choice($choice_name:expr, $choice_values:expr)
        }
        stages {
            $(stage!( $stage_name:expr, $stage_steps:expr $(,)? ))*
        }
    ) => {{
        let mut params = $crate::pipeline::Parameters::new();
        params = params.boolean($bool_name, $bool_value);
        params = params.string($str_name, $str_value.to_string());
        params = params.choice($choice_name, $choice_values);
        $crate::pipeline::Pipeline::builder()
            .agent($crate::pipeline::AgentType::Any)
            .with_parameters(params)
            .stages({
                let mut stages_vec = vec![];
                $(
                    stages_vec.push($crate::pipeline::Stage::new($stage_name.to_string(), $stage_steps));
                )*
                stages_vec
            })
            .build_unchecked()
    }};
    // Pipeline with options
    (
        agent {
            any()
        }
        options {
            timeout(minutes: $minutes:expr)
            retry(count: $retry_count:expr)
        }
        stages {
            $(stage!( $stage_name:expr, $stage_steps:expr $(,)? ))*
        }
    ) => {{
        let mut options = $crate::pipeline::PipelineOptions::new();
        options = options.with_timeout(std::time::Duration::from_secs($minutes * 60));
        options = options.with_retry($retry_count);
        $crate::pipeline::Pipeline::builder()
            .agent($crate::pipeline::AgentType::Any)
            .options(options)
            .stages({
                let mut stages_vec = vec![];
                $(
                    stages_vec.push($crate::pipeline::Stage::new($stage_name.to_string(), $stage_steps));
                )*
                stages_vec
            })
            .build_unchecked()
    }};
    // Pipeline with post conditions
    (
        agent {
            any()
        }
        stages {
            $(stage!( $stage_name:expr, $stage_steps:expr $(,)? ))*
        }
        post {
            always($($always_step:expr),*)
            success($($success_step:expr),*)
            failure($($failure_step:expr),*)
        }
    ) => {{
        let post_vec = vec![
            $crate::pipeline::PostCondition::always(vec![$($always_step),*]),
            $crate::pipeline::PostCondition::success(vec![$($success_step),*]),
            $crate::pipeline::PostCondition::failure(vec![$($failure_step),*]),
        ];
        $crate::pipeline::Pipeline::builder()
            .agent($crate::pipeline::AgentType::Any)
            .stages({
                let mut stages_vec = vec![];
                $(
                    stages_vec.push($crate::pipeline::Stage::new($stage_name.to_string(), $stage_steps));
                )*
                stages_vec
            })
            .posts(post_vec)
            .build_unchecked()
    }};
    // Full pipeline
    (
        agent {
            docker($image:expr)
        }
        environment {
            $($env_key:ident = $env_value:expr),* $(,)?
        }
        parameters {
            boolean($bool_name:expr, $bool_value:expr)
            string($str_name:expr, $str_value:expr)
            choice($choice_name:expr, $choice_values:expr)
        }
        options {
            timeout(minutes: $minutes:expr)
            retry(count: $retry_count:expr)
        }
        stages {
            $(stage!( $stage_name:expr, $stage_steps:expr $(,)? ))*
        }
        post {
            always($($always_step:expr),*)
            success($($success_step:expr),*)
            failure($($failure_step:expr),*)
        }
    ) => {{
        let mut env = $crate::pipeline::Environment::new();
        $(
            env = env.set(stringify!($env_key), $env_value.to_string());
        )*
        let mut params = $crate::pipeline::Parameters::new();
        params = params.boolean($bool_name, $bool_value);
        params = params.string($str_name, $str_value.to_string());
        params = params.choice($choice_name, $choice_values);
        let mut options = $crate::pipeline::PipelineOptions::new();
        options = options.with_timeout(std::time::Duration::from_secs($minutes * 60));
        options = options.with_retry($retry_count);
        let post_vec = vec![
            $crate::pipeline::PostCondition::always(vec![$($always_step),*]),
            $crate::pipeline::PostCondition::success(vec![$($success_step),*]),
            $crate::pipeline::PostCondition::failure(vec![$($failure_step),*]),
        ];
        $crate::pipeline::Pipeline::builder()
            .agent($crate::pipeline::AgentType::docker($image))
            .with_environment(env)
            .with_parameters(params)
            .options(options)
            .stages({
                let mut stages_vec = vec![];
                $(
                    stages_vec.push($crate::pipeline::Stage::new($stage_name.to_string(), $stage_steps));
                )*
                stages_vec
            })
            .posts(post_vec)
            .build_unchecked()
    }};
}

#[cfg(test)]
mod tests {
    use crate::pipeline::{AgentType, StepType, WhenCondition};

    #[test]
    fn test_agent_any_macro() {
        let agent = agent_any!();
        assert!(matches!(agent, AgentType::Any));
    }

    #[test]
    fn test_sh_macro() {
        let step = sh!("echo test");
        assert!(matches!(step.step_type, StepType::Shell { .. }));
    }

    #[test]
    fn test_echo_macro() {
        let step = echo!("test");
        assert!(matches!(step.step_type, StepType::Echo { .. }));
    }

    #[test]
    fn test_timeout_macro() {
        let inner = sh!("echo test");
        let timeout = timeout!(10, inner);
        assert!(matches!(timeout.step_type, StepType::Timeout { .. }));
    }

    #[test]
    fn test_stage_macro() {
        let steps = steps!(sh!("echo test"));
        let stage = stage!("Test", steps);
        assert_eq!(stage.name, "Test");
        assert_eq!(stage.steps.len(), 1);
    }

    #[test]
    fn test_steps_macro() {
        let steps = steps!(sh!("echo 1"), sh!("echo 2"));
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn test_post_macro() {
        let post = post!(always(sh!("cleanup")), success(sh!("notify")));
        assert_eq!(post.len(), 2);
    }

    #[test]
    fn test_when_macro() {
        let when = when!(branch("main"));
        assert!(matches!(when, WhenCondition::Branch { .. }));
    }

    #[test]
    fn test_steps_macro_with_multiple() {
        let steps = steps!(sh!("echo 1"), sh!("echo 2"), sh!("echo 3"));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn test_steps_macro_trailing_comma() {
        let steps = steps!(sh!("echo 1"), sh!("echo 2"),);
        assert_eq!(steps.len(), 2);
    }

    // Sprint 2 Tests - Declarative DSL Macros

    #[test]
    fn test_pipeline_macro_simple() {
        let pipeline = pipeline! {
            agent {
                any()
            }
            stages {
                stage!("Build", steps!(
                    sh!("cargo build")
                ))
            }
        };
        assert!(matches!(pipeline.agent, AgentType::Any));
        assert_eq!(pipeline.stages.len(), 1);
        assert_eq!(pipeline.stages[0].name, "Build");
    }

    #[test]
    fn test_pipeline_macro_with_label_agent() {
        let pipeline = pipeline! {
            agent {
                label("linux")
            }
            stages {
                stage!("Test", steps!(
                    sh!("cargo test")
                ))
            }
        };
        assert!(matches!(pipeline.agent, AgentType::Label(label) if label == "linux"));
    }

    #[test]
    fn test_pipeline_macro_with_docker_agent() {
        let pipeline = pipeline! {
            agent {
                docker("rust:1.70")
            }
            stages {
                stage!("Build", steps!(
                    sh!("cargo build")
                ))
            }
        };
        assert!(matches!(pipeline.agent, AgentType::Docker(docker) if docker.image == "rust:1.70"));
    }

    #[test]
    fn test_pipeline_macro_with_kubernetes_agent() {
        let pipeline = pipeline! {
            agent {
                kubernetes { image("rust:1.70") }
            }
            stages {
                stage!("Build", steps!(
                    sh!("cargo build")
                ))
            }
        };
        assert!(matches!(pipeline.agent, AgentType::Kubernetes(k8s) if k8s.image == "rust:1.70"));
    }

    #[test]
    fn test_pipeline_macro_with_kubernetes_agent_and_label() {
        let pipeline = pipeline! {
            agent {
                kubernetes { image("rust:1.70") label("rust-builder") }
            }
            stages {
                stage!("Build", steps!(
                    sh!("cargo build")
                ))
            }
        };
        assert!(matches!(pipeline.agent, AgentType::Kubernetes(k8s) 
            if k8s.image == "rust:1.70" && k8s.label == Some("rust-builder".to_string())));
    }

    #[test]
    fn test_pipeline_macro_with_environment() {
        let pipeline = pipeline! {
            agent {
                any()
            }
            environment {
                FOO = "bar",
                BAZ = "qux"
            }
            stages {
                stage!("Build", steps!(
                    sh!("echo $FOO")
                ))
            }
        };
        assert_eq!(
            pipeline.environment.vars.get("FOO"),
            Some(&"bar".to_string())
        );
        assert_eq!(
            pipeline.environment.vars.get("BAZ"),
            Some(&"qux".to_string())
        );
    }

    #[test]
    fn test_pipeline_macro_with_parameters() {
        let pipeline = pipeline! {
            agent {
                any()
            }
            parameters {
                boolean("DEBUG", false)
                string("VERSION", "1.0.0")
                choice("ENVIRONMENT", vec!["dev".to_string(), "staging".to_string(), "prod".to_string()])
            }
            stages {
                stage!("Build", steps!(
                    sh!("cargo build")
                ))
            }
        };
        assert_eq!(pipeline.parameters.boolean.get("DEBUG"), Some(&false));
        assert_eq!(
            pipeline.parameters.string.get("VERSION"),
            Some(&"1.0.0".to_string())
        );
        assert_eq!(
            pipeline.parameters.choice.get("ENVIRONMENT"),
            Some(&vec![
                "dev".to_string(),
                "staging".to_string(),
                "prod".to_string()
            ])
        );
    }

    #[test]
    fn test_pipeline_macro_with_options() {
        let pipeline = pipeline! {
            agent {
                any()
            }
            options {
                timeout(minutes: 30)
                retry(count: 3)
            }
            stages {
                stage!("Build", steps!(
                    sh!("cargo build")
                ))
            }
        };
        assert_eq!(
            pipeline.options.timeout,
            Some(std::time::Duration::from_secs(1800))
        );
        assert_eq!(pipeline.options.retry, Some(3));
    }

    #[test]
    fn test_pipeline_macro_with_post() {
        let pipeline = pipeline! {
            agent {
                any()
            }
            stages {
                stage!("Build", steps!(
                    sh!("cargo build")
                ))
            }
            post {
                always(sh!("cleanup"))
                success(sh!("notify"))
                failure(sh!("alert"))
            }
        };
        assert_eq!(pipeline.post.len(), 3);
    }

    #[test]
    fn test_pipeline_macro_full() {
        let pipeline = pipeline! {
            agent {
                docker("rust:1.70")
            }
            environment {
                RUST_VERSION = "1.70.0"
            }
            parameters {
                boolean("DEBUG", false)
                string("VERSION", "1.0.0")
                choice("ENVIRONMENT", vec!["dev".to_string(), "staging".to_string(), "prod".to_string()])
            }
            options {
                timeout(minutes: 30)
                retry(count: 3)
            }
            stages {
                stage!("Build", steps!(
                    sh!("cargo build --release")
                ))
                stage!("Test", steps!(
                    sh!("cargo test"),
                    sh!("cargo clippy")
                ))
                stage!("Deploy", steps!(
                    sh!("cargo deploy")
                ))
            }
            post {
                always(sh!("cleanup"))
                success(sh!("notify"))
                failure(sh!("alert"))
            }
        };
        assert!(matches!(pipeline.agent, AgentType::Docker(docker) if docker.image == "rust:1.70"));
        assert_eq!(
            pipeline.environment.vars.get("RUST_VERSION"),
            Some(&"1.70.0".to_string())
        );
        assert_eq!(pipeline.parameters.boolean.get("DEBUG"), Some(&false));
        assert_eq!(
            pipeline.options.timeout,
            Some(std::time::Duration::from_secs(1800))
        );
        assert_eq!(pipeline.options.retry, Some(3));
        assert_eq!(pipeline.stages.len(), 3);
        assert_eq!(pipeline.stages[0].name, "Build");
        assert_eq!(pipeline.stages[1].name, "Test");
        assert_eq!(pipeline.stages[2].name, "Deploy");
        assert_eq!(pipeline.post.len(), 3);
    }
}

// Épica 2 Tests - Parallel y Matrix

#[cfg(test)]
mod parallel_tests {
    use crate::pipeline::ParallelBranch;

    #[test]
    fn test_parallel_macro_empty() {
        let parallel: Vec<ParallelBranch> = parallel!();
        assert!(parallel.is_empty());
    }

    #[test]
    fn test_parallel_macro_single_branch() {
        let parallel = parallel!(branch!(
            "Build",
            stage!("Build", steps!(sh!("cargo build")))
        ));
        assert_eq!(parallel.len(), 1);
    }

    #[test]
    fn test_parallel_macro_multiple_branches() {
        let parallel = parallel!(
            branch!(
                "Linux",
                stage!(
                    "Linux",
                    steps!(sh!("cargo build --target x86_64-unknown-linux-gnu"))
                )
            ),
            branch!(
                "MacOS",
                stage!(
                    "MacOS",
                    steps!(sh!("cargo build --target x86_64-apple-darwin"))
                )
            ),
            branch!(
                "Windows",
                stage!(
                    "Windows",
                    steps!(sh!("cargo build --target x86_64-pc-windows-msvc"))
                )
            )
        );
        assert_eq!(parallel.len(), 3);
    }

    #[test]
    fn test_parallel_macro_trailing_comma() {
        let parallel = parallel!(
            branch!("A", stage!("A", steps!(sh!("echo A")))),
            branch!("B", stage!("B", steps!(sh!("echo B")))),
        );
        assert_eq!(parallel.len(), 2);
    }

    #[test]
    fn test_branch_macro() {
        let branch = branch!("Build", stage!("Build", steps!(sh!("cargo build"))));
        assert_eq!(branch.name, "Build");
        assert_eq!(branch.stage.name, "Build");
    }
}

#[cfg(test)]
mod matrix_tests {
    use crate::pipeline::MatrixConfig;

    #[test]
    fn test_matrix_config_empty() {
        let matrix = MatrixConfig::new();
        assert!(matrix.axes.is_empty());
    }

    #[test]
    fn test_matrix_config_single_axis() {
        let mut matrix = MatrixConfig::new();
        matrix = matrix.add_axis(
            "os",
            vec![
                "linux".to_string(),
                "macos".to_string(),
                "windows".to_string(),
            ],
        );
        assert_eq!(matrix.axes.len(), 1);
        assert_eq!(matrix.axes[0].name, "os");
        assert_eq!(matrix.axes[0].values.len(), 3);
    }

    #[test]
    fn test_matrix_config_multiple_axes() {
        let mut matrix = MatrixConfig::new();
        matrix = matrix.add_axis("rust", vec!["stable".to_string(), "nightly".to_string()]);
        matrix = matrix.add_axis("os", vec!["linux".to_string(), "macos".to_string()]);
        assert_eq!(matrix.axes.len(), 2);
        // 2 x 2 = 4 combinations
        let combinations = matrix.generate_combinations();
        assert_eq!(combinations.len(), 4);
    }

    #[test]
    fn test_matrix_config_with_exclude() {
        let mut matrix = MatrixConfig::new();
        matrix = matrix.add_axis("rust", vec!["stable".to_string(), "nightly".to_string()]);
        matrix = matrix.add_axis("os", vec!["linux".to_string(), "macos".to_string()]);
        matrix = matrix.add_exclude(vec![
            ("rust".to_string(), "nightly".to_string()),
            ("os".to_string(), "macos".to_string()),
        ]);

        let combinations = matrix.generate_combinations();
        // 4 total - 1 excluded = 3 combinations
        assert_eq!(combinations.len(), 3);
    }

    #[test]
    fn test_matrix_generate_combinations_format() {
        let mut matrix = MatrixConfig::new();
        matrix = matrix.add_axis("version", vec!["1.0".to_string(), "1.1".to_string()]);

        let combinations = matrix.generate_combinations();
        assert_eq!(combinations.len(), 2);

        // Each combination should have the axis name and value
        for combo in &combinations {
            assert!(
                combo.contains(&("version".to_string(), "1.0".to_string()))
                    || combo.contains(&("version".to_string(), "1.1".to_string()))
            );
        }
    }

    #[test]
    fn test_matrix_macro() {
        let matrix = matrix!(os = ["linux", "macos", "windows"]);
        assert_eq!(matrix.axes.len(), 1);
        assert_eq!(matrix.axes[0].name, "os");
        assert_eq!(matrix.axes[0].values.len(), 3);
    }
}
