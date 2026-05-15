//! Pipeline PetClinic: Clone + Build + Test + Report
//!
//! Pipeline que:
//! 1. Verifica herramientas instaladas (git, java, maven)
//! 2. Clona repositorio PetClinic en workspace temporal
//! 3. Compila el proyecto
//! 4. Ejecuta tests
//! 5. Genera reporte con stats del binary
//!
//! ```rust,ignore
//! use pipeliner_macros::pipeline;
//!
//! let petclinic = pipeline! {
//!     name = "PetClinic CI Pipeline"
//!     
//!     stages {
//!         stage!("Prerequisites") {
//!             steps {
//!                 sh!("which git && git --version")
//!                 sh!("which java && java -version")
//!                 sh!("which mvn && mvn --version")
//!             }
//!         }
//!         
//!         stage!("Clone") {
//!             steps {
//!                 sh!("git clone https://github.com/spring-projects/spring-petclinic.git /tmp/petclinic")
//!             }
//!         }
//!         
//!         stage!("Build") {
//!             steps {
//!                 sh!("cd /tmp/petclinic && ./mvnw package -DskipTests")
//!             }
//!         }
//!         
//!         stage!("Test") {
//!             steps {
//!                 sh!("cd /tmp/petclinic && ./mvnw test")
//!             }
//!         }
//!         
//!         stage!("Report") {
//!             steps {
//!                 sh!("cd /tmp/petclinic && find . -name '*.jar' -type f")
//!                 sh!("cd /tmp/petclinic && du -sh target/")
//!                 echo!("Pipeline SUCCESS!")
//!             }
//!         }
//!     }
//! };
//! ```

use pipeliner_macros::pipeline;

fn main() {
    let _pipeline = pipeline! {
        name = "PetClinic CI Pipeline"
        
        stages {
            stage!("Prerequisites") {
                steps {
                    sh!("which git && git --version")
                    sh!("which java && java -version")
                    sh!("which mvn && mvn --version")
                    echo!("Prerequisites satisfied!")
                }
            }
            
            stage!("Clone") {
                steps {
                    sh!("cd /tmp && rm -rf petclinic 2>/dev/null; true")
                    sh!("git clone https://github.com/spring-projects/spring-petclinic.git /tmp/petclinic")
                    sh!("cd /tmp/petclinic && ls -la")
                    echo!("Repository cloned!")
                }
            }
            
            stage!("Build") {
                steps {
                    sh!("cd /tmp/petclinic && ./mvnw package -DskipTests")
                    echo!("Build completed!")
                }
            }
            
            stage!("Test") {
                steps {
                    sh!("cd /tmp/petclinic && ./mvnw test")
                    echo!("Tests completed!")
                }
            }
            
            stage!("Report") {
                steps {
                    sh!("cd /tmp/petclinic && find . -name '*.jar' -type f 2>/dev/null | head -5")
                    sh!("cd /tmp/petclinic && du -sh target/ 2>/dev/null || echo 'No target dir'")
                    sh!("cd /tmp/petclinic && echo '=== BUILD SUCCESS ==='")
                    sh!("cd /tmp/petclinic && echo 'Pipeline completed at: $(date)'")
                    echo!("Pipeline complete!")
                }
            }
        }
    };
    
    println!("Pipeline defined: PetClinic CI Pipeline");
}
